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
mod memory_service;
use ai_service::{AIService, ConversationStore};
use voice_service::VoiceService;
use knowledge_service_simple::{KnowledgeService, upload_document_handler, search_documents_handler, knowledge_stats_handler, list_documents_handler};
use memory_service::MemoryService;

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
    pub memory_service: Option<Arc<MemoryService>>,
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
    
    // Initialize memory service (requires knowledge service)
    let memory_service = match &knowledge_service {
        Some(ks) => {
            match MemoryService::new(None, Arc::clone(ks)) {
                Ok(service) => {
                    info!("Memory service initialized successfully");
                    Some(Arc::new(service))
                }
                Err(e) => {
                    error!("Failed to initialize memory service: {}", e);
                    None
                }
            }
        }
        None => {
            info!("Memory service disabled (requires knowledge service)");
            None
        }
    };
    
    // Create application state
    let state = Arc::new(AppState {
        ai_service: Arc::new(ai_service),
        conversation_store: Arc::new(conversation_store),
        voice_service: Arc::new(voice_service),
        knowledge_service,
        memory_service,
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
        .route("/api/v1/conversation/sessions", get(get_sessions))
        .route("/api/v1/conversation/session/:id", get(get_session_messages))
        
        // Voice endpoints
        .route("/api/v1/voice/transcribe", post(voice_service::transcribe_handler))
        .route("/api/v1/voice/synthesize", post(voice_service::synthesize_handler))
        .route("/api/v1/voice/health", get(voice_service::voice_health))
        
        // Knowledge base endpoints
        .route("/api/v1/knowledge/upload", post(upload_document_handler))
        .route("/api/v1/knowledge/search", get(search_documents_handler))
        .route("/api/v1/knowledge/stats", get(knowledge_stats_handler))
        .route("/api/v1/knowledge/documents", get(list_documents_handler))
        
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
        // Search with lower threshold to find more matches
        if let Ok(search_results) = knowledge_service
            .search_documents(&payload.message, 5, 0.1, None)
            .await 
        {
            if !search_results.is_empty() {
                context = format!(
                    "\n\nRelevant information from your memory:\n{}",
                    search_results
                        .iter()
                        .map(|doc| format!("- {}: {}", doc.title, doc.content))
                        .collect::<Vec<_>>()
                        .join("\n")
                );
                info!("Found {} relevant documents for context", search_results.len());
            } else {
                debug!("No relevant documents found for: {}", payload.message);
            }
        } else {
            debug!("Failed to search knowledge base");
        }
    }
    
    // Combine user message with context
    let enhanced_message = if !context.is_empty() {
        format!("User's question: {}\n\nIMPORTANT - Use this information from previous conversations:{}\n\nAnswer the user's question. If the context contains relevant information (like their name or preferences), use it in your response.", 
                payload.message, context)
    } else {
        payload.message.clone()
    };
    
    debug!("Enhanced message with context: {}", enhanced_message);
    
    // Process message with AI service
    let response = match state.ai_service.process_message(&enhanced_message, &session_id).await {
        Ok(resp) => resp,
        Err(e) => {
            error!("Error processing message: {}", e);
            format!("I apologize, but I encountered an error processing your message. Please try again.")
        }
    };
    
    // Save to database for persistence
    if let Ok(_) = state.conversation_store.save_session(&ai_service::SessionRecord {
        id: session_id.clone(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        metadata: None,
    }).await {
        // Session saved
    }
    
    if let Ok(_) = state.conversation_store.save_message(&ai_service::MessageRecord {
        id: uuid::Uuid::new_v4().to_string(),
        session_id: session_id.clone(),
        role: "user".to_string(),
        content: payload.message.clone(),
        created_at: chrono::Utc::now(),
    }).await {
        // User message saved
    }
    
    if let Ok(_) = state.conversation_store.save_message(&ai_service::MessageRecord {
        id: uuid::Uuid::new_v4().to_string(),
        session_id: session_id.clone(),
        role: "assistant".to_string(),
        content: response.clone(),
        created_at: chrono::Utc::now(),
    }).await {
        // Assistant response saved
    }
    
    // Extract and store important information using memory service
    if let Some(ref memory_service) = state.memory_service {
        tokio::spawn({
            let memory_service = Arc::clone(memory_service);
            let session_id = session_id.clone();
            let user_message = payload.message.clone();
            let assistant_response = response.clone();
            
            async move {
                match memory_service.process_conversation(
                    &session_id,
                    &user_message,
                    &assistant_response
                ).await {
                    Ok(extracted) => {
                        if !extracted.is_empty() {
                            info!("Extracted {} pieces of information from conversation {}", 
                                  extracted.len(), session_id);
                        }
                    }
                    Err(e) => {
                        error!("Failed to extract information from conversation: {}", e);
                    }
                }
            }
        });
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

// Get all conversation sessions
async fn get_sessions(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    match state.conversation_store.get_recent_sessions(20).await {
        Ok(sessions) => {
            // Get summaries for sessions if memory service is available
            let sessions_with_info: Vec<serde_json::Value> = sessions
                .into_iter()
                .map(|session| {
                    serde_json::json!({
                        "id": session.id,
                        "created_at": session.created_at,
                        "updated_at": session.updated_at,
                        "metadata": session.metadata,
                    })
                })
                .collect();
            
            Json(serde_json::json!({
                "sessions": sessions_with_info,
                "total": sessions_with_info.len()
            }))
        }
        Err(e) => {
            error!("Failed to get sessions: {}", e);
            Json(serde_json::json!({
                "sessions": [],
                "error": "Failed to retrieve sessions"
            }))
        }
    }
}

// Get messages for a specific session
async fn get_session_messages(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> impl IntoResponse {
    match state.conversation_store.get_session_messages(&session_id).await {
        Ok(messages) => {
            let messages_formatted: Vec<serde_json::Value> = messages
                .into_iter()
                .map(|msg| {
                    serde_json::json!({
                        "id": msg.id,
                        "role": msg.role,
                        "content": msg.content,
                        "created_at": msg.created_at,
                    })
                })
                .collect();
            
            // Get summary if memory service is available
            let summary = if let Some(ref memory_service) = state.memory_service {
                let msg_pairs: Vec<(String, String)> = messages_formatted
                    .iter()
                    .map(|m| {
                        (
                            m["role"].as_str().unwrap_or("").to_string(),
                            m["content"].as_str().unwrap_or("").to_string()
                        )
                    })
                    .collect();
                
                match memory_service.summarize_conversation(&msg_pairs).await {
                    Ok(summary) => Some(summary),
                    Err(_) => None
                }
            } else {
                None
            };
            
            Json(serde_json::json!({
                "session_id": session_id,
                "messages": messages_formatted,
                "summary": summary,
                "total": messages_formatted.len()
            }))
        }
        Err(e) => {
            error!("Failed to get messages for session {}: {}", session_id, e);
            Json(serde_json::json!({
                "session_id": session_id,
                "messages": [],
                "error": "Failed to retrieve messages"
            }))
        }
    }
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