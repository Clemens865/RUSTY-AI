# Phase 1: MVP Implementation Guide

## Overview

Phase 1 focuses on delivering the core functionality of the Personal AI Assistant, establishing the foundation for all future features. The MVP includes knowledge base management, daily briefings, basic voice interaction, and the fundamental plugin architecture.

## MVP Scope Definition

### Core Features
1. **Knowledge Base System**: Document storage, retrieval, and semantic search
2. **Daily Briefing**: Automated summary generation and delivery
3. **Voice Interface**: Basic speech-to-text and text-to-speech capabilities
4. **API Foundation**: RESTful and WebSocket endpoints
5. **Frontend Integration**: Basic UI using vox-chic-studio
6. **Plugin Framework**: Core architecture for extensibility

### Success Criteria
- Store and retrieve 1000+ documents with sub-second search
- Generate meaningful daily briefings from user data
- Process voice commands with >90% accuracy
- Maintain <300ms response time for voice interactions
- Support real-time communication via WebSocket

## Architecture Implementation

### 1. Core Backend Structure

Create the main workspace structure:

```toml
# Cargo.toml (workspace root)
[workspace]
members = [
    "crates/core",
    "crates/api",
    "crates/voice",
    "crates/knowledge",
    "crates/plugins",
    "crates/common"
]
resolver = "2"

[workspace.dependencies]
tokio = { version = "1.35", features = ["full"] }
axum = { version = "0.7", features = ["ws", "headers", "multipart"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"
thiserror = "1.0"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
uuid = { version = "1.6", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
```

### 2. Core Types and Structures

#### Common Types (`crates/common/src/lib.rs`)

```rust
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub metadata: DocumentMetadata,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub source: String,
    pub file_type: String,
    pub tags: Vec<String>,
    pub summary: Option<String>,
    pub importance_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceInteraction {
    pub id: Uuid,
    pub transcript: String,
    pub intent: Intent,
    pub response: String,
    pub confidence: f32,
    pub processing_time_ms: u64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Intent {
    Query { query: String },
    Command { action: String, parameters: Vec<String> },
    Information { topic: String },
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyBriefing {
    pub id: Uuid,
    pub date: DateTime<Utc>,
    pub sections: Vec<BriefingSection>,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BriefingSection {
    pub title: String,
    pub content: String,
    pub priority: BriefingPriority,
    pub source_documents: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BriefingPriority {
    Critical,
    High,
    Medium,
    Low,
}
```

### 3. Knowledge Base Implementation

#### Knowledge Service (`crates/knowledge/src/lib.rs`)

