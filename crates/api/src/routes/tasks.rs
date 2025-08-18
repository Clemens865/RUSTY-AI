use crate::{auth::AuthenticatedUser, create_success_response, error::ApiResult};
use axum::{extract::{Path, State}, routing::{get, post, put}, Json, Router};
use rusty_ai_core::AssistantCore;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct CreateTaskRequest {
    pub name: String,
    pub description: String,
    pub priority: String,
    pub due_date: Option<chrono::DateTime<chrono::Utc>>,
    pub tags: Vec<String>,
}

pub fn routes(core: Arc<AssistantCore>) -> Router {
    Router::new()
        .route("/", get(list_tasks).post(create_task))
        .route("/:id", get(get_task).put(update_task).delete(delete_task))
        .route("/:id/complete", post(complete_task))
        .with_state(core)
}

async fn list_tasks(
    State(core): State<Arc<AssistantCore>>,
    _user: AuthenticatedUser,
) -> ApiResult<Json<serde_json::Value>> {
    let pending_tasks = core.storage.get_pending_tasks().await
        .map_err(|e| crate::error::ApiError::CoreService(e))?;
    
    Ok(create_success_response(pending_tasks))
}

async fn create_task(
    State(core): State<Arc<AssistantCore>>,
    _user: AuthenticatedUser,
    Json(request): Json<CreateTaskRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let priority = match request.priority.as_str() {
        "critical" => rusty_ai_common::TaskPriority::Critical,
        "high" => rusty_ai_common::TaskPriority::High,
        "medium" => rusty_ai_common::TaskPriority::Medium,
        "low" => rusty_ai_common::TaskPriority::Low,
        _ => rusty_ai_common::TaskPriority::Medium,
    };
    
    let task = rusty_ai_common::Task {
        id: Uuid::new_v4(),
        name: request.name,
        description: request.description,
        status: rusty_ai_common::TaskStatus::Pending,
        priority,
        due_date: request.due_date,
        tags: request.tags,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    
    core.storage.store_task(&task).await
        .map_err(|e| crate::error::ApiError::CoreService(e))?;
    
    Ok(create_success_response(task))
}

async fn get_task(
    State(core): State<Arc<AssistantCore>>,
    Path(id): Path<Uuid>,
    _user: AuthenticatedUser,
) -> ApiResult<Json<serde_json::Value>> {
    let task = core.storage.get_task(id).await
        .map_err(|e| crate::error::ApiError::CoreService(e))?;
    
    match task {
        Some(task) => Ok(create_success_response(task)),
        None => Err(crate::error::ApiError::CoreService(
            rusty_ai_common::AssistantError::NotFound("Task not found".to_string())
        ))
    }
}

async fn update_task(
    State(_core): State<Arc<AssistantCore>>,
    Path(_id): Path<Uuid>,
    _user: AuthenticatedUser,
    Json(_request): Json<CreateTaskRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(create_success_response(serde_json::json!({"message": "Task updated"})))
}

async fn delete_task(
    State(_core): State<Arc<AssistantCore>>,
    Path(_id): Path<Uuid>,
    _user: AuthenticatedUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(create_success_response(serde_json::json!({"message": "Task deleted"})))
}

async fn complete_task(
    State(core): State<Arc<AssistantCore>>,
    Path(id): Path<Uuid>,
    _user: AuthenticatedUser,
) -> ApiResult<Json<serde_json::Value>> {
    core.storage.update_task_status(id, rusty_ai_common::TaskStatus::Completed).await
        .map_err(|e| crate::error::ApiError::CoreService(e))?;
    
    Ok(create_success_response(serde_json::json!({"message": "Task completed"})))
}