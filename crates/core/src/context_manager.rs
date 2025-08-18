use rusty_ai_common::{UserContext, ConversationTurn, Intent, UserPreferences, Result, AssistantError};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use tracing::{info, debug, warn};

pub struct ContextManager {
    active_sessions: HashMap<Uuid, UserSession>,
    max_conversation_length: usize,
    context_retention_hours: i64,
}

#[derive(Debug, Clone)]
pub struct UserSession {
    pub user_id: Uuid,
    pub session_id: Uuid,
    pub context: UserContext,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub conversation_turns: Vec<ConversationTurn>,
}

impl ContextManager {
    pub fn new() -> Self {
        Self {
            active_sessions: HashMap::new(),
            max_conversation_length: 100,
            context_retention_hours: 24,
        }
    }

    pub fn new_with_config(max_conversation_length: usize, context_retention_hours: i64) -> Self {
        Self {
            active_sessions: HashMap::new(),
            max_conversation_length,
            context_retention_hours,
        }
    }

    pub async fn create_session(&mut self, user_id: Uuid, preferences: UserPreferences) -> Result<Uuid> {
        let session_id = Uuid::new_v4();
        let now = Utc::now();

        let context = UserContext {
            user_id,
            session_id,
            preferences,
            active_plugins: Vec::new(),
            conversation_history: Vec::new(),
        };

        let session = UserSession {
            user_id,
            session_id,
            context,
            created_at: now,
            last_activity: now,
            conversation_turns: Vec::new(),
        };

        self.active_sessions.insert(session_id, session);
        info!("Created new session {} for user {}", session_id, user_id);

        Ok(session_id)
    }

    pub async fn get_session(&self, session_id: Uuid) -> Result<&UserSession> {
        self.active_sessions
            .get(&session_id)
            .ok_or_else(|| AssistantError::NotFound(format!("Session not found: {}", session_id)))
    }

    pub async fn get_session_mut(&mut self, session_id: Uuid) -> Result<&mut UserSession> {
        self.active_sessions
            .get_mut(&session_id)
            .ok_or_else(|| AssistantError::NotFound(format!("Session not found: {}", session_id)))
    }

    pub async fn get_user_context(&self, session_id: Uuid) -> Result<&UserContext> {
        let session = self.get_session(session_id).await?;
        Ok(&session.context)
    }

    pub async fn update_last_activity(&mut self, session_id: Uuid) -> Result<()> {
        let session = self.get_session_mut(session_id).await?;
        session.last_activity = Utc::now();
        debug!("Updated last activity for session {}", session_id);
        Ok(())
    }

    pub async fn add_conversation_turn(
        &mut self,
        session_id: Uuid,
        user_input: String,
        assistant_response: String,
        intent: Intent,
    ) -> Result<()> {
        let session = self.get_session_mut(session_id).await?;
        
        let turn = ConversationTurn {
            id: Uuid::new_v4(),
            user_input,
            assistant_response,
            intent,
            timestamp: Utc::now(),
        };

        session.conversation_turns.push(turn.clone());
        session.context.conversation_history.push(turn);
        session.last_activity = Utc::now();

        // Trim conversation history if it exceeds max length
        if session.conversation_turns.len() > self.max_conversation_length {
            let excess = session.conversation_turns.len() - self.max_conversation_length;
            session.conversation_turns.drain(0..excess);
            session.context.conversation_history.drain(0..excess);
            debug!("Trimmed {} old conversation turns from session {}", excess, session_id);
        }

        debug!("Added conversation turn to session {}", session_id);
        Ok(())
    }

    pub async fn update_user_preferences(
        &mut self,
        session_id: Uuid,
        preferences: UserPreferences,
    ) -> Result<()> {
        let session = self.get_session_mut(session_id).await?;
        session.context.preferences = preferences;
        session.last_activity = Utc::now();
        info!("Updated user preferences for session {}", session_id);
        Ok(())
    }

    pub async fn add_active_plugin(&mut self, session_id: Uuid, plugin_id: String) -> Result<()> {
        let session = self.get_session_mut(session_id).await?;
        if !session.context.active_plugins.contains(&plugin_id) {
            session.context.active_plugins.push(plugin_id.clone());
            session.last_activity = Utc::now();
            debug!("Added plugin {} to session {}", plugin_id, session_id);
        }
        Ok(())
    }

    pub async fn remove_active_plugin(&mut self, session_id: Uuid, plugin_id: &str) -> Result<()> {
        let session = self.get_session_mut(session_id).await?;
        session.context.active_plugins.retain(|id| id != plugin_id);
        session.last_activity = Utc::now();
        debug!("Removed plugin {} from session {}", plugin_id, session_id);
        Ok(())
    }

    pub async fn get_conversation_history(&self, session_id: Uuid, limit: Option<usize>) -> Result<Vec<ConversationTurn>> {
        let session = self.get_session(session_id).await?;
        let history = &session.conversation_turns;
        
        match limit {
            Some(n) => {
                let start = if history.len() > n { history.len() - n } else { 0 };
                Ok(history[start..].to_vec())
            }
            None => Ok(history.clone()),
        }
    }