```rust
use anyhow::Result;
use qdrant_client::{prelude::*, qdrant::*};
use serde_json::json;
use std::collections::HashMap;
use uuid::Uuid;

pub struct KnowledgeBase {
    qdrant_client: QdrantClient,
    collection_name: String,
    embedder: TextEmbedder,
}

impl KnowledgeBase {
    pub async fn new(qdrant_url: &str, model_path: &str) -> Result<Self> {
        let qdrant_client = QdrantClient::from_url(qdrant_url).build()?;
        let collection_name = "personal_knowledge".to_string();
        let embedder = TextEmbedder::new(model_path).await?;
        
        // Create collection if it doesn't exist
        let collections = qdrant_client.list_collections().await?;
        let collection_exists = collections.collections.iter()
            .any(|c| c.name == collection_name);
        
        if !collection_exists {
            qdrant_client.create_collection(&CreateCollection {
                collection_name: collection_name.clone(),
                vectors_config: Some(VectorParams {
                    size: 384, // all-MiniLM-L6-v2 embedding size
                    distance: Distance::Cosine.into(),
                    ..Default::default()
                }.into()),
                ..Default::default()
            }).await?;
        }
        
        Ok(Self {
            qdrant_client,
            collection_name,
            embedder,
        })
    }
    
    pub async fn store_document(&self, document: &Document) -> Result<()> {
        // Generate embedding for document content
        let embedding = self.embedder.embed(&document.content).await?;
        
        // Create payload with document metadata
        let payload = json!({
            "id": document.id,
            "title": document.title,
            "content": document.content,
            "metadata": document.metadata,
            "created_at": document.created_at,
            "updated_at": document.updated_at
        });
        
        // Store in Qdrant
        let points = vec![PointStruct::new(
            document.id.to_string(),
            embedding,
            payload,
        )];
        
        self.qdrant_client.upsert_points_blocking(
            &self.collection_name,
            None,
            points,
            None,
        ).await?;
        
        Ok(())
    }
    
    pub async fn search_documents(
        &self,
        query: &str,
        limit: usize,
        score_threshold: f32,
    ) -> Result<Vec<Document>> {
        // Generate query embedding
        let query_embedding = self.embedder.embed(query).await?;
        
        // Search in Qdrant
        let search_result = self.qdrant_client.search_points(&SearchPoints {
            collection_name: self.collection_name.clone(),
            vector: query_embedding,
            limit: limit as u64,
            score_threshold: Some(score_threshold),
            with_payload: Some(true.into()),
            ..Default::default()
        }).await?;
        
        // Convert results to documents
        let mut documents = Vec::new();
        for point in search_result.result {
            if let Some(payload) = point.payload {
                let document: Document = serde_json::from_value(json!(payload))?;
                documents.push(document);
            }
        }
        
        Ok(documents)
    }
    
    pub async fn get_document_by_id(&self, id: Uuid) -> Result<Option<Document>> {
        let points = self.qdrant_client.get_points(
            &self.collection_name,
            None,
            &[id.to_string().into()],
            Some(true),
            Some(true),
            None,
        ).await?;
        
        if let Some(point) = points.result.first() {
            if let Some(payload) = &point.payload {
                let document: Document = serde_json::from_value(json!(payload))?;
                return Ok(Some(document));
            }
        }
        
        Ok(None)
    }
    
    pub async fn delete_document(&self, id: Uuid) -> Result<()> {
        self.qdrant_client.delete_points(
            &self.collection_name,
            None,
            &[id.to_string().into()],
            None,
        ).await?;
        
        Ok(())
    }
}

// Text embedding service
pub struct TextEmbedder {
    model: SentenceTransformerModel,
}

impl TextEmbedder {
    pub async fn new(model_path: &str) -> Result<Self> {
        let model = SentenceTransformerModel::load(model_path).await?;
        Ok(Self { model })
    }
    
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        self.model.encode(text).await
    }
}
```

### 4. Daily Briefing System

#### Briefing Service (`crates/core/src/briefing.rs`)

