use rusty_ai_common::{Result, AssistantError, Document, Task, TaskStatus, DailyBriefing};
use async_trait::async_trait;
use sqlx::{SqlitePool, Postgres, Pool, migrate::MigrateDatabase, Sqlite, Row};
use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use tracing::{info, error, debug};
use serde_json;

#[async_trait]
pub trait Storage: Send + Sync {
    // Document operations
    async fn store_document(&self, document: &Document) -> Result<()>;
    async fn get_document(&self, id: Uuid) -> Result<Option<Document>>;
    async fn update_document(&self, document: &Document) -> Result<()>;
    async fn delete_document(&self, id: Uuid) -> Result<()>;
    async fn search_documents(&self, query: &str, limit: usize) -> Result<Vec<Document>>;
    async fn get_documents_by_tags(&self, tags: &[String], limit: usize) -> Result<Vec<Document>>;

    // Task operations
    async fn store_task(&self, task: &Task) -> Result<()>;
    async fn get_task(&self, id: Uuid) -> Result<Option<Task>>;
    async fn update_task_status(&self, id: Uuid, status: TaskStatus) -> Result<()>;
    async fn get_pending_tasks(&self) -> Result<Vec<Task>>;
    async fn get_tasks_by_status(&self, status: TaskStatus) -> Result<Vec<Task>>;

    // Daily briefing operations
    async fn store_briefing(&self, briefing: &DailyBriefing) -> Result<()>;
    async fn get_briefing(&self, id: Uuid) -> Result<Option<DailyBriefing>>;
    async fn get_latest_briefing(&self) -> Result<Option<DailyBriefing>>;
    async fn get_briefings_by_date_range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<DailyBriefing>>;

    // Maintenance operations
    async fn cleanup_old_data(&self, retention_days: i64) -> Result<usize>;
    async fn health_check(&self) -> Result<StorageHealth>;
}

#[derive(Debug, Clone)]
pub struct StorageHealth {
    pub status: StorageStatus,
    pub connection_pool_size: Option<usize>,
    pub pending_migrations: Option<usize>,
    pub disk_usage_mb: Option<f64>,
    pub last_backup: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StorageStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub database_url: String,
    pub max_connections: u32,
    pub connection_timeout_secs: u64,
    pub enable_wal_mode: bool,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            database_url: "sqlite:./data/assistant.db".to_string(),
            max_connections: 10,
            connection_timeout_secs: 30,
            enable_wal_mode: true,
        }
    }
}

pub struct SqliteStorage {
    pool: SqlitePool,
}

impl SqliteStorage {
    pub async fn new(config: &StorageConfig) -> Result<Self> {
        // Create database if it doesn't exist
        if !Sqlite::database_exists(&config.database_url).await.unwrap_or(false) {
            info!("Creating database at {}", config.database_url);
            Sqlite::create_database(&config.database_url)
                .await
                .map_err(|e| AssistantError::Database(format!("Failed to create database: {}", e)))?;
        }

        // Create connection pool
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(config.max_connections)
            .connect_timeout(std::time::Duration::from_secs(config.connection_timeout_secs))
            .connect(&config.database_url)
            .await
            .map_err(|e| AssistantError::Database(format!("Failed to connect to database: {}", e)))?;

        // Run migrations
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(|e| AssistantError::Database(format!("Failed to run migrations: {}", e)))?;

        // Enable WAL mode for better performance
        if config.enable_wal_mode {
            sqlx::query("PRAGMA journal_mode = WAL")
                .execute(&pool)
                .await
                .map_err(|e| AssistantError::Database(format!("Failed to enable WAL mode: {}", e)))?;
        }

        info!("SQLite storage initialized successfully");
        Ok(Self { pool })
    }
}

