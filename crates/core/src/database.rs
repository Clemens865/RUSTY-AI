use rusty_ai_common::{Result, AssistantError};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
    Pool, Sqlite, SqlitePool,
};
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;
use tracing::{info, warn, error, debug, instrument};

/// Database configuration
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    /// Database URL or file path
    pub database_url: String,
    /// Maximum number of connections in the pool
    pub max_connections: u32,
    /// Minimum number of connections in the pool
    pub min_connections: u32,
    /// Connection timeout
    pub connect_timeout: Duration,
    /// Maximum lifetime of a connection
    pub max_lifetime: Option<Duration>,
    /// Idle timeout for connections
    pub idle_timeout: Option<Duration>,
    /// Enable WAL mode for better concurrency
    pub enable_wal_mode: bool,
    /// Enable foreign key constraints
    pub enable_foreign_keys: bool,
    /// SQLite synchronous mode
    pub synchronous_mode: SqliteSynchronous,
    /// Run migrations on startup
    pub auto_migrate: bool,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            database_url: "sqlite:./data/rusty_ai.db".to_string(),
            max_connections: 20,
            min_connections: 5,
            connect_timeout: Duration::from_secs(30),
            max_lifetime: Some(Duration::from_secs(3600)), // 1 hour
            idle_timeout: Some(Duration::from_secs(600)),   // 10 minutes
            enable_wal_mode: true,
            enable_foreign_keys: true,
            synchronous_mode: SqliteSynchronous::Normal,
            auto_migrate: true,
        }
    }
}

/// Database connection pool manager
pub struct DatabaseManager {
    pool: SqlitePool,
    config: DatabaseConfig,
}

impl DatabaseManager {
    /// Create a new database manager with configuration
    #[instrument(skip(config))]
    pub async fn new(config: DatabaseConfig) -> Result<Self> {
        info!("Initializing database connection pool");
        
        // Ensure database directory exists
        if let Some(parent) = Path::new(&config.database_url.replace("sqlite:", "")).parent() {
            tokio::fs::create_dir_all(parent).await
                .map_err(|e| AssistantError::Database(format!("Failed to create database directory: {}", e)))?;
        }
        
        // Configure SQLite connection options
        let mut connect_options = SqliteConnectOptions::from_str(&config.database_url)
            .map_err(|e| AssistantError::Database(format!("Invalid database URL: {}", e)))?;
        
        // Apply configuration options
        connect_options = connect_options
            .create_if_missing(true)
            .foreign_keys(config.enable_foreign_keys)
            .synchronous(config.synchronous_mode);
        
        if config.enable_wal_mode {
            connect_options = connect_options.journal_mode(SqliteJournalMode::Wal);
        }
        
        // Create connection pool
        let mut pool_options = SqlitePoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(config.connect_timeout);
        
        if let Some(max_lifetime) = config.max_lifetime {
            pool_options = pool_options.max_lifetime(max_lifetime);
        }
        
        if let Some(idle_timeout) = config.idle_timeout {
            pool_options = pool_options.idle_timeout(idle_timeout);
        }
        
        let pool = pool_options
            .connect_with(connect_options)
            .await
            .map_err(|e| AssistantError::Database(format!("Failed to create connection pool: {}", e)))?;
        
        let manager = Self { pool, config };
        
        // Run migrations if enabled
        if manager.config.auto_migrate {
            manager.run_migrations().await?;
        }
        
        // Validate connection
        manager.health_check().await?;
        
        info!("Database connection pool initialized successfully");
        Ok(manager)
    }
    
    /// Get a reference to the connection pool
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
    
    /// Get database configuration
    pub fn config(&self) -> &DatabaseConfig {
        &self.config
    }
    
    /// Run database migrations
    #[instrument(skip(self))]
    pub async fn run_migrations(&self) -> Result<()> {
        info!("Running database migrations");
        
        sqlx::migrate!("../../migrations")
            .run(&self.pool)
            .await
            .map_err(|e| AssistantError::Database(format!("Migration failed: {}", e)))?;
        
        info!("Database migrations completed successfully");
        Ok(())
    }
    