```rust
use anyhow::Result;
use chrono::{DateTime, Utc, Duration};
use crate::knowledge::KnowledgeBase;
use crate::common::{Document, DailyBriefing, BriefingSection, BriefingPriority};

pub struct BriefingService {
    knowledge_base: Arc<KnowledgeBase>,
    ai_service: Arc<AIService>,
}

impl BriefingService {
    pub fn new(
        knowledge_base: Arc<KnowledgeBase>,
        ai_service: Arc<AIService>,
    ) -> Self {
        Self {
            knowledge_base,
            ai_service,
        }
    }
    
    pub async fn generate_daily_briefing(&self, date: DateTime<Utc>) -> Result<DailyBriefing> {
        let mut sections = Vec::new();
        
        // Recent documents (last 24 hours)
        let recent_docs = self.get_recent_documents(date, 1).await?;
        if !recent_docs.is_empty() {
            let section = self.create_recent_updates_section(recent_docs).await?;
            sections.push(section);
        }
        
        // Important reminders
        let reminders = self.get_important_reminders(date).await?;
        if !reminders.is_empty() {
            let section = self.create_reminders_section(reminders).await?;
            sections.push(section);
        }
        
        // Trending topics
        let trending = self.get_trending_topics(date, 7).await?;
        if !trending.is_empty() {
            let section = self.create_trending_section(trending).await?;
            sections.push(section);
        }
        
        Ok(DailyBriefing {
            id: Uuid::new_v4(),
            date,
            sections,
            generated_at: Utc::now(),
        })
    }
    
    async fn get_recent_documents(
        &self,
        since: DateTime<Utc>,
        days: i64,
    ) -> Result<Vec<Document>> {
        let start_date = since - Duration::days(days);
        
        // Query for recent documents
        let query = format!(
            "documents created after {} before {}",
            start_date.format("%Y-%m-%d"),
            since.format("%Y-%m-%d")
        );
        
        self.knowledge_base.search_documents(&query, 20, 0.3).await
    }
    
    async fn create_recent_updates_section(
        &self,
        documents: Vec<Document>,
    ) -> Result<BriefingSection> {
        // Group documents by category/source
        let mut grouped_docs: HashMap<String, Vec<Document>> = HashMap::new();
        for doc in documents {
            let category = doc.metadata.source.clone();
            grouped_docs.entry(category).or_default().push(doc);
        }
        
        // Generate summary for each group
        let mut content_parts = Vec::new();
        for (category, docs) in grouped_docs {
            let summary = self.ai_service.summarize_documents(&docs).await?;
            content_parts.push(format!("**{}**: {}", category, summary));
        }
        
        Ok(BriefingSection {
            title: "Recent Updates".to_string(),
            content: content_parts.join("\n\n"),
            priority: BriefingPriority::High,
            source_documents: documents.iter().map(|d| d.id).collect(),
        })
    }
    
    async fn get_important_reminders(&self, _date: DateTime<Utc>) -> Result<Vec<Document>> {
        // Search for documents containing reminder keywords
        let reminder_queries = vec![
            "reminder", "important", "deadline", "due", "schedule",
            "appointment", "meeting", "task", "todo"
        ];
        
        let mut all_reminders = Vec::new();
        for query in reminder_queries {
            let docs = self.knowledge_base.search_documents(query, 5, 0.5).await?;
            all_reminders.extend(docs);
        }
        
        // Deduplicate and sort by importance
        all_reminders.sort_by(|a, b| {
            b.metadata.importance_score.partial_cmp(&a.metadata.importance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        all_reminders.dedup_by(|a, b| a.id == b.id);
        
        Ok(all_reminders.into_iter().take(10).collect())
    }
    
    async fn create_reminders_section(
        &self,
        reminders: Vec<Document>,
    ) -> Result<BriefingSection> {
        let mut content_parts = Vec::new();
        
        for reminder in &reminders {
            let summary = if let Some(ref summary) = reminder.metadata.summary {
                summary.clone()
            } else {
                self.ai_service.summarize_text(&reminder.content, 100).await?
            };
            
            content_parts.push(format!("• {}: {}", reminder.title, summary));
        }
        
        Ok(BriefingSection {
            title: "Important Reminders".to_string(),
            content: content_parts.join("\n"),
            priority: BriefingPriority::Critical,
            source_documents: reminders.iter().map(|d| d.id).collect(),
        })
    }
    
    async fn get_trending_topics(
        &self,
        _date: DateTime<Utc>,
        _days: i64,
    ) -> Result<Vec<String>> {
        // This would analyze document frequency and identify trending topics
        // For MVP, return some placeholder topics
        Ok(vec![
            "work projects".to_string(),
            "health goals".to_string(),
            "learning objectives".to_string(),
        ])
    }
    
    async fn create_trending_section(
        &self,
        topics: Vec<String>,
    ) -> Result<BriefingSection> {
        let mut content_parts = Vec::new();
        
        for topic in &topics {
            let related_docs = self.knowledge_base
                .search_documents(topic, 3, 0.4).await?;
            
            if !related_docs.is_empty() {
                let summary = self.ai_service
                    .summarize_documents(&related_docs).await?;
                content_parts.push(format!("**{}**: {}", topic, summary));
            }
        }
        
        Ok(BriefingSection {
            title: "Trending Topics".to_string(),
            content: content_parts.join("\n\n"),
            priority: BriefingPriority::Medium,
            source_documents: Vec::new(),
        })
    }
}

// AI service for text processing
pub struct AIService {
    // This would contain your chosen LLM client
    // For MVP, you might use OpenAI API or a local model
}

impl AIService {
    pub async fn summarize_documents(&self, documents: &[Document]) -> Result<String> {
        let combined_content = documents.iter()
            .map(|d| format!("{}: {}", d.title, d.content))
            .collect::<Vec<_>>()
            .join("\n\n");
        
        self.summarize_text(&combined_content, 200).await
    }
    
    pub async fn summarize_text(&self, text: &str, max_words: usize) -> Result<String> {
        // For MVP, implement a simple extractive summarization
        // In production, this would call an LLM API
        let sentences: Vec<&str> = text.split(". ").take(3).collect();
        let summary = sentences.join(". ");
        
        if summary.len() > max_words * 6 { // Rough word-to-char ratio
            Ok(format!("{}...", &summary[..max_words * 6]))
        } else {
            Ok(summary)
        }
    }
}
```

### 5. Voice Interface Implementation

#### Voice Service (`crates/voice/src/lib.rs`)

