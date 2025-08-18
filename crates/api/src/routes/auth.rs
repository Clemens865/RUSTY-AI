use crate::{
    auth::{AuthService, LoginRequest, RefreshRequest},
    create_success_response,
    error::{ApiError, ApiResult},
};
use axum::{
    extract::State,
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info, warn};

#[derive(Debug, Serialize, Deserialize)]
pub struct LogoutRequest {
    pub token: String,
}

#[derive(Debug, Serialize)]
pub struct LogoutResponse {
    pub message: String,
}

pub fn routes(auth_service: Arc<AuthService>) -> Router {
    Router::new()
        .route("/login", post(login))
        .route("/refresh", post(refresh_token))
        .route("/logout", post(logout))
        .route("/validate", post(validate_token))
        .with_state(auth_service)
}

// Login endpoint
async fn login(
    State(auth_service): State<Arc<AuthService>>,
    Json(request): Json<LoginRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    debug!("Login attempt for email: {}", request.email);

    // Validate input
    if request.email.is_empty() {
        return Err(ApiError::Validation("Email is required".to_string()));
    }
    
    if request.password.is_empty() {
        return Err(ApiError::Validation("Password is required".to_string()));
    }

    // Basic email format validation
    if !request.email.contains('@') {
        return Err(ApiError::Validation("Invalid email format".to_string()));
    }

    match auth_service.authenticate(request).await {
        Ok(response) => {
            info!("User logged in successfully: {}", response.user.email);
            Ok(create_success_response(response))
        }
        Err(e) => {
            warn!("Login failed: {}", e);
            Err(e)
        }
    }
}

// Token refresh endpoint
async fn refresh_token(
    State(auth_service): State<Arc<AuthService>>,
    Json(request): Json<RefreshRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    debug!("Token refresh requested");

    if request.refresh_token.is_empty() {
        return Err(ApiError::Validation("Refresh token is required".to_string()));
    }

    match auth_service.refresh_token(request).await {
        Ok(response) => {
            info!("Token refreshed successfully");
            Ok(create_success_response(response))
        }
        Err(e) => {
            warn!("Token refresh failed: {}", e);
            Err(e)
        }
    }
}

// Logout endpoint
async fn logout(
    State(auth_service): State<Arc<AuthService>>,
    Json(request): Json<LogoutRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    debug!("Logout requested");

    if request.token.is_empty() {
        return Err(ApiError::Validation("Token is required".to_string()));
    }

    match auth_service.logout(&request.token).await {
        Ok(()) => {
            info!("User logged out successfully");
            Ok(create_success_response(LogoutResponse {
                message: "Logged out successfully".to_string(),
            }))
        }
        Err(e) => {
            warn!("Logout failed: {}", e);
            Err(e)
        }
    }
}

// Token validation endpoint
async fn validate_token(
    State(auth_service): State<Arc<AuthService>>,
    Json(request): Json<ValidateTokenRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    debug!("Token validation requested");

    if request.token.is_empty() {
        return Err(ApiError::Validation("Token is required".to_string()));
    }

    match auth_service.verify_token(&request.token) {
        Ok(claims) => {
            debug!("Token validated for user: {}", claims.user_id);
            Ok(create_success_response(ValidateTokenResponse {
                valid: true,
                user_id: claims.user_id,
                email: claims.email,
                expires_at: claims.exp,
                permissions: claims.permissions,
            }))
        }
        Err(e) => {
            warn!("Token validation failed: {}", e);
            Ok(create_success_response(ValidateTokenResponse {
                valid: false,
                user_id: uuid::Uuid::nil(),
                email: String::new(),
                expires_at: 0,
                permissions: Vec::new(),
            }))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidateTokenRequest {
    pub token: String,
}

#[derive(Debug, Serialize)]
pub struct ValidateTokenResponse {
    pub valid: bool,
    pub user_id: uuid::Uuid,
    pub email: String,
    pub expires_at: i64,
    pub permissions: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::AuthConfig;
    use axum::http::StatusCode;
    use tower::ServiceExt;

    async fn create_test_app() -> Router {
        let auth_config = AuthConfig::default();
        let auth_service = Arc::new(AuthService::new(auth_config));
        routes(auth_service)
    }

    #[tokio::test]
    async fn test_login_success() {
        let app = create_test_app().await;
        
        let login_request = LoginRequest {
            email: "demo@example.com".to_string(),
            password: "password".to_string(),
        };

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/login")
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(serde_json::to_string(&login_request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_login_invalid_credentials() {
        let app = create_test_app().await;
        
        let login_request = LoginRequest {
            email: "invalid@example.com".to_string(),
            password: "wrong".to_string(),
        };

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/login")
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(serde_json::to_string(&login_request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_login_validation_errors() {
        let app = create_test_app().await;
        
        let login_request = LoginRequest {
            email: "".to_string(),
            password: "password".to_string(),
        };

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/login")
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(serde_json::to_string(&login_request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_token_validation() {
        let auth_config = AuthConfig::default();
        let auth_service = AuthService::new(auth_config);
        
        // First, authenticate to get a token
        let login_request = LoginRequest {
            email: "demo@example.com".to_string(),
            password: "password".to_string(),
        };
        
        let login_response = auth_service.authenticate(login_request).await.unwrap();
        
        // Now validate the token
        let validate_request = ValidateTokenRequest {
            token: login_response.access_token,
        };

        let app = create_test_app().await;
        
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/validate")
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(serde_json::to_string(&validate_request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}