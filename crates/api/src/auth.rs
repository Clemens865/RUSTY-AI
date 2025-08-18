use crate::error::{ApiError, ApiResult};
use axum::{
    async_trait,
    extract::{FromRequestParts, TypedHeader},
    headers::{authorization::Bearer, Authorization},
    http::request::Parts,
    RequestPartsExt,
};
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error, warn};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub token_expiry_hours: i64,
    pub refresh_token_expiry_days: i64,
    pub issuer: String,
    pub audience: String,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            jwt_secret: "default-secret-change-in-production".to_string(),
            token_expiry_hours: 24,
            refresh_token_expiry_days: 30,
            issuer: "rusty-ai-assistant".to_string(),
            audience: "rusty-ai-users".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,        // Subject (user ID)
    pub name: String,       // User name
    pub email: String,      // User email
    pub iat: i64,          // Issued at
    pub exp: i64,          // Expiration time
    pub iss: String,       // Issuer
    pub aud: String,       // Audience
    pub user_id: Uuid,     // User UUID
    pub session_id: Uuid,  // Session UUID
    pub permissions: Vec<String>, // User permissions
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub user: UserInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AuthService {
    config: AuthConfig,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl AuthService {
    pub fn new(config: AuthConfig) -> Self {
        let encoding_key = EncodingKey::from_secret(config.jwt_secret.as_bytes());
        let decoding_key = DecodingKey::from_secret(config.jwt_secret.as_bytes());

        Self {
            config,
            encoding_key,
            decoding_key,
        }
    }

    pub async fn authenticate(&self, request: LoginRequest) -> ApiResult<LoginResponse> {
        // In a real implementation, you would:
        // 1. Hash the password and compare with stored hash
        // 2. Look up user in database
        // 3. Verify user credentials
        
        // For now, we'll use a simple mock authentication
        if request.email == "demo@example.com" && request.password == "password" {
            let user_id = Uuid::new_v4();
            let session_id = Uuid::new_v4();
            let permissions = vec!["read".to_string(), "write".to_string()];

            let access_token = self.create_access_token(
                user_id,
                session_id,
                "Demo User".to_string(),
                request.email.clone(),
                permissions.clone(),
            )?;

            let refresh_token = self.create_refresh_token(user_id)?;

            Ok(LoginResponse {
                access_token,
                refresh_token,
                token_type: "Bearer".to_string(),
                expires_in: self.config.token_expiry_hours * 3600,
                user: UserInfo {
                    id: user_id,
                    name: "Demo User".to_string(),
                    email: request.email,
                    permissions,
                },
            })
        } else {
            Err(ApiError::Authentication("Invalid credentials".to_string()))
        }
    }

    pub async fn refresh_token(&self, request: RefreshRequest) -> ApiResult<LoginResponse> {
        // Verify refresh token
        let claims = self.verify_token(&request.refresh_token)?;
        
        // In a real implementation, you would:
        // 1. Check if refresh token is still valid in database
        // 2. Look up current user information
        // 3. Generate new access token

        let session_id = Uuid::new_v4(); // New session for security
        
        let access_token = self.create_access_token(
            claims.user_id,
            session_id,
            claims.name.clone(),
            claims.email.clone(),
            claims.permissions.clone(),
        )?;

        let new_refresh_token = self.create_refresh_token(claims.user_id)?;

        Ok(LoginResponse {
            access_token,
            refresh_token: new_refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: self.config.token_expiry_hours * 3600,
            user: UserInfo {
                id: claims.user_id,
                name: claims.name,
                email: claims.email,
                permissions: claims.permissions,
            },
        })
    }

    pub fn verify_token(&self, token: &str) -> ApiResult<Claims> {
        let mut validation = Validation::default();
        validation.set_issuer(&[&self.config.issuer]);
        validation.set_audience(&[&self.config.audience]);

        match decode::<Claims>(token, &self.decoding_key, &validation) {
            Ok(token_data) => {
                debug!("Token verified for user: {}", token_data.claims.sub);
                Ok(token_data.claims)
            }
            Err(e) => {
                warn!("Token verification failed: {}", e);
                Err(ApiError::Authentication("Invalid token".to_string()))
            }
        }
    }

    fn create_access_token(
        &self,
        user_id: Uuid,
        session_id: Uuid,
        name: String,
        email: String,
        permissions: Vec<String>,
    ) -> ApiResult<String> {
        let now = Utc::now();
        let exp = now + Duration::hours(self.config.token_expiry_hours);

        let claims = Claims {
            sub: user_id.to_string(),
            name,
            email,
            iat: now.timestamp(),
            exp: exp.timestamp(),
            iss: self.config.issuer.clone(),
            aud: self.config.audience.clone(),
            user_id,
            session_id,
            permissions,
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| {
                error!("Failed to create access token: {}", e);
                ApiError::Internal("Token creation failed".to_string())
            })
    }

    fn create_refresh_token(&self, user_id: Uuid) -> ApiResult<String> {
        let now = Utc::now();
        let exp = now + Duration::days(self.config.refresh_token_expiry_days);

        let claims = Claims {
            sub: user_id.to_string(),
            name: "refresh".to_string(),
            email: "refresh@token".to_string(),
            iat: now.timestamp(),
            exp: exp.timestamp(),
            iss: self.config.issuer.clone(),
            aud: self.config.audience.clone(),
            user_id,
            session_id: Uuid::new_v4(),
            permissions: vec!["refresh".to_string()],
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| {
                error!("Failed to create refresh token: {}", e);
                ApiError::Internal("Token creation failed".to_string())
            })
    }

    pub async fn logout(&self, _token: &str) -> ApiResult<()> {
        // In a real implementation, you would:
        // 1. Add token to blacklist
        // 2. Invalidate refresh token
        // 3. Clean up session data
        
        debug!("User logged out");
        Ok(())
    }

    pub fn has_permission(&self, claims: &Claims, required_permission: &str) -> bool {
        claims.permissions.contains(&required_permission.to_string()) ||
        claims.permissions.contains(&"admin".to_string())
    }
}

// Axum extractor for authenticated requests
#[derive(Debug)]
pub struct AuthenticatedUser {
    pub claims: Claims,
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Extract the authorization header
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| ApiError::Authentication("Missing authorization header".to_string()))?;

        // Get the auth service from extensions (set by middleware)
        let auth_service = parts
            .extensions
            .get::<Arc<AuthService>>()
            .ok_or_else(|| ApiError::Internal("Auth service not available".to_string()))?;

        // Verify the token
        let claims = auth_service.verify_token(bearer.token())?;

        Ok(AuthenticatedUser { claims })
    }
}