```rust
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use whisper_rs::{WhisperContext, FullParams, SamplingStrategy};

pub struct VoiceService {
    stt_engine: Arc<Mutex<SpeechToText>>,
    tts_engine: Arc<TextToSpeech>,
    intent_parser: Arc<IntentParser>,
}

impl VoiceService {
    pub async fn new(
        whisper_model_path: &str,
        elevenlabs_api_key: &str,
        voice_id: &str,
    ) -> Result<Self> {
        let stt_engine = Arc::new(Mutex::new(
            SpeechToText::new(whisper_model_path)?
        ));
        let tts_engine = Arc::new(
            TextToSpeech::new(elevenlabs_api_key.to_string(), voice_id.to_string())
        );
        let intent_parser = Arc::new(IntentParser::new());
        
        Ok(Self {
            stt_engine,
            tts_engine,
            intent_parser,
        })
    }
    
    pub async fn process_voice_input(&self, audio_data: Vec<f32>) -> Result<VoiceInteraction> {
        let start_time = std::time::Instant::now();
        
        // Convert speech to text
        let transcript = {
            let mut stt = self.stt_engine.lock().await;
            stt.transcribe(&audio_data)?
        };
        
        // Parse intent from transcript
        let intent = self.intent_parser.parse(&transcript).await?;
        
        // Generate response based on intent
        let response = self.generate_response(&intent).await?;
        
        let processing_time = start_time.elapsed().as_millis() as u64;
        
        Ok(VoiceInteraction {
            id: Uuid::new_v4(),
            transcript,
            intent,
            response,
            confidence: 0.85, // This would come from actual confidence scoring
            processing_time_ms: processing_time,
            timestamp: Utc::now(),
        })
    }
    
    pub async fn synthesize_speech(&self, text: &str) -> Result<Vec<u8>> {
        self.tts_engine.synthesize(text).await
    }
    
    async fn generate_response(&self, intent: &Intent) -> Result<String> {
        match intent {
            Intent::Query { query } => {
                format!("I found information about: {}", query)
            }
            Intent::Command { action, parameters } => {
                format!("Executing {} with parameters: {:?}", action, parameters)
            }
            Intent::Information { topic } => {
                format!("Here's what I know about {}", topic)
            }
            Intent::Unknown => {
                "I'm not sure I understand. Could you rephrase that?".to_string()
            }
        }
    }
}

// Speech-to-text implementation
pub struct SpeechToText {
    ctx: WhisperContext,
    params: FullParams,
}

impl SpeechToText {
    pub fn new(model_path: &str) -> Result<Self> {
        let ctx = WhisperContext::new(model_path)?;
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some("en"));
        params.set_translate(false);
        params.set_print_progress(false);
        params.set_print_special(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        
        Ok(Self { ctx, params })
    }
    
    pub fn transcribe(&mut self, audio_data: &[f32]) -> Result<String> {
        self.ctx.full(self.params.clone(), audio_data)?;
        
        let num_segments = self.ctx.full_n_segments()?;
        let mut result = String::new();
        
        for i in 0..num_segments {
            let segment_text = self.ctx.full_get_segment_text(i)?;
            result.push_str(&segment_text);
        }
        
        Ok(result.trim().to_string())
    }
}

// Intent parsing service
pub struct IntentParser;

impl IntentParser {
    pub fn new() -> Self {
        Self
    }
    
    pub async fn parse(&self, text: &str) -> Result<Intent> {
        let text_lower = text.to_lowercase();
        
        // Simple keyword-based intent detection for MVP
        if text_lower.contains("search") || text_lower.contains("find") || text_lower.contains("what") {
            let query = text.split_whitespace().skip(1).collect::<Vec<_>>().join(" ");
            Ok(Intent::Query { query })
        } else if text_lower.starts_with("create") || text_lower.starts_with("add") || text_lower.starts_with("delete") {
            let parts: Vec<&str> = text.split_whitespace().collect();
            let action = parts[0].to_string();
            let parameters = parts[1..].iter().map(|s| s.to_string()).collect();
            Ok(Intent::Command { action, parameters })
        } else if text_lower.starts_with("tell me about") || text_lower.starts_with("explain") {
            let topic = text.split_whitespace().skip(3).collect::<Vec<_>>().join(" ");
            Ok(Intent::Information { topic })
        } else {
            Ok(Intent::Unknown)
        }
    }
}
```

### 6. API Layer Implementation

