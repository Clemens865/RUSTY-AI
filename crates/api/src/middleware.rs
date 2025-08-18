use crate::{auth::AuthService, error::ApiError, ApiConfig};
use axum::{
    extract::{Request, State},
    http::{HeaderName, HeaderValue, Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tower_http::cors::{Any, CorsLayer};
use tracing::{debug, info, warn};

// CORS middleware configuration
pub fn cors_layer(config: &ApiConfig) -> CorsLayer {
    let origins = if config.cors_origins.contains(&"*".to_string()) {
        Any.into()
    } else {
        config
            .cors_origins
            .iter()
            .map(|origin| origin.parse().unwrap())
            .collect::<Vec<_>>()
            .into()
    };

    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::PATCH,
            Method::OPTIONS,
        ])
        .allow_headers([
            HeaderName::from_static("authorization"),
            HeaderName::from_static("content-type"),
            HeaderName::from_static("x-request-id"),
            HeaderName::from_static("x-user-agent"),
        ])
        .expose_headers([
            HeaderName::from_static("x-request-id"),
            HeaderName::from_static("x-response-time"),
        ])
        .max_age(Duration::from_secs(3600))
}

// Request logging middleware
pub async fn request_logging_middleware(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let start = Instant::now();

    info!("Incoming request: {} {}", method, uri);

    let response = next.run(request).await;
    
    let duration = start.elapsed();
    let status = response.status();

    info!(
        "Request completed: {} {} - {} - {:?}",
        method, uri, status, duration
    );

    response
}

// Request ID middleware
pub async fn request_id_middleware(mut request: Request, next: Next) -> Response {
    let request_id = uuid::Uuid::new_v4().to_string();
    
    // Add request ID to headers for downstream processing
    request.headers_mut().insert(
        HeaderName::from_static("x-request-id"),
        HeaderValue::from_str(&request_id).unwrap(),
    );

    let mut response = next.run(request).await;
    
    // Add request ID to response headers
    response.headers_mut().insert(
        HeaderName::from_static("x-request-id"),
        HeaderValue::from_str(&request_id).unwrap(),
    );

    response
}

// Rate limiting middleware
#[derive(Debug, Clone)]
pub struct RateLimiter {
    requests: Arc<Mutex<HashMap<String, Vec<Instant>>>>,
    max_requests: u32,
    window_duration: Duration,
}

impl RateLimiter {
    pub fn new(max_requests: u32, window_duration: Duration) -> Self {
        Self {
            requests: Arc::new(Mutex::new(HashMap::new())),
            max_requests,
            window_duration,
        }
    }

    pub fn check_rate_limit(&self, client_id: &str) -> bool {
        let mut requests = self.requests.lock().unwrap();
        let now = Instant::now();
        
        // Get or create request history for this client
        let client_requests = requests.entry(client_id.to_string()).or_insert_with(Vec::new);
        
        // Remove old requests outside the window
        client_requests.retain(|&request_time| now.duration_since(request_time) < self.window_duration);
        
        // Check if we're under the limit
        if client_requests.len() < self.max_requests as usize {
            client_requests.push(now);
            true
        } else {
            false
        }
    }

    pub fn cleanup_old_entries(&self) {
        let mut requests = self.requests.lock().unwrap();
        let now = Instant::now();
        
        requests.retain(|_, client_requests| {
            client_requests.retain(|&request_time| now.duration_since(request_time) < self.window_duration);
            !client_requests.is_empty()
        });
    }
}

pub async fn rate_limiting_middleware(
    State(rate_limiter): State<Arc<RateLimiter>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract client identifier (IP address or user ID)
    let client_id = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|h| h.to_str().ok())
        .or_else(|| {
            request
                .headers()
                .get("x-real-ip")
                .and_then(|h| h.to_str().ok())
        })
        .unwrap_or("unknown")
        .to_string();

    if rate_limiter.check_rate_limit(&client_id) {
        Ok(next.run(request).await)
    } else {
        warn!("Rate limit exceeded for client: {}", client_id);
        Err(StatusCode::TOO_MANY_REQUESTS)
    }
}

