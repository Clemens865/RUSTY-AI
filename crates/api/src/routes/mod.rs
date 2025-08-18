pub mod health;
pub mod auth;
pub mod conversation;
pub mod plugins;
pub mod knowledge;
pub mod tasks;
pub mod briefing;
pub mod voice;

use axum::{routing::get, Router};
use std::sync::Arc;
use crate::auth::AuthService;
use rusty_ai_core::AssistantCore;

pub fn create_routes(
    core: Arc<AssistantCore>,
    auth_service: Arc<AuthService>,
) -> Router {
    Router::new()
        // Health check routes (no authentication required)
        .nest("/health", health::routes())
        
        // Authentication routes
        .nest("/auth", auth::routes(auth_service.clone()))
        
        // Protected routes (require authentication)
        .nest("/api/v1", protected_routes(core, auth_service))
}

fn protected_routes(
    core: Arc<AssistantCore>,
    auth_service: Arc<AuthService>,
) -> Router {
    Router::new()
        // Conversation endpoints
        .nest("/conversation", conversation::routes(core.clone()))
        
        // Plugin management endpoints
        .nest("/plugins", plugins::routes(core.clone()))
        
        // Knowledge base endpoints
        .nest("/knowledge", knowledge::routes(core.clone()))
        
        // Task management endpoints
        .nest("/tasks", tasks::routes(core.clone()))
        
        // Daily briefing endpoints
        .nest("/briefing", briefing::routes(core.clone()))
        
        // Voice interaction endpoints
        .nest("/voice", voice::routes(core.clone()))
        
        // Add auth service to state for authentication middleware
        .with_state(auth_service)
}

// Fallback handler for unmatched routes
pub async fn not_found_handler() -> axum::http::StatusCode {
    axum::http::StatusCode::NOT_FOUND
}