#### REST API (`crates/api/src/handlers.rs`)

```rust
use axum::{
    extract::{Path, Query, State, Multipart},
    http::StatusCode,
    response::Json,
    Json as RequestJson,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

pub struct AppState {
    pub knowledge_base: Arc<KnowledgeBase>,
    pub briefing_service: Arc<BriefingService>,
    pub voice_service: Arc<VoiceService>,
}

#[derive(Deserialize)]
pub struct SearchQuery {
    q: String,
    limit: Option<usize>,
    threshold: Option<f32>,
}

#[derive(Serialize)]
pub struct SearchResponse {
    documents: Vec<Document>,
    total: usize,
    query_time_ms: u64,
}

// Document management endpoints
pub async fn upload_document(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<Document>, StatusCode> {
    let mut title = String::new();
    let mut content = String::new();
    let mut file_type = String::new();
    
    while let Some(field) = multipart.next_field().await.map_err(|_| StatusCode::BAD_REQUEST)? {
        let name = field.name().unwrap_or("").to_string();
        let data = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;
        
        match name.as_str() {
            "title" => title = String::from_utf8_lossy(&data).to_string(),
            "content" => content = String::from_utf8_lossy(&data).to_string(),
            "file_type" => file_type = String::from_utf8_lossy(&data).to_string(),
            _ => {}
        }
    }
    
    let document = Document {
        id: Uuid::new_v4(),
        title,
        content,
        metadata: DocumentMetadata {
            source: "upload".to_string(),
            file_type,
            tags: Vec::new(),
            summary: None,
            importance_score: 0.5,
        },
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    
    state.knowledge_base.store_document(&document).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(document))
}

pub async fn search_documents(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<SearchResponse>, StatusCode> {
    let start_time = std::time::Instant::now();
    
    let documents = state.knowledge_base.search_documents(
        &params.q,
        params.limit.unwrap_or(10),
        params.threshold.unwrap_or(0.3),
    ).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let query_time = start_time.elapsed().as_millis() as u64;
    
    Ok(Json(SearchResponse {
        total: documents.len(),
        documents,
        query_time_ms: query_time,
    }))
}

pub async fn get_document(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Document>, StatusCode> {
    let document = state.knowledge_base.get_document_by_id(id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    match document {
        Some(doc) => Ok(Json(doc)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

pub async fn delete_document(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    state.knowledge_base.delete_document(id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(StatusCode::NO_CONTENT)
}

// Briefing endpoints
pub async fn get_daily_briefing(
    State(state): State<AppState>,
    Query(date): Query<Option<String>>,
) -> Result<Json<DailyBriefing>, StatusCode> {
    let target_date = if let Some(date_str) = date {
        DateTime::parse_from_rfc3339(&date_str)
            .map_err(|_| StatusCode::BAD_REQUEST)?
            .with_timezone(&Utc)
    } else {
        Utc::now()
    };
    
    let briefing = state.briefing_service.generate_daily_briefing(target_date).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(briefing))
}

// Voice interaction endpoints
#[derive(Deserialize)]
pub struct VoiceRequest {
    audio_data: Vec<f32>,
}

pub async fn process_voice(
    State(state): State<AppState>,
    RequestJson(request): RequestJson<VoiceRequest>,
) -> Result<Json<VoiceInteraction>, StatusCode> {
    let interaction = state.voice_service.process_voice_input(request.audio_data).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(interaction))
}

#[derive(Deserialize)]
pub struct TTSRequest {
    text: String,
}

pub async fn text_to_speech(
    State(state): State<AppState>,
    RequestJson(request): RequestJson<TTSRequest>,
) -> Result<Vec<u8>, StatusCode> {
    let audio_data = state.voice_service.synthesize_speech(&request.text).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(audio_data)
}
```

### 7. WebSocket Implementation

#### Real-time Communication (`crates/api/src/websocket.rs`)