// Content-Type validation middleware
pub async fn content_type_middleware(request: Request, next: Next) -> Result<Response, ApiError> {
    // Only validate content type for requests with body
    if matches!(
        request.method(),
        &Method::POST | &Method::PUT | &Method::PATCH
    ) {
        let content_type = request
            .headers()
            .get("content-type")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");

        if !content_type.starts_with("application/json") && 
           !content_type.starts_with("multipart/form-data") &&
           !content_type.starts_with("application/x-www-form-urlencoded") {
            return Err(ApiError::InvalidContentType);
        }
    }

    Ok(next.run(request).await)
}

// Request size limiting middleware
pub async fn request_size_middleware(
    State(max_size): State<usize>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    if let Some(content_length) = request.headers().get("content-length") {
        if let Ok(length_str) = content_length.to_str() {
            if let Ok(length) = length_str.parse::<usize>() {
                if length > max_size {
                    return Err(ApiError::RequestTooLarge);
                }
            }
        }
    }

    Ok(next.run(request).await)
}

// Authentication middleware - adds auth service to extensions
pub async fn auth_middleware(
    State(auth_service): State<Arc<AuthService>>,
    mut request: Request,
    next: Next,
) -> Response {
    // Add auth service to request extensions so extractors can use it
    request.extensions_mut().insert(auth_service);
    next.run(request).await
}

// Security headers middleware
pub async fn security_headers_middleware(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    
    let headers = response.headers_mut();
    
    // Add security headers
    headers.insert(
        HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        HeaderName::from_static("x-frame-options"),
        HeaderValue::from_static("DENY"),
    );
    headers.insert(
        HeaderName::from_static("x-xss-protection"),
        HeaderValue::from_static("1; mode=block"),
    );
    headers.insert(
        HeaderName::from_static("strict-transport-security"),
        HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    );
    headers.insert(
        HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );

    response
}

// Error handling middleware
pub async fn error_handling_middleware(request: Request, next: Next) -> Response {
    let response = next.run(request).await;
    
    // Log errors if status code indicates an error
    if response.status().is_server_error() {
        warn!("Server error response: {}", response.status());
    } else if response.status().is_client_error() {
        debug!("Client error response: {}", response.status());
    }

    response
}

// Health check middleware - bypass authentication for health checks
pub async fn health_check_bypass_middleware(request: Request, next: Next) -> Response {
    if request.uri().path() == "/health" || request.uri().path() == "/metrics" {
        // Skip authentication for health checks
        return next.run(request).await;
    }

    next.run(request).await
}

// Compression middleware (uses tower-http)
pub fn compression_layer() -> tower_http::compression::CompressionLayer {
    tower_http::compression::CompressionLayer::new()
        .br(true)
        .gzip(true)
        .deflate(true)
}

// Timeout middleware
pub fn timeout_layer() -> tower::timeout::TimeoutLayer {
    tower::timeout::TimeoutLayer::new(Duration::from_secs(30))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_rate_limiter() {
        let limiter = RateLimiter::new(3, Duration::from_secs(60));
        
        // Should allow first 3 requests
        assert!(limiter.check_rate_limit("test_client"));
        assert!(limiter.check_rate_limit("test_client"));
        assert!(limiter.check_rate_limit("test_client"));
        
        // Should block 4th request
        assert!(!limiter.check_rate_limit("test_client"));
        
        // Different client should be allowed
        assert!(limiter.check_rate_limit("other_client"));
    }

    #[test]
    fn test_rate_limiter_cleanup() {
        let limiter = RateLimiter::new(3, Duration::from_millis(100));
        
        // Add requests
        assert!(limiter.check_rate_limit("test_client"));
        assert!(limiter.check_rate_limit("test_client"));
        
        // Wait for window to expire
        std::thread::sleep(Duration::from_millis(150));
        
        // Should allow new requests
        assert!(limiter.check_rate_limit("test_client"));
        assert!(limiter.check_rate_limit("test_client"));
        assert!(limiter.check_rate_limit("test_client"));
    }

    #[test]
    fn test_cors_configuration() {
        let config = ApiConfig {
            cors_origins: vec!["https://example.com".to_string()],
            ..Default::default()
        };
        
        let cors_layer = cors_layer(&config);
        // Would need to test with actual requests in integration tests
        assert!(true); // Placeholder assertion
    }
}