    /// Perform health check on the database connection
    #[instrument(skip(self))]
    pub async fn health_check(&self) -> Result<DatabaseHealth> {
        debug!("Performing database health check");
        
        let start_time = std::time::Instant::now();
        
        // Test basic connectivity
        let connectivity_result = sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await;
        
        let connectivity_time = start_time.elapsed();
        
        if connectivity_result.is_err() {
            error!("Database connectivity check failed: {:?}", connectivity_result.unwrap_err());
            return Ok(DatabaseHealth {
                is_healthy: false,
                connectivity_ms: connectivity_time.as_millis() as u64,
                pool_stats: self.get_pool_stats(),
                error_message: Some("Connectivity check failed".to_string()),
            });
        }
        
        // Check pool statistics
        let pool_stats = self.get_pool_stats();
        
        // Determine health status
        let is_healthy = connectivity_time < Duration::from_millis(1000) // Response time < 1s
            && pool_stats.connections_available > 0;
        
        let health = DatabaseHealth {
            is_healthy,
            connectivity_ms: connectivity_time.as_millis() as u64,
            pool_stats,
            error_message: None,
        };
        
        if health.is_healthy {
            debug!("Database health check passed");
        } else {
            warn!("Database health check indicates degraded performance");
        }
        
        Ok(health)
    }
    
    /// Get connection pool statistics
    pub fn get_pool_stats(&self) -> PoolStats {
        PoolStats {
            connections_total: self.pool.size(),
            connections_idle: self.pool.num_idle(),
            connections_available: self.pool.size() - (self.pool.size() - self.pool.num_idle()),
        }
    }
    
    /// Optimize database performance
    #[instrument(skip(self))]
    pub async fn optimize(&self) -> Result<()> {
        info!("Running database optimization");
        
        // Analyze query performance
        sqlx::query("ANALYZE")
            .execute(&self.pool)
            .await
            .map_err(|e| AssistantError::Database(format!("ANALYZE failed: {}", e)))?;
        
        // Vacuum database to reclaim space
        sqlx::query("VACUUM")
            .execute(&self.pool)
            .await
            .map_err(|e| AssistantError::Database(format!("VACUUM failed: {}", e)))?;
        
        info!("Database optimization completed");
        Ok(())
    }
    
    /// Create a backup of the database
    #[instrument(skip(self))]
    pub async fn backup(&self, backup_path: &Path) -> Result<()> {
        info!("Creating database backup to: {:?}", backup_path);
        
        // Ensure backup directory exists
        if let Some(parent) = backup_path.parent() {
            tokio::fs::create_dir_all(parent).await
                .map_err(|e| AssistantError::Database(format!("Failed to create backup directory: {}", e)))?;
        }
        
        // For SQLite, we can use the backup API or simply copy the file
        // Here we'll use a SQL-based approach for simplicity
        let backup_sql = format!("VACUUM INTO '{}'", backup_path.display());
        
        sqlx::query(&backup_sql)
            .execute(&self.pool)
            .await
            .map_err(|e| AssistantError::Database(format!("Backup failed: {}", e)))?;
        
        info!("Database backup completed successfully");
        Ok(())
    }
    
    /// Get database size information
    pub async fn get_size_info(&self) -> Result<DatabaseSizeInfo> {
        let result = sqlx::query!(
            r#"
            SELECT 
                page_count * page_size as database_size,
                freelist_count * page_size as free_space
            FROM pragma_page_count(), pragma_page_size(), pragma_freelist_count()
            "#
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AssistantError::Database(format!("Failed to get size info: {}", e)))?;
        
        Ok(DatabaseSizeInfo {
            total_size_bytes: result.database_size.unwrap_or(0) as u64,
            free_space_bytes: result.free_space.unwrap_or(0) as u64,
        })
    }
    
    /// Close the database connection pool
    pub async fn close(&self) {
        info!("Closing database connection pool");
        self.pool.close().await;
    }
    
    /// Execute a transaction with retry logic
    pub async fn execute_transaction<F, R>(&self, transaction_fn: F) -> Result<R>
    where
        F: Fn(&mut sqlx::Transaction<Sqlite>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<R>> + Send + '_>> + Send + Sync,
        R: Send,
    {
        const MAX_RETRIES: u32 = 3;
        let mut retry_count = 0;
        
        loop {
            let mut tx = self.pool.begin().await
                .map_err(|e| AssistantError::Database(format!("Failed to begin transaction: {}", e)))?;
            
            match transaction_fn(&mut tx).await {
                Ok(result) => {
                    tx.commit().await
                        .map_err(|e| AssistantError::Database(format!("Failed to commit transaction: {}", e)))?;
                    return Ok(result);
                }
                Err(e) => {
                    let _ = tx.rollback().await;
                    
                    retry_count += 1;
                    if retry_count >= MAX_RETRIES {
                        return Err(e);
                    }
                    
                    // Wait before retry (exponential backoff)
                    let delay = Duration::from_millis(100 * 2_u64.pow(retry_count - 1));
                    tokio::time::sleep(delay).await;
                    
                    warn!("Transaction failed, retrying ({}/{}): {}", retry_count, MAX_RETRIES, e);
                }
            }
        }
    }
}

/// Database health information
#[derive(Debug, Clone)]
pub struct DatabaseHealth {
    pub is_healthy: bool,
    pub connectivity_ms: u64,
    pub pool_stats: PoolStats,
    pub error_message: Option<String>,
}

/// Connection pool statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub connections_total: u32,
    pub connections_idle: u32,
    pub connections_available: u32,
}

