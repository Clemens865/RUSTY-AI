use crate::{auth::AuthenticatedUser, create_success_response, error::ApiResult};
use axum::{extract::{Path, State}, routing::{get, post}, Json, Router};
use rusty_ai_core::AssistantCore;
use std::sync::Arc;

pub fn routes(core: Arc<AssistantCore>) -> Router {
    Router::new()
        .route("/", get(list_plugins))
        .route("/:plugin_id", get(get_plugin).post(configure_plugin))
        .route("/:plugin_id/enable", post(enable_plugin))
        .route("/:plugin_id/disable", post(disable_plugin))
        .with_state(core)
}

async fn list_plugins(
    State(_core): State<Arc<AssistantCore>>,
    _user: AuthenticatedUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(create_success_response(serde_json::json!({"plugins": []})))
}

async fn get_plugin(
    State(_core): State<Arc<AssistantCore>>,
    Path(_plugin_id): Path<String>,
    _user: AuthenticatedUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(create_success_response(serde_json::json!({"plugin": {}})))
}

async fn configure_plugin(
    State(_core): State<Arc<AssistantCore>>,
    Path(_plugin_id): Path<String>,
    _user: AuthenticatedUser,
    Json(_config): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(create_success_response(serde_json::json!({"message": "Plugin configured"})))
}

async fn enable_plugin(
    State(_core): State<Arc<AssistantCore>>,
    Path(_plugin_id): Path<String>,
    _user: AuthenticatedUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(create_success_response(serde_json::json!({"message": "Plugin enabled"})))
}

async fn disable_plugin(
    State(_core): State<Arc<AssistantCore>>,
    Path(_plugin_id): Path<String>,
    _user: AuthenticatedUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(create_success_response(serde_json::json!({"message": "Plugin disabled"})))
}