// Permission checker extractor
pub struct RequirePermission(pub String);

#[async_trait]
impl<S> FromRequestParts<S> for RequirePermission
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let user = AuthenticatedUser::from_request_parts(parts, state).await?;
        
        // Get the auth service from extensions
        let auth_service = parts
            .extensions
            .get::<Arc<AuthService>>()
            .ok_or_else(|| ApiError::Internal("Auth service not available".to_string()))?;

        // For this extractor, we'll check permissions in the handler
        // This is just a placeholder implementation
        Ok(RequirePermission("read".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_auth_service_creation() {
        let config = AuthConfig::default();
        let auth_service = AuthService::new(config);
        
        // Test token creation and verification
        let user_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let permissions = vec!["read".to_string()];
        
        let token = auth_service.create_access_token(
            user_id,
            session_id,
            "Test User".to_string(),
            "test@example.com".to_string(),
            permissions.clone(),
        ).unwrap();
        
        let claims = auth_service.verify_token(&token).unwrap();
        assert_eq!(claims.user_id, user_id);
        assert_eq!(claims.permissions, permissions);
    }

    #[tokio::test]
    async fn test_authentication() {
        let config = AuthConfig::default();
        let auth_service = AuthService::new(config);
        
        let request = LoginRequest {
            email: "demo@example.com".to_string(),
            password: "password".to_string(),
        };
        
        let response = auth_service.authenticate(request).await.unwrap();
        assert_eq!(response.token_type, "Bearer");
        assert!(!response.access_token.is_empty());
    }

    #[tokio::test]
    async fn test_invalid_credentials() {
        let config = AuthConfig::default();
        let auth_service = AuthService::new(config);
        
        let request = LoginRequest {
            email: "invalid@example.com".to_string(),
            password: "wrong".to_string(),
        };
        
        let result = auth_service.authenticate(request).await;
        assert!(result.is_err());
    }
}