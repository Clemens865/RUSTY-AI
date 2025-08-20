use anyhow::Result;
use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestAssistantMessageArgs,
        ChatCompletionRequestMessage,
        ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestUserMessageArgs,
        CreateChatCompletionRequestArgs,
    },
    Client,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationContext {
    pub session_id: String,
    pub messages: Vec<ChatMessage>,
    pub system_prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

pub struct AIService {
    client: Client<OpenAIConfig>,
    model: String,
    max_tokens: u16,
    temperature: f32,
    conversations: Arc<RwLock<std::collections::HashMap<String, ConversationContext>>>,
}

impl AIService {
    pub fn new(api_key: Option<String>) -> Result<Self> {
        // Use environment variable if no API key provided
        let config = if let Some(key) = api_key {
            OpenAIConfig::new().with_api_key(key)
        } else {
            // This will use OPENAI_API_KEY environment variable
            OpenAIConfig::new()
        };

        let client = Client::with_config(config);
        
        Ok(Self {
            client,
            model: "gpt-3.5-turbo".to_string(), // Using fastest model for quick responses
            max_tokens: 800,
            temperature: 0.7,
            conversations: Arc::new(RwLock::new(std::collections::HashMap::new())),
        })
    }

    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: u16) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature;
        self
    }

    pub async fn process_message(
        &self,
        message: &str,
        session_id: &str,
    ) -> Result<String> {
        debug!("Processing message for session: {}", session_id);
        
        // Get or create conversation context
        let mut conversations = self.conversations.write().await;
        let context = conversations.entry(session_id.to_string()).or_insert_with(|| {
            ConversationContext {
                session_id: session_id.to_string(),
                messages: Vec::new(),
                system_prompt: self.get_system_prompt(),
            }
        });

        // Add user message to context
        context.messages.push(ChatMessage {
            role: "user".to_string(),
            content: message.to_string(),
            timestamp: chrono::Utc::now(),
        });

        // Prepare messages for OpenAI API
        let mut openai_messages = vec![
            ChatCompletionRequestMessage::System(
                ChatCompletionRequestSystemMessageArgs::default()
                    .content(context.system_prompt.clone())
                    .build()?
            )
        ];

        // Add conversation history (keep last 10 messages for context)
        let history_messages: Vec<_> = context.messages
            .iter()
            .rev()
            .take(10)
            .rev()
            .collect();

        for msg in history_messages {
            let openai_msg = match msg.role.as_str() {
                "user" => ChatCompletionRequestMessage::User(
                    ChatCompletionRequestUserMessageArgs::default()
                        .content(msg.content.clone())
                        .build()?
                ),
                "assistant" => ChatCompletionRequestMessage::Assistant(
                    ChatCompletionRequestAssistantMessageArgs::default()
                        .content(msg.content.clone())
                        .build()?
                ),
                _ => continue,
            };
            openai_messages.push(openai_msg);
        }

        // Create chat completion request
        let request = CreateChatCompletionRequestArgs::default()
            .model(&self.model)
            .messages(openai_messages)
            .max_tokens(self.max_tokens)
            .temperature(self.temperature)
            .build()?;

        // Call OpenAI API
        let response = match self.client.chat().create(request).await {
            Ok(resp) => resp,
            Err(e) => {
                error!("OpenAI API error: {}", e);
                return Ok(self.get_fallback_response());
            }
        };

        // Extract response text
        let response_text = response
            .choices
            .first()
            .and_then(|choice| choice.message.content.clone())
            .unwrap_or_else(|| self.get_fallback_response());

        // Add assistant response to context
        context.messages.push(ChatMessage {
            role: "assistant".to_string(),
            content: response_text.clone(),
            timestamp: chrono::Utc::now(),
        });

        // Limit conversation history to prevent unbounded growth
        if context.messages.len() > 50 {
            context.messages.drain(0..10);
        }

        info!("Generated response for session: {}", session_id);
        Ok(response_text)
    }

    pub async fn clear_session(&self, session_id: &str) -> Result<()> {
        let mut conversations = self.conversations.write().await;
        conversations.remove(session_id);
        Ok(())
    }

    pub async fn get_session_history(&self, session_id: &str) -> Result<Vec<ChatMessage>> {
        let conversations = self.conversations.read().await;
        if let Some(context) = conversations.get(session_id) {
            Ok(context.messages.clone())
        } else {
            Ok(Vec::new())
        }
    }

    fn get_system_prompt(&self) -> String {
        "You are a helpful personal AI assistant. You are knowledgeable, friendly, and professional. \
         You help users with various tasks including answering questions, providing information, \
         and assisting with productivity. Keep your responses concise and relevant. \
         If you don't know something, be honest about it.".to_string()
    }

    fn get_fallback_response(&self) -> String {
        "I apologize, but I'm having trouble processing your request at the moment. \
         Please try again or rephrase your question.".to_string()
    }
}

// Database models for persistence
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct SessionRecord {
    pub id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub metadata: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct MessageRecord {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub struct ConversationStore {
    pool: sqlx::SqlitePool,
}

impl ConversationStore {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = sqlx::SqlitePool::connect(database_url).await?;
        
        // Create tables if they don't exist
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                created_at TIMESTAMP NOT NULL,
                updated_at TIMESTAMP NOT NULL,
                metadata TEXT
            )
            "#,
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at TIMESTAMP NOT NULL,
                FOREIGN KEY (session_id) REFERENCES sessions(id)
            )
            "#,
        )
        .execute(&pool)
        .await?;

        Ok(Self { pool })
    }

    pub async fn save_session(&self, session: &SessionRecord) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO sessions (id, created_at, updated_at, metadata)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                updated_at = excluded.updated_at,
                metadata = excluded.metadata
            "#,
        )
        .bind(&session.id)
        .bind(&session.created_at)
        .bind(&session.updated_at)
        .bind(&session.metadata)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn save_message(&self, message: &MessageRecord) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO messages (id, session_id, role, content, created_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&message.id)
        .bind(&message.session_id)
        .bind(&message.role)
        .bind(&message.content)
        .bind(&message.created_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_session_messages(&self, session_id: &str) -> Result<Vec<MessageRecord>> {
        let messages = sqlx::query_as::<_, MessageRecord>(
            r#"
            SELECT * FROM messages
            WHERE session_id = ?
            ORDER BY created_at ASC
            "#,
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(messages)
    }

    pub async fn get_session(&self, session_id: &str) -> Result<Option<SessionRecord>> {
        let session = sqlx::query_as::<_, SessionRecord>(
            r#"
            SELECT * FROM sessions
            WHERE id = ?
            "#,
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(session)
    }
    
    pub async fn get_recent_sessions(&self, limit: i32) -> Result<Vec<SessionRecord>> {
        let sessions = sqlx::query_as::<_, SessionRecord>(
            r#"
            SELECT id, created_at, updated_at, metadata 
            FROM sessions 
            ORDER BY updated_at DESC 
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        
        Ok(sessions)
    }
    
}