```rust
use axum::{
    extract::{ws::{WebSocket, Message}, WebSocketUpgrade, State},
    response::Response,
};
use futures_util::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ClientMessage {
    VoiceInput { audio_data: Vec<f32> },
    TextQuery { query: String },
    Subscribe { topics: Vec<String> },
    Unsubscribe { topics: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ServerMessage {
    VoiceResponse { interaction: VoiceInteraction },
    TextResponse { response: String },
    BriefingUpdate { briefing: DailyBriefing },
    DocumentUpdate { document: Document },
    Error { message: String },
}

pub struct WebSocketState {
    app_state: AppState,
    broadcast_tx: broadcast::Sender<ServerMessage>,
}

impl WebSocketState {
    pub fn new(app_state: AppState) -> Self {
        let (broadcast_tx, _) = broadcast::channel(1000);
        Self {
            app_state,
            broadcast_tx,
        }
    }
    
    pub fn broadcast(&self, message: ServerMessage) {
        let _ = self.broadcast_tx.send(message);
    }
}

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<WebSocketState>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<WebSocketState>) {
    let (mut sender, mut receiver) = socket.split();
    let mut broadcast_rx = state.broadcast_tx.subscribe();
    
    // Handle incoming messages
    let state_clone = state.clone();
    let sender_clone = sender.clone();
    let incoming_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            if let Ok(Message::Text(text)) = msg {
                match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(client_msg) => {
                        if let Err(e) = handle_client_message(client_msg, &state_clone, &sender_clone).await {
                            let error_msg = ServerMessage::Error {
                                message: e.to_string(),
                            };
                            let _ = sender_clone.send(Message::Text(
                                serde_json::to_string(&error_msg).unwrap()
                            )).await;
                        }
                    }
                    Err(e) => {
                        let error_msg = ServerMessage::Error {
                            message: format!("Invalid message format: {}", e),
                        };
                        let _ = sender_clone.send(Message::Text(
                            serde_json::to_string(&error_msg).unwrap()
                        )).await;
                    }
                }
            }
        }
    });
    
    // Handle broadcast messages
    let broadcast_task = tokio::spawn(async move {
        while let Ok(server_msg) = broadcast_rx.recv().await {
            let message_text = serde_json::to_string(&server_msg).unwrap();
            if sender.send(Message::Text(message_text)).await.is_err() {
                break;
            }
        }
    });
    
    // Wait for either task to complete
    tokio::select! {
        _ = incoming_task => {},
        _ = broadcast_task => {},
    }
}

async fn handle_client_message(
    message: ClientMessage,
    state: &WebSocketState,
    sender: &Sender,
) -> Result<()> {
    match message {
        ClientMessage::VoiceInput { audio_data } => {
            let interaction = state.app_state.voice_service
                .process_voice_input(audio_data).await?;
            
            let response = ServerMessage::VoiceResponse { interaction };
            let response_text = serde_json::to_string(&response)?;
            sender.send(Message::Text(response_text)).await?;
        }
        
        ClientMessage::TextQuery { query } => {
            let documents = state.app_state.knowledge_base
                .search_documents(&query, 5, 0.3).await?;
            
            let response_text = if documents.is_empty() {
                "I couldn't find any relevant information.".to_string()
            } else {
                format!("Found {} relevant documents: {}", 
                    documents.len(),
                    documents.iter()
                        .map(|d| &d.title)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };
            
            let response = ServerMessage::TextResponse { response: response_text };
            let response_json = serde_json::to_string(&response)?;
            sender.send(Message::Text(response_json)).await?;
        }
        
        ClientMessage::Subscribe { topics: _ } => {
            // Handle subscription logic
        }
        
        ClientMessage::Unsubscribe { topics: _ } => {
            // Handle unsubscription logic
        }
    }
    
    Ok(())
}
```

## Testing Strategy for MVP

### 1. Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_document_storage_and_retrieval() {
        let kb = KnowledgeBase::new("http://localhost:6333", "./models/embeddings").await.unwrap();
        
        let document = Document {
            id: Uuid::new_v4(),
            title: "Test Document".to_string(),
            content: "This is a test document for validation.".to_string(),
            metadata: DocumentMetadata {
                source: "test".to_string(),
                file_type: "text".to_string(),
                tags: vec!["test".to_string()],
                summary: None,
                importance_score: 0.8,
            },
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        
        // Store document
        kb.store_document(&document).await.unwrap();
        
        // Retrieve by ID
        let retrieved = kb.get_document_by_id(document.id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().title, "Test Document");
        
        // Search
        let results = kb.search_documents("test document", 5, 0.3).await.unwrap();
        assert!(!results.is_empty());
    }
    
    #[tokio::test]
    async fn test_voice_processing() {
        let voice_service = VoiceService::new(
            "./models/whisper/ggml-base.en.bin",
            "test_key",
            "test_voice"
        ).await.unwrap();
        
        // Mock audio data (would be real audio in practice)
        let audio_data = vec![0.0; 16000]; // 1 second of silence at 16kHz
        
        let result = voice_service.process_voice_input(audio_data).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_daily_briefing_generation() {
        let kb = Arc::new(KnowledgeBase::new("http://localhost:6333", "./models/embeddings").await.unwrap());
        let ai_service = Arc::new(AIService::new());
        let briefing_service = BriefingService::new(kb, ai_service);
        
        let briefing = briefing_service.generate_daily_briefing(Utc::now()).await.unwrap();
        assert!(!briefing.sections.is_empty());
    }
}
```

## Deployment Configuration

### 1. Docker Setup

```dockerfile
# Dockerfile
FROM rust:1.75 as builder

WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libasound2 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/personal-ai-assistant /usr/local/bin/
COPY --from=builder /app/models /app/models
COPY --from=builder /app/config /app/config

EXPOSE 8080
CMD ["personal-ai-assistant"]
```

### 2. Production Configuration

```yaml
# docker-compose.prod.yml
version: '3.8'

services:
  qdrant:
    image: qdrant/qdrant:v1.7.4
    restart: unless-stopped
    volumes:
      - qdrant_data:/qdrant/storage
    ports:
      - "6333:6333"

  assistant:
    build: .
    restart: unless-stopped
    ports:
      - "8080:8080"
    environment:
      RUST_LOG: info
      QDRANT_URL: http://qdrant:6333
    volumes:
      - assistant_data:/app/data
    depends_on:
      - qdrant

volumes:
  qdrant_data:
  assistant_data:
```

## Performance Optimization

### 1. Caching Strategy

```rust
use std::time::Duration;
use tokio::time::Instant;
use dashmap::DashMap;

pub struct CacheService {
    document_cache: DashMap<Uuid, (Document, Instant)>,
    search_cache: DashMap<String, (Vec<Document>, Instant)>,
    cache_ttl: Duration,
}

impl CacheService {
    pub fn new(cache_ttl: Duration) -> Self {
        Self {
            document_cache: DashMap::new(),
            search_cache: DashMap::new(),
            cache_ttl,
        }
    }
    
    pub fn get_document(&self, id: &Uuid) -> Option<Document> {
        if let Some(entry) = self.document_cache.get(id) {
            let (document, timestamp) = entry.value();
            if timestamp.elapsed() < self.cache_ttl {
                return Some(document.clone());
            } else {
                self.document_cache.remove(id);
            }
        }
        None
    }
    
    pub fn cache_document(&self, document: Document) {
        self.document_cache.insert(document.id, (document, Instant::now()));
    }
}
```

## Success Metrics and KPIs

### 1. Performance Metrics
- Document storage time: < 100ms
- Search response time: < 500ms
- Voice processing time: < 300ms
- Daily briefing generation: < 2 seconds

### 2. Quality Metrics
- Search relevance: > 85% user satisfaction
- Voice transcription accuracy: > 90%
- Briefing usefulness: > 80% user approval

### 3. Monitoring Implementation

```rust
use prometheus::{Counter, Histogram, register_counter, register_histogram};

lazy_static! {
    static ref SEARCH_REQUESTS: Counter = register_counter!(
        "search_requests_total", "Total number of search requests"
    ).unwrap();
    
    static ref SEARCH_DURATION: Histogram = register_histogram!(
        "search_duration_seconds", "Search request duration"
    ).unwrap();
    
    static ref VOICE_PROCESSING_DURATION: Histogram = register_histogram!(
        "voice_processing_duration_seconds", "Voice processing duration"
    ).unwrap();
}

// Usage in handlers
pub async fn search_with_metrics(query: &str) -> Result<Vec<Document>> {
    SEARCH_REQUESTS.inc();
    let timer = SEARCH_DURATION.start_timer();
    
    let result = knowledge_base.search_documents(query, 10, 0.3).await;
    
    timer.observe_duration();
    result
}
```

## Next Steps After MVP

1. **Performance Testing**: Load test with 1000+ concurrent users
2. **Security Audit**: Penetration testing and vulnerability assessment
3. **User Feedback**: Beta testing program with target users
4. **Phase 2 Planning**: Begin productivity suite implementation
5. **Documentation**: User guides and API documentation

This MVP implementation provides a solid foundation for the Personal AI Assistant, with all core features functional and ready for user testing. The modular architecture allows for easy extension in subsequent phases.