/// Database size information
#[derive(Debug, Clone)]
pub struct DatabaseSizeInfo {
    pub total_size_bytes: u64,
    pub free_space_bytes: u64,
}

impl DatabaseSizeInfo {
    pub fn used_space_bytes(&self) -> u64 {
        self.total_size_bytes.saturating_sub(self.free_space_bytes)
    }
    
    pub fn usage_percentage(&self) -> f64 {
        if self.total_size_bytes == 0 {
            0.0
        } else {
            (self.used_space_bytes() as f64 / self.total_size_bytes as f64) * 100.0
        }
    }
}

/// Database utility functions
pub struct DatabaseUtils;

impl DatabaseUtils {
    /// Generate a new UUID for database records
    pub fn generate_id() -> String {
        uuid::Uuid::new_v4().to_string()
    }
    
    /// Convert JSON string to serde_json::Value
    pub fn parse_json(json_str: &str) -> Result<serde_json::Value> {
        serde_json::from_str(json_str)
            .map_err(|e| AssistantError::Database(format!("Invalid JSON: {}", e)))
    }
    
    /// Convert serde_json::Value to JSON string
    pub fn stringify_json(value: &serde_json::Value) -> Result<String> {
        serde_json::to_string(value)
            .map_err(|e| AssistantError::Database(format!("JSON serialization failed: {}", e)))
    }
    
    /// Escape SQL LIKE pattern
    pub fn escape_like_pattern(pattern: &str) -> String {
        pattern
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_")
    }
    
    /// Build full-text search query
    pub fn build_fts_query(terms: &[String]) -> String {
        if terms.is_empty() {
            return "*".to_string();
        }
        
        terms
            .iter()
            .map(|term| format!("\"{}\"", term.replace('"', "\"\"")))
            .collect::<Vec<_>>()
            .join(" AND ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    async fn create_test_database() -> Result<DatabaseManager> {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        
        let config = DatabaseConfig {
            database_url: format!("sqlite:{}", db_path.display()),
            max_connections: 5,
            min_connections: 1,
            auto_migrate: false, // Don't auto-migrate in tests
            ..DatabaseConfig::default()
        };
        
        DatabaseManager::new(config).await
    }
    
    #[tokio::test]
    async fn test_database_manager_creation() {
        let db = create_test_database().await.unwrap();
        assert!(db.pool().is_closed() == false);
    }
    
    #[tokio::test]
    async fn test_health_check() {
        let db = create_test_database().await.unwrap();
        let health = db.health_check().await.unwrap();
        assert!(health.is_healthy);
        assert!(health.connectivity_ms < 1000);
    }
    
    #[tokio::test]
    async fn test_pool_stats() {
        let db = create_test_database().await.unwrap();
        let stats = db.get_pool_stats();
        assert!(stats.connections_total > 0);
    }
    
    #[test]
    fn test_database_utils() {
        let id = DatabaseUtils::generate_id();
        assert!(!id.is_empty());
        
        let json_value = serde_json::json!({"test": "value"});
        let json_string = DatabaseUtils::stringify_json(&json_value).unwrap();
        let parsed_value = DatabaseUtils::parse_json(&json_string).unwrap();
        assert_eq!(json_value, parsed_value);
        
        let pattern = DatabaseUtils::escape_like_pattern("test%_string");
        assert_eq!(pattern, "test\\%\\_string");
        
        let fts_query = DatabaseUtils::build_fts_query(&[
            "hello".to_string(),
            "world".to_string(),
        ]);
        assert_eq!(fts_query, "\"hello\" AND \"world\"");
    }
    
    #[tokio::test]
    async fn test_size_info() {
        let db = create_test_database().await.unwrap();
        let size_info = db.get_size_info().await.unwrap();
        assert!(size_info.total_size_bytes >= 0);
    }
}