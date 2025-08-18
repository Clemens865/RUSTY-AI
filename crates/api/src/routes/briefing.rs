use crate::{auth::AuthenticatedUser, create_success_response, error::ApiResult};
use axum::{extract::{Path, State}, routing::{get, post}, Json, Router};
use rusty_ai_core::AssistantCore;
use std::sync::Arc;
use uuid::Uuid;

pub fn routes(core: Arc<AssistantCore>) -> Router {
    Router::new()
        .route("/today", get(get_today_briefing))
        .route("/generate", post(generate_briefing))
        .route("/:id", get(get_briefing))
        .route("/history", get(get_briefing_history))
        .with_state(core)
}

async fn get_today_briefing(
    State(core): State<Arc<AssistantCore>>,
    user: AuthenticatedUser,
) -> ApiResult<Json<serde_json::Value>> {
    let briefing = core.briefing_generator.get_latest_briefing().await
        .map_err(|e| crate::error::ApiError::CoreService(e))?;
    
    Ok(create_success_response(briefing))
}

async fn generate_briefing(
    State(core): State<Arc<AssistantCore>>,
    user: AuthenticatedUser,
) -> ApiResult<Json<serde_json::Value>> {
    let user_context = rusty_ai_common::UserContext {
        user_id: user.claims.user_id,
        session_id: user.claims.session_id,
        preferences: rusty_ai_common::UserPreferences {
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
        },
        active_plugins: vec![],
        conversation_history: vec![],
    };
    
    let briefing = core.briefing_generator
        .generate_daily_briefing(chrono::Utc::now(), &user_context).await
        .map_err(|e| crate::error::ApiError::CoreService(e))?;
    
    Ok(create_success_response(briefing))
}

async fn get_briefing(
    State(core): State<Arc<AssistantCore>>,
    Path(id): Path<Uuid>,
    _user: AuthenticatedUser,
) -> ApiResult<Json<serde_json::Value>> {
    let briefing = core.storage.get_briefing(id).await
        .map_err(|e| crate::error::ApiError::CoreService(e))?;
    
    match briefing {
        Some(briefing) => Ok(create_success_response(briefing)),
        None => Err(crate::error::ApiError::CoreService(
            rusty_ai_common::AssistantError::NotFound("Briefing not found".to_string())
        ))
    }
}

async fn get_briefing_history(
    State(core): State<Arc<AssistantCore>>,
    _user: AuthenticatedUser,
) -> ApiResult<Json<serde_json::Value>> {
    let briefings = core.briefing_generator.get_briefing_history(7).await
        .map_err(|e| crate::error::ApiError::CoreService(e))?;
    
    Ok(create_success_response(briefings))
}