#[async_trait]
impl Storage for SqliteStorage {
    async fn store_document(&self, document: &Document) -> Result<()> {
        let metadata_json = serde_json::to_string(&document.metadata)
            .map_err(|e| AssistantError::Internal(format!("Failed to serialize metadata: {}", e)))?;

        sqlx::query!(
            r#"
            INSERT INTO documents (id, title, content, metadata, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
            document.id.to_string(),
            document.title,
            document.content,
            metadata_json,
            document.created_at,
            document.updated_at
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AssistantError::Database(format!("Failed to store document: {}", e)))?;

        debug!("Stored document: {}", document.id);
        Ok(())
    }

    async fn get_document(&self, id: Uuid) -> Result<Option<Document>> {
        let row = sqlx::query!(
            "SELECT * FROM documents WHERE id = ?",
            id.to_string()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AssistantError::Database(format!("Failed to get document: {}", e)))?;

        match row {
            Some(row) => {
                let metadata = serde_json::from_str(&row.metadata)
                    .map_err(|e| AssistantError::Internal(format!("Failed to deserialize metadata: {}", e)))?;

                Ok(Some(Document {
                    id: Uuid::parse_str(&row.id)
                        .map_err(|e| AssistantError::Internal(format!("Invalid UUID: {}", e)))?,
                    title: row.title,
                    content: row.content,
                    metadata,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                }))
            }
            None => Ok(None),
        }
    }

    async fn update_document(&self, document: &Document) -> Result<()> {
        let metadata_json = serde_json::to_string(&document.metadata)
            .map_err(|e| AssistantError::Internal(format!("Failed to serialize metadata: {}", e)))?;

        let result = sqlx::query!(
            r#"
            UPDATE documents 
            SET title = ?, content = ?, metadata = ?, updated_at = ?
            WHERE id = ?
            "#,
            document.title,
            document.content,
            metadata_json,
            document.updated_at,
            document.id.to_string()
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AssistantError::Database(format!("Failed to update document: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(AssistantError::NotFound(format!("Document not found: {}", document.id)));
        }

        debug!("Updated document: {}", document.id);
        Ok(())
    }

    async fn delete_document(&self, id: Uuid) -> Result<()> {
        let result = sqlx::query!(
            "DELETE FROM documents WHERE id = ?",
            id.to_string()
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AssistantError::Database(format!("Failed to delete document: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(AssistantError::NotFound(format!("Document not found: {}", id)));
        }

        debug!("Deleted document: {}", id);
        Ok(())
    }

    async fn search_documents(&self, query: &str, limit: usize) -> Result<Vec<Document>> {
        let rows = sqlx::query!(
            r#"
            SELECT * FROM documents 
            WHERE title LIKE ? OR content LIKE ?
            ORDER BY updated_at DESC
            LIMIT ?
            "#,
            format!("%{}%", query),
            format!("%{}%", query),
            limit as i32
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AssistantError::Database(format!("Failed to search documents: {}", e)))?;

        let mut documents = Vec::new();
        for row in rows {
            let metadata = serde_json::from_str(&row.metadata)
                .map_err(|e| AssistantError::Internal(format!("Failed to deserialize metadata: {}", e)))?;

            documents.push(Document {
                id: Uuid::parse_str(&row.id)
                    .map_err(|e| AssistantError::Internal(format!("Invalid UUID: {}", e)))?,
                title: row.title,
                content: row.content,
                metadata,
                created_at: row.created_at,
                updated_at: row.updated_at,
            });
        }

        Ok(documents)
    }

    async fn get_documents_by_tags(&self, tags: &[String], limit: usize) -> Result<Vec<Document>> {
        let tags_json: Vec<String> = tags.iter().map(|tag| format!("%\"{}\"", tag)).collect();
        let tag_conditions = tags_json.iter()
            .map(|_| "metadata LIKE ?")
            .collect::<Vec<_>>()
            .join(" OR ");

        let query_str = format!(
            "SELECT * FROM documents WHERE {} ORDER BY updated_at DESC LIMIT ?",
            tag_conditions
        );

        let mut query = sqlx::query(&query_str);
        for tag_condition in &tags_json {
            query = query.bind(tag_condition);
        }
        query = query.bind(limit as i32);

        let rows = query
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AssistantError::Database(format!("Failed to get documents by tags: {}", e)))?;

        let mut documents = Vec::new();
        for row in rows {
            let id_str: String = row.get("id");
            let title: String = row.get("title");
            let content: String = row.get("content");
            let metadata_str: String = row.get("metadata");
            let created_at: DateTime<Utc> = row.get("created_at");
            let updated_at: DateTime<Utc> = row.get("updated_at");

            let metadata = serde_json::from_str(&metadata_str)
                .map_err(|e| AssistantError::Internal(format!("Failed to deserialize metadata: {}", e)))?;

            documents.push(Document {
                id: Uuid::parse_str(&id_str)
                    .map_err(|e| AssistantError::Internal(format!("Invalid UUID: {}", e)))?,
                title,
                content,
                metadata,
                created_at,
                updated_at,
            });
        }

        Ok(documents)
    }

    async fn store_task(&self, task: &Task) -> Result<()> {
        let tags_json = serde_json::to_string(&task.tags)
            .map_err(|e| AssistantError::Internal(format!("Failed to serialize tags: {}", e)))?;

        sqlx::query!(
            r#"
            INSERT INTO tasks (id, name, description, status, priority, due_date, tags, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            task.id.to_string(),
            task.name,
            task.description,
            task.status.to_string(),
            task.priority.to_string(),
            task.due_date,
            tags_json,
            task.created_at,
            task.updated_at
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AssistantError::Database(format!("Failed to store task: {}", e)))?;

        debug!("Stored task: {}", task.id);
        Ok(())
    }

    async fn get_task(&self, id: Uuid) -> Result<Option<Task>> {
        let row = sqlx::query!(
            "SELECT * FROM tasks WHERE id = ?",
            id.to_string()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AssistantError::Database(format!("Failed to get task: {}", e)))?;

        match row {
            Some(row) => {
                let tags: Vec<String> = serde_json::from_str(&row.tags)
                    .map_err(|e| AssistantError::Internal(format!("Failed to deserialize tags: {}", e)))?;

                Ok(Some(Task {
                    id: Uuid::parse_str(&row.id)
                        .map_err(|e| AssistantError::Internal(format!("Invalid UUID: {}", e)))?,
                    name: row.name,
                    description: row.description,
                    status: row.status.parse()
                        .map_err(|e| AssistantError::Internal(format!("Invalid status: {}", e)))?,
                    priority: row.priority.parse()
                        .map_err(|e| AssistantError::Internal(format!("Invalid priority: {}", e)))?,
                    due_date: row.due_date,
                    tags,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                }))
            }
            None => Ok(None),
        }
    }

    async fn update_task_status(&self, id: Uuid, status: TaskStatus) -> Result<()> {
        let result = sqlx::query!(
            "UPDATE tasks SET status = ?, updated_at = ? WHERE id = ?",
            status.to_string(),
            Utc::now(),
            id.to_string()
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AssistantError::Database(format!("Failed to update task status: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(AssistantError::NotFound(format!("Task not found: {}", id)));
        }

        debug!("Updated task status: {} -> {:?}", id, status);
        Ok(())
    }

    async fn get_pending_tasks(&self) -> Result<Vec<Task>> {
        self.get_tasks_by_status(TaskStatus::Pending).await
    }

    async fn get_tasks_by_status(&self, status: TaskStatus) -> Result<Vec<Task>> {
        let rows = sqlx::query!(
            "SELECT * FROM tasks WHERE status = ? ORDER BY created_at ASC",
            status.to_string()
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AssistantError::Database(format!("Failed to get tasks by status: {}", e)))?;

        let mut tasks = Vec::new();
        for row in rows {
            let tags: Vec<String> = serde_json::from_str(&row.tags)
                .map_err(|e| AssistantError::Internal(format!("Failed to deserialize tags: {}", e)))?;

            tasks.push(Task {
                id: Uuid::parse_str(&row.id)
                    .map_err(|e| AssistantError::Internal(format!("Invalid UUID: {}", e)))?,
                name: row.name,
                description: row.description,
                status: row.status.parse()
                    .map_err(|e| AssistantError::Internal(format!("Invalid status: {}", e)))?,
                priority: row.priority.parse()
                    .map_err(|e| AssistantError::Internal(format!("Invalid priority: {}", e)))?,
                due_date: row.due_date,
                tags,
                created_at: row.created_at,
                updated_at: row.updated_at,
            });
        }

        Ok(tasks)
    }

    async fn store_briefing(&self, briefing: &DailyBriefing) -> Result<()> {
        let sections_json = serde_json::to_string(&briefing.sections)
            .map_err(|e| AssistantError::Internal(format!("Failed to serialize sections: {}", e)))?;

        sqlx::query!(
            r#"
            INSERT INTO daily_briefings (id, date, sections, generated_at)
            VALUES (?, ?, ?, ?)
            "#,
            briefing.id.to_string(),
            briefing.date,
            sections_json,
            briefing.generated_at
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AssistantError::Database(format!("Failed to store briefing: {}", e)))?;

        debug!("Stored briefing: {}", briefing.id);
        Ok(())
    }

    async fn get_briefing(&self, id: Uuid) -> Result<Option<DailyBriefing>> {
        let row = sqlx::query!(
            "SELECT * FROM daily_briefings WHERE id = ?",
            id.to_string()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AssistantError::Database(format!("Failed to get briefing: {}", e)))?;

        match row {
            Some(row) => {
                let sections = serde_json::from_str(&row.sections)
                    .map_err(|e| AssistantError::Internal(format!("Failed to deserialize sections: {}", e)))?;

                Ok(Some(DailyBriefing {
                    id: Uuid::parse_str(&row.id)
                        .map_err(|e| AssistantError::Internal(format!("Invalid UUID: {}", e)))?,
                    date: row.date,
                    sections,
                    generated_at: row.generated_at,
                }))
            }
            None => Ok(None),
        }
    }

    async fn get_latest_briefing(&self) -> Result<Option<DailyBriefing>> {
        let row = sqlx::query!(
            "SELECT * FROM daily_briefings ORDER BY date DESC LIMIT 1"
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AssistantError::Database(format!("Failed to get latest briefing: {}", e)))?;

        match row {
            Some(row) => {
                let sections = serde_json::from_str(&row.sections)
                    .map_err(|e| AssistantError::Internal(format!("Failed to deserialize sections: {}", e)))?;

                Ok(Some(DailyBriefing {
                    id: Uuid::parse_str(&row.id)
                        .map_err(|e| AssistantError::Internal(format!("Invalid UUID: {}", e)))?,
                    date: row.date,
                    sections,
                    generated_at: row.generated_at,
                }))
            }
            None => Ok(None),
        }
    }

    async fn get_briefings_by_date_range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<DailyBriefing>> {
        let rows = sqlx::query!(
            "SELECT * FROM daily_briefings WHERE date BETWEEN ? AND ? ORDER BY date DESC",
            start,
            end
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AssistantError::Database(format!("Failed to get briefings by date range: {}", e)))?;

        let mut briefings = Vec::new();
        for row in rows {
            let sections = serde_json::from_str(&row.sections)
                .map_err(|e| AssistantError::Internal(format!("Failed to deserialize sections: {}", e)))?;

            briefings.push(DailyBriefing {
                id: Uuid::parse_str(&row.id)
                    .map_err(|e| AssistantError::Internal(format!("Invalid UUID: {}", e)))?,
                date: row.date,
                sections,
                generated_at: row.generated_at,
            });
        }

        Ok(briefings)
    }

    async fn cleanup_old_data(&self, retention_days: i64) -> Result<usize> {
        let cutoff = Utc::now() - chrono::Duration::days(retention_days);
        
        let documents_result = sqlx::query!(
            "DELETE FROM documents WHERE updated_at < ?",
            cutoff
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AssistantError::Database(format!("Failed to cleanup old documents: {}", e)))?;

        let tasks_result = sqlx::query!(
            "DELETE FROM tasks WHERE updated_at < ? AND status IN ('Completed', 'Cancelled', 'Failed')",
            cutoff
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AssistantError::Database(format!("Failed to cleanup old tasks: {}", e)))?;

        let briefings_result = sqlx::query!(
            "DELETE FROM daily_briefings WHERE date < ?",
            cutoff
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AssistantError::Database(format!("Failed to cleanup old briefings: {}", e)))?;

        let total_deleted = documents_result.rows_affected() + 
                           tasks_result.rows_affected() + 
                           briefings_result.rows_affected();

        info!("Cleaned up {} old records", total_deleted);
        Ok(total_deleted as usize)
    }

    async fn health_check(&self) -> Result<StorageHealth> {
        // Test database connectivity
        let pool_status = sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await;

        let status = match pool_status {
            Ok(_) => StorageStatus::Healthy,
            Err(_) => StorageStatus::Unhealthy,
        };

        Ok(StorageHealth {
            status,
            connection_pool_size: Some(self.pool.size() as usize),
            pending_migrations: None,
            disk_usage_mb: None,
            last_backup: None,
        })
    }
}

// Helper function to create storage instance
pub async fn create_storage(config: &StorageConfig) -> Result<Arc<dyn Storage + Send + Sync>> {
    let storage = SqliteStorage::new(config).await?;
    Ok(Arc::new(storage))
}

// Implement Display for enums to support string conversion
impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::Pending => write!(f, "Pending"),
            TaskStatus::InProgress => write!(f, "InProgress"),
            TaskStatus::Completed => write!(f, "Completed"),
            TaskStatus::Cancelled => write!(f, "Cancelled"),
            TaskStatus::Failed => write!(f, "Failed"),
        }
    }
}

impl std::str::FromStr for TaskStatus {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "Pending" => Ok(TaskStatus::Pending),
            "InProgress" => Ok(TaskStatus::InProgress),
            "Completed" => Ok(TaskStatus::Completed),
            "Cancelled" => Ok(TaskStatus::Cancelled),
            "Failed" => Ok(TaskStatus::Failed),
            _ => Err(format!("Invalid task status: {}", s)),
        }
    }
}

impl std::fmt::Display for rusty_ai_common::TaskPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            rusty_ai_common::TaskPriority::Critical => write!(f, "Critical"),
            rusty_ai_common::TaskPriority::High => write!(f, "High"),
            rusty_ai_common::TaskPriority::Medium => write!(f, "Medium"),
            rusty_ai_common::TaskPriority::Low => write!(f, "Low"),
        }
    }
}

impl std::str::FromStr for rusty_ai_common::TaskPriority {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "Critical" => Ok(rusty_ai_common::TaskPriority::Critical),
            "High" => Ok(rusty_ai_common::TaskPriority::High),
            "Medium" => Ok(rusty_ai_common::TaskPriority::Medium),
            "Low" => Ok(rusty_ai_common::TaskPriority::Low),
            _ => Err(format!("Invalid task priority: {}", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusty_ai_common::DocumentMetadata;

    #[tokio::test]
    async fn test_storage_operations() {
        let config = StorageConfig {
            database_url: "sqlite::memory:".to_string(),
            ..Default::default()
        };

        let storage = SqliteStorage::new(&config).await.unwrap();

        // Test document operations
        let doc = Document {
            id: Uuid::new_v4(),
            title: "Test Document".to_string(),
            content: "Test content".to_string(),
            metadata: DocumentMetadata {
                source: "test".to_string(),
                file_type: "text".to_string(),
                tags: vec!["test".to_string()],
                summary: None,
                importance_score: 0.5,
                embeddings: None,
            },
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        storage.store_document(&doc).await.unwrap();
        let retrieved = storage.get_document(doc.id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().title, doc.title);
    }
}