    pub async fn get_recent_context(&self, session_id: Uuid, turns: usize) -> Result<String> {
        let recent_turns = self.get_conversation_history(session_id, Some(turns)).await?;
        
        let context = recent_turns
            .iter()
            .map(|turn| format!("User: {}\nAssistant: {}", turn.user_input, turn.assistant_response))
            .collect::<Vec<_>>()
            .join("\n\n");
        
        Ok(context)
    }

    pub async fn cleanup_expired_sessions(&mut self) -> Result<usize> {
        let cutoff = Utc::now() - chrono::Duration::hours(self.context_retention_hours);
        let initial_count = self.active_sessions.len();
        
        self.active_sessions.retain(|_, session| session.last_activity > cutoff);
        
        let removed_count = initial_count - self.active_sessions.len();
        if removed_count > 0 {
            info!("Cleaned up {} expired sessions", removed_count);
        }
        
        Ok(removed_count)
    }

    pub async fn get_active_session_count(&self) -> usize {
        self.active_sessions.len()
    }

    pub async fn get_user_sessions(&self, user_id: Uuid) -> Vec<&UserSession> {
        self.active_sessions
            .values()
            .filter(|session| session.user_id == user_id)
            .collect()
    }

    pub async fn destroy_session(&mut self, session_id: Uuid) -> Result<()> {
        match self.active_sessions.remove(&session_id) {
            Some(_) => {
                info!("Destroyed session {}", session_id);
                Ok(())
            }
            None => Err(AssistantError::NotFound(format!("Session not found: {}", session_id)))
        }
    }

    pub async fn destroy_user_sessions(&mut self, user_id: Uuid) -> Result<usize> {
        let session_ids: Vec<Uuid> = self.active_sessions
            .iter()
            .filter_map(|(id, session)| {
                if session.user_id == user_id {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect();

        let count = session_ids.len();
        for session_id in session_ids {
            self.active_sessions.remove(&session_id);
        }

        if count > 0 {
            info!("Destroyed {} sessions for user {}", count, user_id);
        }

        Ok(count)
    }

    pub async fn get_session_summary(&self, session_id: Uuid) -> Result<SessionSummary> {
        let session = self.get_session(session_id).await?;
        
        Ok(SessionSummary {
            session_id: session.session_id,
            user_id: session.user_id,
            created_at: session.created_at,
            last_activity: session.last_activity,
            turn_count: session.conversation_turns.len(),
            active_plugins: session.context.active_plugins.clone(),
        })
    }

    pub async fn get_all_session_summaries(&self) -> Vec<SessionSummary> {
        self.active_sessions
            .values()
            .map(|session| SessionSummary {
                session_id: session.session_id,
                user_id: session.user_id,
                created_at: session.created_at,
                last_activity: session.last_activity,
                turn_count: session.conversation_turns.len(),
                active_plugins: session.context.active_plugins.clone(),
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct SessionSummary {
    pub session_id: Uuid,
    pub user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub turn_count: usize,
    pub active_plugins: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusty_ai_common::{VoiceSettings, NotificationSettings};

    fn create_test_preferences() -> UserPreferences {
        UserPreferences {
            language: "en".to_string(),
            timezone: "UTC".to_string(),
            voice_settings: VoiceSettings {
                enabled: true,
                voice_id: "default".to_string(),
                speed: 1.0,
                pitch: 1.0,
            },
            notification_settings: NotificationSettings {
                enabled: true,
                channels: vec![],
                quiet_hours: None,
            },
        }
    }

    #[tokio::test]
    async fn test_create_session() {
        let mut manager = ContextManager::new();
        let user_id = Uuid::new_v4();
        let preferences = create_test_preferences();

        let session_id = manager.create_session(user_id, preferences).await.unwrap();
        assert!(manager.active_sessions.contains_key(&session_id));

        let session = manager.get_session(session_id).await.unwrap();
        assert_eq!(session.user_id, user_id);
        assert_eq!(session.session_id, session_id);
    }

    #[tokio::test]
    async fn test_conversation_turns() {
        let mut manager = ContextManager::new();
        let user_id = Uuid::new_v4();
        let preferences = create_test_preferences();

        let session_id = manager.create_session(user_id, preferences).await.unwrap();

        manager.add_conversation_turn(
            session_id,
            "Hello".to_string(),
            "Hi there!".to_string(),
            Intent::Query { query: "greeting".to_string() },
        ).await.unwrap();

        let history = manager.get_conversation_history(session_id, None).await.unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].user_input, "Hello");
        assert_eq!(history[0].assistant_response, "Hi there!");
    }

    #[tokio::test]
    async fn test_session_cleanup() {
        let mut manager = ContextManager::new_with_config(10, 0); // 0 hour retention
        let user_id = Uuid::new_v4();
        let preferences = create_test_preferences();

        let session_id = manager.create_session(user_id, preferences).await.unwrap();
        assert_eq!(manager.get_active_session_count().await, 1);

        // Wait a bit and cleanup
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        let removed = manager.cleanup_expired_sessions().await.unwrap();
        assert_eq!(removed, 1);
        assert_eq!(manager.get_active_session_count().await, 0);
    }
}