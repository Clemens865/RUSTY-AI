use crate::{
    auth::AuthService,
    middleware::{
        auth_middleware, compression_layer, cors_layer, error_handling_middleware,
        rate_limiting_middleware, request_id_middleware, request_logging_middleware,
        request_size_middleware, security_headers_middleware, timeout_layer, RateLimiter,
    },
    routes::{create_routes, not_found_handler},
    websocket::{websocket_handler, WebSocketManager},
    ApiConfig,
};
use axum::{
    routing::get,
    Router,
};
use rusty_ai_core::AssistantCore;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::signal;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing::{error, info};

pub struct ApiServer {
    config: ApiConfig,
    core: Arc<AssistantCore>,
    auth_service: Arc<AuthService>,
    rate_limiter: Arc<RateLimiter>,
    websocket_manager: Arc<WebSocketManager>,
}

impl ApiServer {
    pub fn new(
        config: ApiConfig,
        core: Arc<AssistantCore>,
        auth_service: Arc<AuthService>,
    ) -> Self {
        let rate_limiter = Arc::new(RateLimiter::new(
            config.rate_limit_requests_per_minute,
            Duration::from_secs(60),
        ));
        
        let websocket_manager = Arc::new(WebSocketManager::new(core.clone()));

        Self {
            config,
            core,
            auth_service,
            rate_limiter,
            websocket_manager,
        }
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let app = self.create_app().await;
        let addr = SocketAddr::from(([0, 0, 0, 0], self.config.port));

        info!("Starting API server on {}", addr);
        info!("CORS origins: {:?}", self.config.cors_origins);
        info!("WebSocket support: {}", self.config.enable_websockets);

        // Start background tasks
        self.start_background_tasks().await;

        let listener = tokio::net::TcpListener::bind(addr).await?;
        
        info!("API server listening on {}", addr);

        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await?;

        info!("API server stopped");
        Ok(())
    }

    async fn create_app(&self) -> Router {
        let mut app = Router::new()
            // WebSocket endpoint (if enabled)
            .route("/ws", get(websocket_handler))
            // Main API routes
            .merge(create_routes(self.core.clone(), self.auth_service.clone()))
            // Fallback for unmatched routes
            .fallback(not_found_handler);

        // Add WebSocket state if enabled
        if self.config.enable_websockets {
            app = app.with_state(self.websocket_manager.clone());
        }

        // Add middleware stack
        app.layer(
            ServiceBuilder::new()
                // Outermost layers (applied last)
                .layer(TraceLayer::new_for_http())
                .layer(timeout_layer())
                .layer(compression_layer())
                .layer(cors_layer(&self.config))
                
                // Security and validation layers
                .layer(axum::middleware::from_fn(security_headers_middleware))
                .layer(axum::middleware::from_fn_with_state(
                    self.config.max_request_size,
                    request_size_middleware,
                ))
                .layer(axum::middleware::from_fn_with_state(
                    self.rate_limiter.clone(),
                    rate_limiting_middleware,
                ))
                
                // Logging and request tracking
                .layer(axum::middleware::from_fn(request_id_middleware))
                .layer(axum::middleware::from_fn(request_logging_middleware))
                .layer(axum::middleware::from_fn(error_handling_middleware))
                
                // Authentication layer
                .layer(axum::middleware::from_fn_with_state(
                    self.auth_service.clone(),
                    auth_middleware,
                ))
        )
    }

    async fn start_background_tasks(&self) {
        let rate_limiter = self.rate_limiter.clone();
        
        // Rate limiter cleanup task
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5 minutes
            loop {
                interval.tick().await;
                rate_limiter.cleanup_old_entries();
            }
        });

        // Session cleanup task
        let context_manager = self.core.context_manager.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(3600)); // 1 hour
            loop {
                interval.tick().await;
                if let Ok(mut manager) = context_manager.try_write() {
                    if let Err(e) = manager.cleanup_expired_sessions().await {
                        error!("Error cleaning up expired sessions: {}", e);
                    }
                }
            }
        });

        // Task execution background service
        let orchestrator = self.core.orchestrator.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60)); // 1 minute
            loop {
                interval.tick().await;
                if let Err(e) = orchestrator.execute_pending_tasks().await {
                    error!("Error executing pending tasks: {}", e);
                }
            }
        });

        info!("Background tasks started");
    }

    pub async fn health_check(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Check core services
        let storage_health = self.core.storage.health_check().await?;
        
        match storage_health.status {
            rusty_ai_core::storage::StorageStatus::Healthy => {
                info!("Storage health check passed");
            }
            _ => {
                error!("Storage health check failed: {:?}", storage_health);
                return Err("Storage health check failed".into());
            }
        }

        // Check WebSocket connections if enabled
        if self.config.enable_websockets {
            let active_connections = self.websocket_manager.get_active_connections().await;
            info!("Active WebSocket connections: {}", active_connections);
        }

        info!("API server health check passed");
        Ok(())
    }

    pub fn get_config(&self) -> &ApiConfig {
        &self.config
    }

    pub async fn get_metrics(&self) -> serde_json::Value {
        let storage_health = self.core.storage.health_check().await.unwrap_or_else(|_| {
            rusty_ai_core::storage::StorageHealth {
                status: rusty_ai_core::storage::StorageStatus::Unhealthy,
                connection_pool_size: None,
                pending_migrations: None,
                disk_usage_mb: None,
                last_backup: None,
            }
        });

        serde_json::json!({
            "timestamp": chrono::Utc::now(),
            "api": {
                "active_websocket_connections": self.websocket_manager.get_active_connections().await,
                "rate_limit_enabled": true,
                "websockets_enabled": self.config.enable_websockets
            },
            "storage": {
                "status": format!("{:?}", storage_health.status),
                "connection_pool_size": storage_health.connection_pool_size.unwrap_or(0)
            },
            "sessions": {
                "active_count": self.core.context_manager.read().await.get_active_session_count().await
            }
        })
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C, shutting down...");
        },
        _ = terminate => {
            info!("Received SIGTERM, shutting down...");
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::AuthConfig;
    use rusty_ai_core::CoreConfig;

    async fn create_test_server() -> ApiServer {
        let core_config = CoreConfig::default();
        let core = Arc::new(AssistantCore::new(core_config).await.unwrap());
        
        let auth_config = AuthConfig::default();
        let auth_service = Arc::new(AuthService::new(auth_config));
        
        let api_config = ApiConfig::default();
        
        ApiServer::new(api_config, core, auth_service)
    }

    #[tokio::test]
    async fn test_server_creation() {
        let server = create_test_server().await;
        assert_eq!(server.config.port, 8080);
        assert!(server.config.enable_websockets);
    }

    #[tokio::test]
    async fn test_health_check() {
        let server = create_test_server().await;
        
        // Health check should pass for a properly configured server
        let result = server.health_check().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_metrics_collection() {
        let server = create_test_server().await;
        
        let metrics = server.get_metrics().await;
        assert!(metrics.is_object());
        assert!(metrics["timestamp"].is_string());
        assert!(metrics["api"].is_object());
        assert!(metrics["storage"].is_object());
    }
}