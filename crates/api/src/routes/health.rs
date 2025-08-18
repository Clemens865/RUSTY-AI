use crate::{create_success_response, HealthCheck, ServiceHealth};
use axum::{routing::get, Json, Router};
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::debug;

pub fn routes() -> Router {
    Router::new()
        .route("/", get(health_check))
        .route("/ready", get(readiness_check))
        .route("/live", get(liveness_check))
        .route("/metrics", get(metrics))
}

// Basic health check endpoint
async fn health_check() -> Json<serde_json::Value> {
    debug!("Health check requested");
    
    let uptime = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let health = HealthCheck {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime,
        services: ServiceHealth {
            database: check_database_health().await,
            storage: check_storage_health().await,
            plugins: check_plugins_health().await,
            voice: check_voice_health().await,
        },
    };

    create_success_response(health).0
}

// Kubernetes readiness probe
async fn readiness_check() -> Json<serde_json::Value> {
    debug!("Readiness check requested");
    
    // Check if all critical services are ready
    let database_ready = check_database_health().await == "healthy";
    let storage_ready = check_storage_health().await == "healthy";
    
    if database_ready && storage_ready {
        Json(json!({
            "status": "ready",
            "timestamp": chrono::Utc::now(),
            "checks": {
                "database": "ready",
                "storage": "ready"
            }
        }))
    } else {
        Json(json!({
            "status": "not_ready",
            "timestamp": chrono::Utc::now(),
            "checks": {
                "database": if database_ready { "ready" } else { "not_ready" },
                "storage": if storage_ready { "ready" } else { "not_ready" }
            }
        }))
    }
}

// Kubernetes liveness probe
async fn liveness_check() -> Json<serde_json::Value> {
    debug!("Liveness check requested");
    
    Json(json!({
        "status": "alive",
        "timestamp": chrono::Utc::now(),
        "uptime_seconds": SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }))
}

// Basic metrics endpoint
async fn metrics() -> Json<serde_json::Value> {
    debug!("Metrics requested");
    
    // In a production system, you would collect real metrics
    Json(json!({
        "timestamp": chrono::Utc::now(),
        "system": {
            "memory_usage_mb": get_memory_usage(),
            "cpu_usage_percent": get_cpu_usage(),
            "uptime_seconds": SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
        },
        "application": {
            "active_sessions": 0, // Would be populated from session manager
            "total_requests": 0,  // Would be populated from metrics collector
            "error_rate": 0.0,    // Would be calculated from error metrics
            "avg_response_time_ms": 0.0
        },
        "services": {
            "database_connections": 0,
            "plugin_count": 0,
            "voice_pipeline_status": "inactive"
        }
    }))
}

// Helper functions for health checks
async fn check_database_health() -> String {
    // In a real implementation, this would:
    // 1. Try to connect to the database
    // 2. Execute a simple query
    // 3. Check connection pool status
    
    // For now, assume healthy
    "healthy".to_string()
}

async fn check_storage_health() -> String {
    // In a real implementation, this would:
    // 1. Check file system availability
    // 2. Verify write permissions
    // 3. Check disk space
    
    "healthy".to_string()
}

async fn check_plugins_health() -> String {
    // In a real implementation, this would:
    // 1. Check plugin manager status
    // 2. Verify active plugins are responding
    // 3. Check plugin resource usage
    
    "healthy".to_string()
}

async fn check_voice_health() -> String {
    // In a real implementation, this would:
    // 1. Check voice pipeline status
    // 2. Verify external API connections (Whisper, ElevenLabs)
    // 3. Check audio processing capabilities
    
    "inactive".to_string()
}

fn get_memory_usage() -> u64 {
    // In a real implementation, use a proper system metrics library
    // For now, return a placeholder value
    0
}

fn get_cpu_usage() -> f64 {
    // In a real implementation, use a proper system metrics library
    // For now, return a placeholder value
    0.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_health_check() {
        let app = routes();
        
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_readiness_check() {
        let app = routes();
        
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/ready")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_liveness_check() {
        let app = routes();
        
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/live")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}