use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use std::collections::HashMap;

// Document types for knowledge base
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub metadata: DocumentMetadata,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub source: String,
    pub file_type: String,
    pub tags: Vec<String>,
    pub summary: Option<String>,
    pub importance_score: f32,
    pub embeddings: Option<Vec<f32>>,
}

// Voice interaction types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceInteraction {
    pub id: Uuid,
    pub transcript: String,
    pub intent: Intent,
    pub response: String,
    pub confidence: f32,
    pub processing_time_ms: u64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Intent {
    Query { query: String },
    Command { action: String, parameters: Vec<String> },
    Information { topic: String },
    Unknown,
}

// Daily briefing types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyBriefing {
    pub id: Uuid,
    pub date: DateTime<Utc>,
    pub sections: Vec<BriefingSection>,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BriefingSection {
    pub title: String,
    pub content: String,
    pub priority: BriefingPriority,
    pub source_documents: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BriefingPriority {
    Critical,
    High,
    Medium,
    Low,
}

// Plugin system types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub capabilities: Vec<String>,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    pub enabled: bool,
    pub priority: i32,
    pub settings: HashMap<String, serde_json::Value>,
}

// Error types
#[derive(Debug, thiserror::Error)]
pub enum AssistantError {
    #[error("Database error: {0}")]
    Database(String),
    
    #[error("API error: {0}")]
    Api(String),
    
    #[error("Voice processing error: {0}")]
    VoiceProcessing(String),
    
    #[error("Plugin error: {0}")]
    Plugin(String),
    
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Unauthorized")]
    Unauthorized,
    
    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, AssistantError>;

// API response types
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: DateTime<Utc>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: Utc::now(),
        }
    }
    
    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
            timestamp: Utc::now(),
        }
    }
}

// Task types for orchestration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub due_date: Option<DateTime<Utc>>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskPriority {
    Critical,
    High,
    Medium,
    Low,
}

// User context and session management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContext {
    pub user_id: Uuid,
    pub session_id: Uuid,
    pub preferences: UserPreferences,
    pub active_plugins: Vec<String>,
    pub conversation_history: Vec<ConversationTurn>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    pub language: String,
    pub timezone: String,
    pub voice_settings: VoiceSettings,
    pub notification_settings: NotificationSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceSettings {
    pub enabled: bool,
    pub voice_id: String,
    pub speed: f32,
    pub pitch: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSettings {
    pub enabled: bool,
    pub channels: Vec<NotificationChannel>,
    pub quiet_hours: Option<(String, String)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationChannel {
    Email,
    Sms,
    Push,
    InApp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTurn {
    pub id: Uuid,
    pub user_input: String,
    pub assistant_response: String,
    pub intent: Intent,
    pub timestamp: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_document_creation() {
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
        
        assert_eq!(doc.title, "Test Document");
        assert_eq!(doc.metadata.source, "test");
    }
    
    #[test]
    fn test_api_response() {
        let response = ApiResponse::success("data");
        assert!(response.success);
        assert_eq!(response.data, Some("data"));
        
        let error_response: ApiResponse<String> = ApiResponse::error("error".to_string());
        assert!(!error_response.success);
        assert_eq!(error_response.error, Some("error".to_string()));
    }
}