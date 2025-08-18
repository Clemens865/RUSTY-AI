pub mod routes;
pub mod middleware;
pub mod websocket;
pub mod auth;
pub mod server;
pub mod error;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use rusty_ai_common::{ApiResponse, AssistantError};
use serde_json::json;
use std::sync::Arc;
use tracing::error;

pub use server::ApiServer;

// Re-export common types
pub use rusty_ai_common;
pub use rusty_ai_core;

#[derive(Debug, Clone)]
pub struct ApiConfig {
    pub host: String,
    pub port: u16,
    pub cors_origins: Vec<String>,
    pub jwt_secret: String,
    pub enable_websockets: bool,
    pub max_request_size: usize,
    pub rate_limit_requests_per_minute: u32,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            cors_origins: vec!["*".to_string()],
            jwt_secret: "default-secret-change-in-production".to_string(),
            enable_websockets: true,
            max_request_size: 16 * 1024 * 1024, // 16MB
            rate_limit_requests_per_minute: 60,
        }
    }
}

// Global error handling
impl IntoResponse for AssistantError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AssistantError::Database(msg) => {
                error!("Database error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error occurred")
            }
            AssistantError::Api(msg) => {
                error!("API error: {}", msg);
                (StatusCode::BAD_REQUEST, msg.as_str())
            }
            AssistantError::VoiceProcessing(msg) => {
                error!("Voice processing error: {}", msg);
                (StatusCode::UNPROCESSABLE_ENTITY, "Voice processing failed")
            }
            AssistantError::Plugin(msg) => {
                error!("Plugin error: {}", msg);
                (StatusCode::SERVICE_UNAVAILABLE, "Plugin service unavailable")
            }
            AssistantError::Configuration(msg) => {
                error!("Configuration error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "Configuration error")
            }
            AssistantError::NotFound(msg) => {
                (StatusCode::NOT_FOUND, msg.as_str())
            }
            AssistantError::Unauthorized => {
                (StatusCode::UNAUTHORIZED, "Unauthorized access")
            }
            AssistantError::Internal(msg) => {
                error!("Internal error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
            }
        };

        let response = ApiResponse::<()>::error(error_message.to_string());
        (status, Json(response)).into_response()
    }
}

// Health check response
#[derive(serde::Serialize)]
pub struct HealthCheck {
    pub status: String,
    pub version: String,
    pub uptime: u64,
    pub services: ServiceHealth,
}

#[derive(serde::Serialize)]
pub struct ServiceHealth {
    pub database: String,
    pub storage: String,
    pub plugins: String,
    pub voice: String,
}

// Common API utilities
pub fn create_success_response<T: serde::Serialize>(data: T) -> Json<ApiResponse<T>> {
    Json(ApiResponse::success(data))
}

pub fn create_error_response(message: String) -> Json<ApiResponse<()>> {
    Json(ApiResponse::error(message))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_config_default() {
        let config = ApiConfig::default();
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 8080);
        assert!(config.enable_websockets);
    }

    #[test]
    fn test_success_response_creation() {
        let data = json!({"message": "test"});
        let response = create_success_response(data);
        assert!(response.0.success);
    }

    #[test]
    fn test_error_response_creation() {
        let response = create_error_response("Test error".to_string());
        assert!(!response.0.success);
        assert_eq!(response.0.error, Some("Test error".to_string()));
    }
}