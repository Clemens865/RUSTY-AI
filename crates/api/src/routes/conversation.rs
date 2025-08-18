use crate::{
    auth::AuthenticatedUser,
    create_success_response,
    error::{ApiError, ApiResult},
};
use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use rusty_ai_common::{ConversationTurn, Intent};
use rusty_ai_core::AssistantCore;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error, info};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatRequest {
    pub message: String,
    pub session_id: Option<Uuid>,
    pub context: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub response: String,
    pub session_id: Uuid,
    pub intent: Intent,
    pub conversation_id: Uuid,
    pub processing_time_ms: u64,
    pub suggested_actions: Vec<SuggestedAction>,
}

#[derive(Debug, Serialize)]
pub struct SuggestedAction {
    pub action_type: String,
    pub label: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct ConversationHistory {
    pub session_id: Uuid,
    pub turns: Vec<ConversationTurn>,
    pub total_turns: usize,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_activity: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSessionRequest {
    pub preferences: Option<rusty_ai_common::UserPreferences>,
}

#[derive(Debug, Serialize)]
pub struct CreateSessionResponse {
    pub session_id: Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub fn routes(core: Arc<AssistantCore>) -> Router {
    Router::new()
        .route("/chat", post(chat))
        .route("/sessions", post(create_session))
        .route("/sessions/:session_id", get(get_session).delete(delete_session))
        .route("/sessions/:session_id/history", get(get_conversation_history))
        .route("/sessions/:session_id/context", get(get_session_context))
        .route("/active", get(get_active_sessions))
        .with_state(core)
}

// Main chat endpoint
async fn chat(
    State(core): State<Arc<AssistantCore>>,
    user: AuthenticatedUser,
    Json(request): Json<ChatRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let start_time = std::time::Instant::now();
    
    debug!("Chat request from user {}: {}", user.claims.user_id, request.message);

    // Validate input
    if request.message.trim().is_empty() {
        return Err(ApiError::Validation("Message cannot be empty".to_string()));
    }

    if request.message.len() > 10000 {
        return Err(ApiError::Validation("Message too long (max 10000 characters)".to_string()));
    }

    // Get or create session
    let session_id = match request.session_id {
        Some(id) => id,
        None => {
            // Create new session
            let preferences = rusty_ai_common::UserPreferences {
                language: "en".to_string(),
                timezone: "UTC".to_string(),
                voice_settings: rusty_ai_common::VoiceSettings {
                    enabled: false,
                    voice_id: "default".to_string(),
                    speed: 1.0,
                    pitch: 1.0,
                },
                notification_settings: rusty_ai_common::NotificationSettings {
                    enabled: false,
                    channels: vec![],
                    quiet_hours: None,
                },
            };
            
            core.context_manager
                .write()
                .await
                .create_session(user.claims.user_id, preferences)
                .await
                .map_err(|e| ApiError::CoreService(e))?
        }
    };

    // Classify intent
    let intent = {
        let context_manager = core.context_manager.read().await;
        let user_context = context_manager
            .get_user_context(session_id)
            .await
            .map_err(|e| ApiError::CoreService(e))?;
        
        let classification = core.intent_classifier.classify(&request.message, Some(user_context));
        classification.intent
    };

    // Process the message through orchestrator
    let response = {
        let context_manager = core.context_manager.read().await;
        let user_context = context_manager
            .get_user_context(session_id)
            .await
            .map_err(|e| ApiError::CoreService(e))?;
            
        core.orchestrator
            .process_intent(intent.clone(), user_context)
            .await
            .map_err(|e| ApiError::CoreService(e))?
    };

    // Update conversation history
    {
        let mut context_manager = core.context_manager.write().await;
        context_manager
            .add_conversation_turn(session_id, request.message.clone(), response.clone(), intent.clone())
            .await
            .map_err(|e| ApiError::CoreService(e))?;
    }

    let processing_time = start_time.elapsed().as_millis() as u64;
    let conversation_id = Uuid::new_v4();

    // Generate suggested actions based on intent
    let suggested_actions = generate_suggested_actions(&intent, &response);

    info!(
        "Chat response generated for user {} in {}ms", 
        user.claims.user_id, 
        processing_time
    );

    Ok(create_success_response(ChatResponse {
        response,
        session_id,
        intent,
        conversation_id,
        processing_time_ms: processing_time,
        suggested_actions,
    }))
}

// Create new conversation session
async fn create_session(
    State(core): State<Arc<AssistantCore>>,
    user: AuthenticatedUser,
    Json(request): Json<CreateSessionRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    debug!("Creating new session for user {}", user.claims.user_id);

    let preferences = request.preferences.unwrap_or_else(|| {
        rusty_ai_common::UserPreferences {
            language: "en".to_string(),
            timezone: "UTC".to_string(),
            voice_settings: rusty_ai_common::VoiceSettings {
                enabled: false,
                voice_id: "default".to_string(),
                speed: 1.0,
                pitch: 1.0,
            },
            notification_settings: rusty_ai_common::NotificationSettings {
                enabled: false,
                channels: vec![],
                quiet_hours: None,
            },
        }
    });

    let session_id = core
        .context_manager
        .write()
        .await
        .create_session(user.claims.user_id, preferences)
        .await
        .map_err(|e| ApiError::CoreService(e))?;

    info!("Created session {} for user {}", session_id, user.claims.user_id);

    Ok(create_success_response(CreateSessionResponse {
        session_id,
        created_at: chrono::Utc::now(),
    }))
}

// Get session information
async fn get_session(
    State(core): State<Arc<AssistantCore>>,
    Path(session_id): Path<Uuid>,
    user: AuthenticatedUser,
) -> ApiResult<Json<serde_json::Value>> {
    debug!("Getting session {} for user {}", session_id, user.claims.user_id);

    let context_manager = core.context_manager.read().await;
    let session = context_manager
        .get_session(session_id)
        .await
        .map_err(|e| ApiError::CoreService(e))?;

    // Verify user owns this session
    if session.user_id != user.claims.user_id {
        return Err(ApiError::Authorization("Access denied to this session".to_string()));
    }

    let summary = context_manager
        .get_session_summary(session_id)
        .await
        .map_err(|e| ApiError::CoreService(e))?;

    Ok(create_success_response(summary))
}

// Delete session
async fn delete_session(
    State(core): State<Arc<AssistantCore>>,
    Path(session_id): Path<Uuid>,
    user: AuthenticatedUser,
) -> ApiResult<Json<serde_json::Value>> {
    debug!("Deleting session {} for user {}", session_id, user.claims.user_id);

    // Verify user owns this session first
    {
        let context_manager = core.context_manager.read().await;
        let session = context_manager
            .get_session(session_id)
            .await
            .map_err(|e| ApiError::CoreService(e))?;

        if session.user_id != user.claims.user_id {
            return Err(ApiError::Authorization("Access denied to this session".to_string()));
        }
    }

    core.context_manager
        .write()
        .await
        .destroy_session(session_id)
        .await
        .map_err(|e| ApiError::CoreService(e))?;

    info!("Deleted session {} for user {}", session_id, user.claims.user_id);

    Ok(create_success_response(serde_json::json!({
        "message": "Session deleted successfully"
    })))
}

// Get conversation history
async fn get_conversation_history(
    State(core): State<Arc<AssistantCore>>,
    Path(session_id): Path<Uuid>,
    Query(query): Query<HistoryQuery>,
    user: AuthenticatedUser,
) -> ApiResult<Json<serde_json::Value>> {
    debug!("Getting conversation history for session {}", session_id);

    let context_manager = core.context_manager.read().await;
    
    // Verify user owns this session
    let session = context_manager
        .get_session(session_id)
        .await
        .map_err(|e| ApiError::CoreService(e))?;

    if session.user_id != user.claims.user_id {
        return Err(ApiError::Authorization("Access denied to this session".to_string()));
    }

    let turns = context_manager
        .get_conversation_history(session_id, query.limit)
        .await
        .map_err(|e| ApiError::CoreService(e))?;

    let history = ConversationHistory {
        session_id,
        turns: if let Some(offset) = query.offset {
            turns.into_iter().skip(offset).collect()
        } else {
            turns
        },
        total_turns: session.conversation_turns.len(),
        created_at: session.created_at,
        last_activity: session.last_activity,
    };

    Ok(create_success_response(history))
}

// Get session context
async fn get_session_context(
    State(core): State<Arc<AssistantCore>>,
    Path(session_id): Path<Uuid>,
    user: AuthenticatedUser,
) -> ApiResult<Json<serde_json::Value>> {
    debug!("Getting session context for session {}", session_id);

    let context_manager = core.context_manager.read().await;
    let user_context = context_manager
        .get_user_context(session_id)
        .await
        .map_err(|e| ApiError::CoreService(e))?;

    // Verify user owns this session
    if user_context.user_id != user.claims.user_id {
        return Err(ApiError::Authorization("Access denied to this session".to_string()));
    }

    Ok(create_success_response(user_context))
}

// Get active sessions for user
async fn get_active_sessions(
    State(core): State<Arc<AssistantCore>>,
    user: AuthenticatedUser,
) -> ApiResult<Json<serde_json::Value>> {
    debug!("Getting active sessions for user {}", user.claims.user_id);

    let context_manager = core.context_manager.read().await;
    let sessions = context_manager
        .get_user_sessions(user.claims.user_id)
        .await;

    let session_summaries: Vec<_> = sessions
        .iter()
        .map(|session| {
            serde_json::json!({
                "session_id": session.session_id,
                "created_at": session.created_at,
                "last_activity": session.last_activity,
                "turn_count": session.conversation_turns.len(),
                "active_plugins": session.context.active_plugins
            })
        })
        .collect();

    Ok(create_success_response(session_summaries))
}

// Helper function to generate suggested actions
fn generate_suggested_actions(intent: &Intent, _response: &str) -> Vec<SuggestedAction> {
    match intent {
        Intent::Query { .. } => vec![
            SuggestedAction {
                action_type: "search".to_string(),
                label: "Search for more information".to_string(),
                description: "Search the knowledge base for related content".to_string(),
                parameters: serde_json::json!({}),
            },
            SuggestedAction {
                action_type: "clarify".to_string(),
                label: "Ask for clarification".to_string(),
                description: "Ask follow-up questions for more details".to_string(),
                parameters: serde_json::json!({}),
            },
        ],
        Intent::Command { action, .. } => {
            if action == "task" {
                vec![
                    SuggestedAction {
                        action_type: "view_tasks".to_string(),
                        label: "View all tasks".to_string(),
                        description: "See your current task list".to_string(),
                        parameters: serde_json::json!({}),
                    },
                    SuggestedAction {
                        action_type: "schedule".to_string(),
                        label: "Schedule task".to_string(),
                        description: "Set due date and reminders".to_string(),
                        parameters: serde_json::json!({}),
                    },
                ]
            } else {
                vec![]
            }
        },
        Intent::Information { .. } => vec![
            SuggestedAction {
                action_type: "related".to_string(),
                label: "Show related topics".to_string(),
                description: "Find related information and documents".to_string(),
                parameters: serde_json::json!({}),
            },
        ],
        Intent::Unknown => vec![
            SuggestedAction {
                action_type: "help".to_string(),
                label: "Get help".to_string(),
                description: "Learn about available commands and features".to_string(),
                parameters: serde_json::json!({}),
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{AuthConfig, AuthService, Claims};
    use rusty_ai_core::CoreConfig;
    use std::sync::Arc;

    async fn create_test_setup() -> (Arc<AssistantCore>, AuthenticatedUser) {
        let core_config = CoreConfig::default();
        let core = Arc::new(AssistantCore::new(core_config).await.unwrap());
        
        let user = AuthenticatedUser {
            claims: Claims {
                sub: "test-user".to_string(),
                name: "Test User".to_string(),
                email: "test@example.com".to_string(),
                iat: chrono::Utc::now().timestamp(),
                exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp(),
                iss: "test".to_string(),
                aud: "test".to_string(),
                user_id: uuid::Uuid::new_v4(),
                session_id: uuid::Uuid::new_v4(),
                permissions: vec!["read".to_string(), "write".to_string()],
            },
        };

        (core, user)
    }

    #[tokio::test]
    async fn test_create_session() {
        let (core, user) = create_test_setup().await;
        
        let request = CreateSessionRequest {
            preferences: None,
        };

        // This would normally be tested with the full router
        // For now, we'll test the core functionality
        let session_id = core
            .context_manager
            .write()
            .await
            .create_session(
                user.claims.user_id,
                rusty_ai_common::UserPreferences {
                    language: "en".to_string(),
                    timezone: "UTC".to_string(),
                    voice_settings: rusty_ai_common::VoiceSettings {
                        enabled: false,
                        voice_id: "default".to_string(),
                        speed: 1.0,
                        pitch: 1.0,
                    },
                    notification_settings: rusty_ai_common::NotificationSettings {
                        enabled: false,
                        channels: vec![],
                        quiet_hours: None,
                    },
                }
            )
            .await
            .unwrap();

        assert!(!session_id.is_nil());
    }

    #[test]
    fn test_suggested_actions_generation() {
        let intent = Intent::Query { query: "test".to_string() };
        let actions = generate_suggested_actions(&intent, "test response");
        
        assert!(!actions.is_empty());
        assert!(actions.iter().any(|a| a.action_type == "search"));
    }
}