use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use rusty_ai_common::ApiResponse;
use serde_json::json;
use thiserror::Error;
use tracing::error;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Authentication error: {0}")]
    Authentication(String),
    
    #[error("Authorization error: {0}")]
    Authorization(String),
    
    #[error("Rate limit exceeded")]
    RateLimit,
    
    #[error("Request too large")]
    RequestTooLarge,
    
    #[error("Invalid content type")]
    InvalidContentType,
    
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    #[error("WebSocket error: {0}")]
    WebSocket(String),
    
    #[error("Core service error: {0}")]
    CoreService(#[from] rusty_ai_common::AssistantError),
    
    #[error("Internal server error: {0}")]
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_message, error_code) = match self {
            ApiError::Validation(msg) => {
                (StatusCode::BAD_REQUEST, msg, "VALIDATION_ERROR")
            }
            ApiError::Authentication(msg) => {
                (StatusCode::UNAUTHORIZED, msg, "AUTHENTICATION_ERROR")
            }
            ApiError::Authorization(msg) => {
                (StatusCode::FORBIDDEN, msg, "AUTHORIZATION_ERROR")
            }
            ApiError::RateLimit => {
                (StatusCode::TOO_MANY_REQUESTS, "Rate limit exceeded".to_string(), "RATE_LIMIT")
            }
            ApiError::RequestTooLarge => {
                (StatusCode::PAYLOAD_TOO_LARGE, "Request payload too large".to_string(), "REQUEST_TOO_LARGE")
            }
            ApiError::InvalidContentType => {
                (StatusCode::UNSUPPORTED_MEDIA_TYPE, "Invalid content type".to_string(), "INVALID_CONTENT_TYPE")
            }
            ApiError::Serialization(msg) => {
                error!("Serialization error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "Serialization error".to_string(), "SERIALIZATION_ERROR")
            }
            ApiError::WebSocket(msg) => {
                error!("WebSocket error: {}", msg);
                (StatusCode::BAD_REQUEST, msg, "WEBSOCKET_ERROR")
            }
            ApiError::CoreService(err) => {
                error!("Core service error: {}", err);
                match err {
                    rusty_ai_common::AssistantError::NotFound(msg) => {
                        (StatusCode::NOT_FOUND, msg, "NOT_FOUND")
                    }
                    rusty_ai_common::AssistantError::Unauthorized => {
                        (StatusCode::UNAUTHORIZED, "Unauthorized".to_string(), "UNAUTHORIZED")
                    }
                    _ => {
                        (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string(), "INTERNAL_ERROR")
                    }
                }
            }
            ApiError::Internal(msg) => {
                error!("Internal error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string(), "INTERNAL_ERROR")
            }
        };

        let response_body = json!({
            "success": false,
            "error": error_message,
            "error_code": error_code,
            "timestamp": chrono::Utc::now()
        });

        (status, Json(response_body)).into_response()
    }
}

// Helper function to create validation errors
pub fn validation_error(message: &str) -> ApiError {
    ApiError::Validation(message.to_string())
}

// Helper function to create authentication errors
pub fn auth_error(message: &str) -> ApiError {
    ApiError::Authentication(message.to_string())
}

// Helper function to create authorization errors
pub fn authz_error(message: &str) -> ApiError {
    ApiError::Authorization(message.to_string())
}

// Helper function to create internal errors
pub fn internal_error(message: &str) -> ApiError {
    ApiError::Internal(message.to_string())
}

// Result type for API operations
pub type ApiResult<T> = Result<T, ApiError>;

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    #[test]
    fn test_validation_error() {
        let error = validation_error("Invalid input");
        assert!(matches!(error, ApiError::Validation(_)));
    }

    #[test]
    fn test_auth_error() {
        let error = auth_error("Invalid token");
        assert!(matches!(error, ApiError::Authentication(_)));
    }

    #[test]
    fn test_rate_limit_error() {
        let error = ApiError::RateLimit;
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    }
}