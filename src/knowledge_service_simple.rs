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
    Qdrant, Payload,
    qdrant::{
        CreateCollectionBuilder, Distance, PointStruct, SearchPointsBuilder,
        UpsertPointsBuilder, VectorParamsBuilder,
    },
};
use serde::{Deserialize, Serialize};
use serde_json;
use std::sync::Arc;
use tracing::{debug, error, info};
use uuid::Uuid;

const COLLECTION_NAME: &str = "personal_knowledge";
const EMBEDDING_MODEL: &str = "text-embedding-3-small";
const EMBEDDING_DIMENSION: u64 = 1536;
const MAX_CHUNK_SIZE: usize = 2000; // Characters per chunk

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
    qdrant_client: Qdrant,
    openai_client: Client<OpenAIConfig>,
    collection_name: String,
}

impl KnowledgeService {
    pub async fn new(openai_api_key: Option<String>) -> Result<Self> {
        // Initialize Qdrant client using gRPC port (6334)
        let qdrant_client = Qdrant::from_url("http://localhost:6334")
            .timeout(std::time::Duration::from_secs(10))
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
            let create_collection = CreateCollectionBuilder::new(&self.collection_name)
                .vectors_config(VectorParamsBuilder::new(EMBEDDING_DIMENSION, Distance::Cosine));
            
            self.qdrant_client
                .create_collection(create_collection)
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
    
    // Simple text chunking
    fn chunk_text(&self, text: &str, max_size: usize) -> Vec<String> {
        let mut chunks = Vec::new();
        let mut current_chunk = String::new();
        
        for sentence in text.split(". ") {
            if current_chunk.len() + sentence.len() > max_size && !current_chunk.is_empty() {
                chunks.push(current_chunk.clone());
                current_chunk = String::new();
            }
            current_chunk.push_str(sentence);
            current_chunk.push_str(". ");
        }
        
        if !current_chunk.is_empty() {
            chunks.push(current_chunk);
        }
        
        if chunks.is_empty() {
            chunks.push(text.to_string());
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
        let chunks = self.chunk_text(&content, MAX_CHUNK_SIZE);
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
            
            // Create payload for Qdrant
            let payload: Payload = serde_json::json!({
                "id": document.id,
                "title": document.title,
                "content": document.content,
                "chunk_index": document.chunk_index,
                "total_chunks": document.total_chunks,
                "source": document.source,
                "created_at": document.created_at.to_rfc3339(),
                "tags": document.tags,
            }).try_into()?;
            
            // Create point for Qdrant - use a unique UUID for each chunk
            let point_id = Uuid::new_v4().to_string();
            
            points.push(PointStruct::new(
                point_id,
                embedding,
                payload,
            ));
        }
        
        // Upload points to Qdrant
        let upsert_points = UpsertPointsBuilder::new(&self.collection_name, points);
        
        self.qdrant_client
            .upsert_points(upsert_points)
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
        _tags_filter: Option<Vec<String>>,
    ) -> Result<Vec<DocumentMatch>> {
        debug!("Searching for: {}", query);
        
        // Generate embedding for query
        let query_embedding = self.generate_embedding(query).await?;
        
        // Search in Qdrant
        let search_points = SearchPointsBuilder::new(
            &self.collection_name,
            query_embedding,
            limit as u64,
        )
        .score_threshold(score_threshold)
        .with_payload(true);
        
        let search_result = self.qdrant_client
            .search_points(search_points)
            .await?;
        
        // Convert results to DocumentMatch
        let documents: Vec<DocumentMatch> = search_result
            .result
            .into_iter()
            .filter_map(|point| {
                let payload = point.payload;
                let title = payload.get("title")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "Unknown".to_string());
                
                let content = payload.get("content")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| String::new());
                
                let id = payload.get("id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| String::new());
                
                let chunk_index = payload.get("chunk_index")
                    .and_then(|v| v.as_integer())
                    .map(|i| i as usize)
                    .unwrap_or(0);
                
                let source = payload.get("source")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "Unknown".to_string());
                
                Some(DocumentMatch {
                    id,
                    title,
                    content,
                    score: point.score,
                    chunk_index,
                    source,
                })
            })
            .collect();
        
        info!("Found {} relevant documents", documents.len());
        
        Ok(documents)
    }
    
    // Get collection statistics
    pub async fn get_stats(&self) -> Result<serde_json::Value> {
        let collection_info = self.qdrant_client
            .collection_info(&self.collection_name)
            .await?;
        
        Ok(serde_json::json!({
            "collection": self.collection_name,
            "vectors_count": collection_info.result.as_ref().and_then(|r| r.vectors_count).unwrap_or(0),
            "indexed_vectors_count": collection_info.result.as_ref().and_then(|r| r.indexed_vectors_count).unwrap_or(0),
        }))
    }
}

// HTTP Handlers
pub async fn upload_document_handler(
    State(state): State<Arc<crate::AppState>>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    // Check if knowledge service is available
    let knowledge_service = match &state.knowledge_service {
        Some(service) => service,
        None => {
            return (StatusCode::SERVICE_UNAVAILABLE, "Knowledge base service is not available").into_response();
        }
    };
    
    let mut title = String::new();
    let mut content = String::new();
    let mut source = String::new();
    let mut tags = Vec::new();
    
    while let Some(mut field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap_or("").to_string();
        let filename = field.file_name().map(|s| s.to_string());
        let data = field.bytes().await.unwrap_or_default();
        let value = String::from_utf8_lossy(&data).to_string();
        
        match name.as_str() {
            "title" => title = value,
            "content" => content = value,
            "source" => source = value,
            "tags" => tags = value.split(',').map(|s| s.trim().to_string()).collect(),
            "file" => {
                // Handle file upload
                if let Some(fname) = filename {
                    source = fname;
                    content = value;
                }
            }
            _ => {}
        }
    }
    
    if title.is_empty() || content.is_empty() {
        return (StatusCode::BAD_REQUEST, "Title and content are required").into_response();
    }
    
    match knowledge_service.store_document(title, content, source, tags).await {
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
    // Check if knowledge service is available
    let knowledge_service = match &state.knowledge_service {
        Some(service) => service,
        None => {
            return (StatusCode::SERVICE_UNAVAILABLE, "Knowledge base service is not available").into_response();
        }
    };
    
    let limit = params.limit.unwrap_or(10);
    let threshold = params.threshold.unwrap_or(0.3);
    
    match knowledge_service.search_documents(
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
    // Check if knowledge service is available
    let knowledge_service = match &state.knowledge_service {
        Some(service) => service,
        None => {
            return Json(serde_json::json!({
                "collection": "unavailable",
                "vectors_count": 0,
                "indexed_vectors_count": 0,
                "message": "Knowledge base service is not available"
            })).into_response();
        }
    };
    
    match knowledge_service.get_stats().await {
        Ok(stats) => Json(stats).into_response(),
        Err(e) => {
            error!("Failed to get stats: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get statistics").into_response()
        }
    }
}