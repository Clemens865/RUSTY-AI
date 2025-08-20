use anyhow::{Result, Context};
use async_openai::{
    config::OpenAIConfig,
    types::CreateEmbeddingRequestArgs,
    Client,
};
use axum::{
    extract::{Multipart, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use qdrant_client::{
    prelude::*,
    qdrant::{
        CreateCollection, Distance, PointStruct, SearchPoints, VectorParams, VectorsConfig,
        Filter, FieldCondition, Match, Condition, ScoredPoint,
        with_payload_selector::SelectorOptions, WithPayloadSelector,
        DataType,
    },
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tiktoken_rs::p50k_base;
use tracing::{debug, error, info};
use uuid::Uuid;

const COLLECTION_NAME: &str = "personal_knowledge";
const EMBEDDING_MODEL: &str = "text-embedding-3-small";
const EMBEDDING_DIMENSION: u64 = 1536;
const MAX_TOKENS_PER_CHUNK: usize = 500;
const CHUNK_OVERLAP: usize = 50;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub title: String,
    pub content: String,
    pub chunk_index: usize,
    pub total_chunks: usize,
    pub source: String,
    pub tags: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentUploadResponse {
    pub document_id: String,
    pub title: String,
    pub chunks_created: usize,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub query: String,
    pub limit: Option<usize>,
    pub threshold: Option<f32>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub documents: Vec<DocumentMatch>,
    pub query: String,
    pub total_results: usize,
}

#[derive(Debug, Serialize)]
pub struct DocumentMatch {
    pub id: String,
    pub title: String,
    pub content: String,
    pub score: f32,
    pub chunk_index: usize,
    pub source: String,
}

pub struct KnowledgeService {
    qdrant_client: QdrantClient,
    openai_client: Client<OpenAIConfig>,
    collection_name: String,
}

impl KnowledgeService {
    pub async fn new(openai_api_key: Option<String>) -> Result<Self> {
        // Initialize Qdrant client
        let qdrant_client = QdrantClient::from_url("http://localhost:6333")
            .build()
            .context("Failed to create Qdrant client")?;
        
        // Initialize OpenAI client for embeddings
        let config = if let Some(key) = openai_api_key {
            OpenAIConfig::new().with_api_key(key)
        } else {
            OpenAIConfig::new() // Uses OPENAI_API_KEY env var
        };
        let openai_client = Client::with_config(config);
        
        let service = Self {
            qdrant_client,
            openai_client,
            collection_name: COLLECTION_NAME.to_string(),
        };
        
        // Ensure collection exists
        service.ensure_collection().await?;
        
        Ok(service)
    }
    
    async fn ensure_collection(&self) -> Result<()> {
        // Check if collection exists
        let collections = self.qdrant_client.list_collections().await?;
        let exists = collections.collections.iter()
            .any(|c| c.name == self.collection_name);
        
        if !exists {
            info!("Creating Qdrant collection: {}", self.collection_name);
            
            // Create collection with proper vector configuration
            self.qdrant_client
                .create_collection(&CreateCollection {
                    collection_name: self.collection_name.clone(),
                    vectors_config: Some(VectorsConfig {
                        config: Some(qdrant_client::qdrant::vectors_config::Config::Params(
                            VectorParams {
                                size: EMBEDDING_DIMENSION,
                                distance: Distance::Cosine.into(),
                                hnsw_config: None,
                                quantization_config: None,
                                on_disk: Some(false),
                            },
                        )),
                    }),
                    ..Default::default()
                })
                .await?;
            
            info!("Collection created successfully");
        }
        
        Ok(())
    }
    
    // Generate embeddings using OpenAI
    pub async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>> {
        let request = CreateEmbeddingRequestArgs::default()
            .model(EMBEDDING_MODEL)
            .input([text])
            .build()?;
        
        let response = self.openai_client.embeddings().create(request).await?;
        
        let embedding = response
            .data
            .first()
            .ok_or_else(|| anyhow::anyhow!("No embedding returned"))?
            .embedding
            .clone();
        
        Ok(embedding)
    }
    
    // Split document into chunks
    fn chunk_text(&self, text: &str, max_tokens: usize) -> Vec<String> {
        let bpe = p50k_base().unwrap();
        let tokens = bpe.encode_with_special_tokens(text);
        
        let mut chunks = Vec::new();
        let mut start = 0;
        
        while start < tokens.len() {
            let end = std::cmp::min(start + max_tokens, tokens.len());
            let chunk_tokens = &tokens[start..end];
            
            if let Ok(chunk_text) = bpe.decode(chunk_tokens.to_vec()) {
                chunks.push(chunk_text);
            }
            
            // Move start with overlap
            start = if end >= tokens.len() {
                tokens.len()
            } else {
                end - CHUNK_OVERLAP
            };
        }
        
        chunks
    }
    
    // Store document in knowledge base
    pub async fn store_document(
        &self,
        title: String,
        content: String,
        source: String,
        tags: Vec<String>,
    ) -> Result<DocumentUploadResponse> {
        let document_id = Uuid::new_v4().to_string();
        let chunks = self.chunk_text(&content, MAX_TOKENS_PER_CHUNK);
        let total_chunks = chunks.len();
        
        info!("Storing document '{}' with {} chunks", title, total_chunks);
        
        let mut points = Vec::new();
        
        for (index, chunk) in chunks.iter().enumerate() {
            // Generate embedding for chunk
            let embedding = self.generate_embedding(chunk).await?;
            
            // Create document metadata
            let document = Document {
                id: document_id.clone(),
                title: title.clone(),
                content: chunk.clone(),
                chunk_index: index,
                total_chunks,
                source: source.clone(),
                tags: tags.clone(),
                created_at: chrono::Utc::now(),
            };
            
            // Create point for Qdrant
            let point_id = format!("{}_{}", document_id, index);
            let payload = serde_json::to_value(&document)?;
            
            points.push(PointStruct::new(
                point_id,
                embedding,
                payload,
            ));
        }
        
        // Upload points to Qdrant
        self.qdrant_client
            .upsert_points_blocking(&self.collection_name, None, points, None)
            .await?;
        
        info!("Successfully stored document '{}'", title);
        
        Ok(DocumentUploadResponse {
            document_id,
            title,
            chunks_created: total_chunks,
            message: format!("Document stored successfully with {} chunks", total_chunks),
        })
    }
    
    // Search documents using semantic similarity
    pub async fn search_documents(
        &self,
        query: &str,
        limit: usize,
        score_threshold: f32,
        tags_filter: Option<Vec<String>>,
    ) -> Result<Vec<DocumentMatch>> {
        debug!("Searching for: {}", query);
        
        // Generate embedding for query
        let query_embedding = self.generate_embedding(query).await?;
        
        // Build filter if tags are provided
        let filter = tags_filter.map(|tags| {
            Filter::must(
                tags.into_iter()
                    .map(|tag| {
                        Condition::field(
                            "tags",
                            FieldCondition::match_any(vec![tag.into()]),
                        )
                    })
                    .collect::<Vec<_>>(),
            )
        });
        
        // Search in Qdrant
        let search_result = self.qdrant_client
            .search_points(&SearchPoints {
                collection_name: self.collection_name.clone(),
                vector: query_embedding,
                limit: limit as u64,
                score_threshold: Some(score_threshold),
                with_payload: Some(WithPayloadSelector {
                    selector_options: Some(SelectorOptions::Enable(true)),
                }),
                filter,
                ..Default::default()
            })
            .await?;
        
        // Convert results to DocumentMatch
        let documents: Vec<DocumentMatch> = search_result
            .result
            .into_iter()
            .filter_map(|point| {
                if let Some(payload) = point.payload {
                    if let Ok(doc) = serde_json::from_value::<Document>(serde_json::Value::Object(payload)) {
                        Some(DocumentMatch {
                            id: doc.id,
                            title: doc.title,
                            content: doc.content,
                            score: point.score,
                            chunk_index: doc.chunk_index,
                            source: doc.source,
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();
        
        info!("Found {} relevant documents", documents.len());
        
        Ok(documents)
    }
    
    // Delete document by ID
    pub async fn delete_document(&self, document_id: &str) -> Result<()> {
        // Delete all chunks of the document
        let filter = Filter::must(vec![
            Condition::field(
                "id",
                FieldCondition::match_value(document_id.into()),
            ),
        ]);
        
        self.qdrant_client
            .delete_points(&self.collection_name, None, &filter.into(), None)
            .await?;
        
        info!("Deleted document: {}", document_id);
        
        Ok(())
    }
    
    // Get collection statistics
    pub async fn get_stats(&self) -> Result<serde_json::Value> {
        let collection_info = self.qdrant_client
            .collection_info(&self.collection_name)
            .await?;
        
        Ok(serde_json::json!({
            "collection": self.collection_name,
            "vectors_count": collection_info.result.map(|r| r.vectors_count).unwrap_or(0),
            "indexed_vectors_count": collection_info.result.map(|r| r.indexed_vectors_count).unwrap_or(0),
            "status": collection_info.status,
        }))
    }
}

// HTTP Handlers
pub async fn upload_document_handler(
    State(state): State<Arc<crate::AppState>>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut title = String::new();
    let mut content = String::new();
    let mut source = String::new();
    let mut tags = Vec::new();
    
    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap_or("").to_string();
        let data = field.bytes().await.unwrap_or_default();
        let value = String::from_utf8_lossy(&data).to_string();
        
        match name.as_str() {
            "title" => title = value,
            "content" => content = value,
            "source" => source = value,
            "tags" => tags = value.split(',').map(|s| s.trim().to_string()).collect(),
            "file" => {
                // Handle file upload
                if let Some(filename) = field.file_name() {
                    source = filename.to_string();
                    // For now, treat file content as text
                    // In production, you'd extract text from PDFs, etc.
                    content = value;
                }
            }
            _ => {}
        }
    }
    
    if title.is_empty() || content.is_empty() {
        return (StatusCode::BAD_REQUEST, "Title and content are required").into_response();
    }
    
    match state.knowledge_service.store_document(title, content, source, tags).await {
        Ok(response) => Json(response).into_response(),
        Err(e) => {
            error!("Failed to store document: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to store document").into_response()
        }
    }
}

pub async fn search_documents_handler(
    State(state): State<Arc<crate::AppState>>,
    Query(params): Query<SearchQuery>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(10);
    let threshold = params.threshold.unwrap_or(0.3);
    
    match state.knowledge_service.search_documents(
        &params.query,
        limit,
        threshold,
        params.tags,
    ).await {
        Ok(documents) => {
            Json(SearchResult {
                total_results: documents.len(),
                documents,
                query: params.query,
            }).into_response()
        }
        Err(e) => {
            error!("Search failed: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Search failed").into_response()
        }
    }
}

pub async fn knowledge_stats_handler(
    State(state): State<Arc<crate::AppState>>,
) -> impl IntoResponse {
    match state.knowledge_service.get_stats().await {
        Ok(stats) => Json(stats).into_response(),
        Err(e) => {
            error!("Failed to get stats: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get statistics").into_response()
        }
    }
}