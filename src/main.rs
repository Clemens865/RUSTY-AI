use anyhow::Result;
use tracing::{info, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rusty_ai=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting Personal AI Assistant API...");
    
    // Load environment variables
    dotenv::dotenv().ok();
    
    // Start the server
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port);
    
    info!("Server running on http://{}", addr);
    
    // For now, just keep the server running
    tokio::signal::ctrl_c().await?;
    info!("Shutting down...");
    
    Ok(())
}