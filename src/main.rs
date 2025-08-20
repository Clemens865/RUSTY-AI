use anyhow::Result;
use axum::{
    extract::{State, Json, WebSocketUpgrade},
    http::{Method, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_http::cors::{CorsLayer, Any};
use tracing::{info, error, debug};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod ai_service;
mod voice_service;
mod knowledge_service_simple;
use ai_service::{AIService, ConversationStore};
use voice_service::VoiceService;
use knowledge_service_simple::{KnowledgeService, upload_document_handler, search_documents_handler, knowledge_stats_handler};

// Request/Response structures
#[derive(Debug, Deserialize)]
struct ChatRequest {
    message: String,
    #[serde(default)]
    session_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct ChatResponse {
    response: String,
    session_id: String,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: String,
    service: String,
    version: String,
}

// Application state
#[derive(Clone)]
pub struct AppState {
    pub ai_service: Arc<AIService>,
    pub conversation_store: Arc<ConversationStore>,
    pub voice_service: Arc<VoiceService>,
    pub knowledge_service: Option<Arc<KnowledgeService>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rusty_ai=debug,tower_http=debug,axum=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting Personal AI Assistant API...");
    
    // Load environment variables
    dotenv::dotenv().ok();
    
    // Initialize AI service
    let ai_service = AIService::new(None)?; // Will use OPENAI_API_KEY env var
    
    // Initialize conversation store
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:./data/rusty_ai.db".to_string());
    let conversation_store = ConversationStore::new(&database_url).await?;
    
    // Initialize voice service
    let elevenlabs_api_key = std::env::var("ELEVENLABS_API_KEY").ok();
    let voice_service = VoiceService::new(None, elevenlabs_api_key)?;
    
    // Initialize knowledge service (optional - if Qdrant is not available, backend can still run)
    let knowledge_service = match KnowledgeService::new(None).await {
        Ok(service) => {
            info!("Knowledge service initialized successfully");
            Some(Arc::new(service))
        }
        Err(e) => {
            error!("Failed to initialize knowledge service (Qdrant may not be running): {}", e);
            info!("Starting without knowledge base features - chat and voice will still work");
            None
        }
    };
    
    // Create application state
    let state = Arc::new(AppState {
        ai_service: Arc::new(ai_service),
        conversation_store: Arc::new(conversation_store),
        voice_service: Arc::new(voice_service),
        knowledge_service,
    });
    
    // Build the router
    let app = Router::new()
        // Health check endpoints
        .route("/health", get(health_check))
        .route("/health/ready", get(health_ready))
        .route("/health/live", get(health_live))
        
        // API v1 routes
        .route("/api/v1/conversation/send", post(chat_handler))
        .route("/api/v1/conversation/history", get(get_history))
        
        // Voice endpoints
        .route("/api/v1/voice/transcribe", post(voice_service::transcribe_handler))
        .route("/api/v1/voice/synthesize", post(voice_service::synthesize_handler))
        .route("/api/v1/voice/health", get(voice_service::voice_health))
        
        // Knowledge base endpoints
        .route("/api/v1/knowledge/upload", post(upload_document_handler))
        .route("/api/v1/knowledge/search", get(search_documents_handler))
        .route("/api/v1/knowledge/stats", get(knowledge_stats_handler))
        
        // WebSocket endpoint
        .route("/ws", get(websocket_handler))
        
        // Add state
        .with_state(state)
        
        // Add CORS layer to allow frontend connections
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
                .allow_headers(Any)
        );
    
    // Get the port from environment or use default
    let port = std::env::var("PORT").unwrap_or_else(|_| "8081".to_string());
    let addr = format!("0.0.0.0:{}", port);
    
    info!("Server starting on http://{}", addr);
    
    // Parse the address
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("Server listening on http://{}", addr);
    
    // Start the server
    axum::serve(listener, app)
        .await
        .map_err(|e| anyhow::anyhow!("Server error: {}", e))?;
    
    Ok(())
}

// Health check handlers
async fn health_check() -> impl IntoResponse {
    Json(HealthResponse {
        status: "healthy".to_string(),
        service: "rusty-ai".to_string(),
        version: "0.1.0".to_string(),
    })
}

async fn health_ready() -> impl IntoResponse {
    Json(serde_json::json!({
        "ready": true,
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

async fn health_live() -> impl IntoResponse {
    Json(serde_json::json!({
        "alive": true,
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

// Chat handler with RAG
async fn chat_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ChatRequest>,
) -> impl IntoResponse {
    debug!("Received chat request: {:?}", payload);
    
    let session_id = payload.session_id.unwrap_or_else(|| {
        uuid::Uuid::new_v4().to_string()
    });
    
    // Search knowledge base for relevant context (if available)
    let mut context = String::new();
    if let Some(ref knowledge_service) = state.knowledge_service {
        if let Ok(search_results) = knowledge_service
            .search_documents(&payload.message, 3, 0.3, None)
            .await 
        {
            if !search_results.is_empty() {
                context = format!(
                    "\n\nRelevant information from knowledge base:\n{}",
                    search_results
                        .iter()
                        .map(|doc| format!("- {}: {}", doc.title, doc.content))
                        .collect::<Vec<_>>()
                        .join("\n")
                );
                debug!("Found {} relevant documents for context", search_results.len());
            }
        }
    }
    
    // Combine user message with context
    let enhanced_message = if !context.is_empty() {
        format!("{}\n\nContext:{}\n\nPlease answer based on the provided context when relevant.", 
                payload.message, context)
    } else {
        payload.message.clone()
    };
    
    // Process message with AI service
    let response = match state.ai_service.process_message(&enhanced_message, &session_id).await {
        Ok(resp) => resp,
        Err(e) => {
            error!("Error processing message: {}", e);
            format!("I apologize, but I encountered an error processing your message. Please try again.")
        }
    };
    
    // Save to database for persistence
    if let Ok(store) = state.conversation_store.save_session(&ai_service::SessionRecord {
        id: session_id.clone(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        metadata: None,
    }).await {
        // Session saved
    }
    
    if let Ok(store) = state.conversation_store.save_message(&ai_service::MessageRecord {
        id: uuid::Uuid::new_v4().to_string(),
        session_id: session_id.clone(),
        role: "user".to_string(),
        content: payload.message.clone(),
        created_at: chrono::Utc::now(),
    }).await {
        // User message saved
    }
    
    if let Ok(store) = state.conversation_store.save_message(&ai_service::MessageRecord {
        id: uuid::Uuid::new_v4().to_string(),
        session_id: session_id.clone(),
        role: "assistant".to_string(),
        content: response.clone(),
        created_at: chrono::Utc::now(),
    }).await {
        // Assistant response saved
    }
    
    info!("Sending AI response for session: {}", session_id);
    
    Json(ChatResponse {
        response,
        session_id,
    })
}

// Get conversation history
async fn get_history(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    debug!("Getting conversation history");
    
    // For now, return empty history
    // In production, you would get session_id from query params or auth token
    Json(serde_json::json!({
        "history": [],
        "message": "History endpoint ready. Pass session_id as query parameter to get specific session history."
    }))
}

// WebSocket handler
async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: axum::extract::ws::WebSocket, state: Arc<AppState>) {
    info!("New WebSocket connection established");
    
    // Send a welcome message
    let welcome = serde_json::json!({
        "type": "welcome",
        "message": "Connected to Personal AI Assistant"
    });
    
    if let Err(e) = socket.send(axum::extract::ws::Message::Text(
        welcome.to_string()
    )).await {
        error!("Failed to send welcome message: {}", e);
        return;
    }
    
    // Handle incoming messages
    while let Some(msg) = socket.recv().await {
        match msg {
            Ok(axum::extract::ws::Message::Text(text)) => {
                debug!("Received WebSocket message: {}", text);
                
                // Echo the message back for now
                let response = serde_json::json!({
                    "type": "response",
                    "message": format!("Echo: {}", text)
                });
                
                if let Err(e) = socket.send(axum::extract::ws::Message::Text(
                    response.to_string()
                )).await {
                    error!("Failed to send response: {}", e);
                    break;
                }
            }
            Ok(axum::extract::ws::Message::Close(_)) => {
                info!("WebSocket connection closed by client");
                break;
            }
            Err(e) => {
                error!("WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }
    
    info!("WebSocket connection closed");
}