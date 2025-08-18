use crate::{auth::AuthenticatedUser, create_success_response, error::ApiResult};
use axum::{extract::{Path, Query, State}, routing::{get, post}, Json, Router};
use rusty_ai_core::AssistantCore;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub limit: Option<usize>,
}

#[derive(Serialize, Deserialize)]
pub struct DocumentUpload {
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
}

pub fn routes(core: Arc<AssistantCore>) -> Router {
    Router::new()
        .route("/search", get(search_documents))
        .route("/documents", post(upload_document).get(list_documents))
        .route("/documents/:id", get(get_document).delete(delete_document))
        .with_state(core)
}

async fn search_documents(
    State(core): State<Arc<AssistantCore>>,
    Query(query): Query<SearchQuery>,
    _user: AuthenticatedUser,
) -> ApiResult<Json<serde_json::Value>> {
    let limit = query.limit.unwrap_or(10);
    let documents = core.storage.search_documents(&query.q, limit).await
        .map_err(|e| crate::error::ApiError::CoreService(e))?;
    
    Ok(create_success_response(serde_json::json!({
        "documents": documents,
        "total": documents.len()
    })))
}

async fn upload_document(
    State(core): State<Arc<AssistantCore>>,
    _user: AuthenticatedUser,
    Json(upload): Json<DocumentUpload>,
) -> ApiResult<Json<serde_json::Value>> {
    let document = rusty_ai_common::Document {
        id: Uuid::new_v4(),
        title: upload.title,
        content: upload.content,
        metadata: rusty_ai_common::DocumentMetadata {
            source: "api_upload".to_string(),
            file_type: "text".to_string(),
            tags: upload.tags,
            summary: None,
            importance_score: 0.5,
            embeddings: None,
        },
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    
    core.storage.store_document(&document).await
        .map_err(|e| crate::error::ApiError::CoreService(e))?;
    
    Ok(create_success_response(document))
}

async fn list_documents(
    State(core): State<Arc<AssistantCore>>,
    _user: AuthenticatedUser,
) -> ApiResult<Json<serde_json::Value>> {
    let documents = core.storage.search_documents("", 50).await
        .map_err(|e| crate::error::ApiError::CoreService(e))?;
    
    Ok(create_success_response(documents))
}

async fn get_document(
    State(core): State<Arc<AssistantCore>>,
    Path(id): Path<Uuid>,
    _user: AuthenticatedUser,
) -> ApiResult<Json<serde_json::Value>> {
    let document = core.storage.get_document(id).await
        .map_err(|e| crate::error::ApiError::CoreService(e))?;
    
    match document {
        Some(doc) => Ok(create_success_response(doc)),
        None => Err(crate::error::ApiError::CoreService(
            rusty_ai_common::AssistantError::NotFound("Document not found".to_string())
        ))
    }
}

async fn delete_document(
    State(core): State<Arc<AssistantCore>>,
    Path(id): Path<Uuid>,
    _user: AuthenticatedUser,
) -> ApiResult<Json<serde_json::Value>> {
    core.storage.delete_document(id).await
        .map_err(|e| crate::error::ApiError::CoreService(e))?;
    
    Ok(create_success_response(serde_json::json!({
        "message": "Document deleted successfully"
    })))
}