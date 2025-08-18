use rusty_ai_common::{Result, AssistantError, DailyBriefing, BriefingSection, BriefingPriority, Document, Task, TaskStatus, UserContext};
use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc, TimeZone};
use tracing::{info, debug, error};
use super::storage::Storage;

pub struct BriefingGenerator {
    storage: Arc<dyn Storage + Send + Sync>,
    config: BriefingConfig,
}

#[derive(Debug, Clone)]
pub struct BriefingConfig {
    pub max_sections: usize,
    pub max_documents_per_section: usize,
    pub importance_threshold: f32,
    pub include_completed_tasks: bool,
    pub task_lookback_days: i64,
    pub document_lookback_days: i64,
}

impl Default for BriefingConfig {
    fn default() -> Self {
        Self {
            max_sections: 10,
            max_documents_per_section: 5,
            importance_threshold: 0.3,
            include_completed_tasks: true,
            task_lookback_days: 7,
            document_lookback_days: 30,
        }
    }
}

impl BriefingGenerator {
    pub fn new(storage: Arc<dyn Storage + Send + Sync>) -> Self {
        Self {
            storage,
            config: BriefingConfig::default(),
        }
    }

    pub fn new_with_config(storage: Arc<dyn Storage + Send + Sync>, config: BriefingConfig) -> Self {
        Self {
            storage,
            config,
        }
    }

    pub async fn generate_daily_briefing(&self, date: DateTime<Utc>, user_context: &UserContext) -> Result<DailyBriefing> {
        info!("Generating daily briefing for {}", date.format("%Y-%m-%d"));

        let mut sections = Vec::new();

        // Add task overview section
        if let Ok(task_section) = self.generate_task_section(date).await {
            sections.push(task_section);
        }

        // Add recent documents section
        if let Ok(docs_section) = self.generate_documents_section(date).await {
            sections.push(docs_section);
        }

        // Add priority items section
        if let Ok(priority_section) = self.generate_priority_section(date).await {
            sections.push(priority_section);
        }

        // Add upcoming items section
        if let Ok(upcoming_section) = self.generate_upcoming_section(date).await {
            sections.push(upcoming_section);
        }

        // Add knowledge insights section
        if let Ok(insights_section) = self.generate_insights_section(date).await {
            sections.push(insights_section);
        }

        // Sort sections by priority
        sections.sort_by(|a, b| b.priority.cmp(&a.priority));

        // Limit to max sections
        if sections.len() > self.config.max_sections {
            sections.truncate(self.config.max_sections);
        }

        let briefing = DailyBriefing {
            id: Uuid::new_v4(),
            date,
            sections,
            generated_at: Utc::now(),
        };

        // Store the briefing
        self.storage.store_briefing(&briefing).await?;

        info!("Generated daily briefing with {} sections", briefing.sections.len());
        Ok(briefing)
    }

    async fn generate_task_section(&self, date: DateTime<Utc>) -> Result<BriefingSection> {
        let start_date = date - chrono::Duration::days(self.config.task_lookback_days);
        
        // Get pending tasks
        let pending_tasks = self.storage.get_tasks_by_status(TaskStatus::Pending).await?;
        
        // Get completed tasks from the last week
        let completed_tasks = if self.config.include_completed_tasks {
            self.storage.get_tasks_by_status(TaskStatus::Completed).await?
                .into_iter()
                .filter(|task| task.updated_at >= start_date)
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        let content = self.format_task_overview(&pending_tasks, &completed_tasks);
        let priority = if pending_tasks.is_empty() {
            BriefingPriority::Low
        } else if pending_tasks.len() > 10 {
            BriefingPriority::High
        } else {
            BriefingPriority::Medium
        };

        let source_documents = pending_tasks.iter().chain(completed_tasks.iter())
            .map(|_| Uuid::new_v4()) // Placeholder - would link to task documents
            .collect();

        Ok(BriefingSection {
            title: "Task Overview".to_string(),
            content,
            priority,
            source_documents,
        })
    }

    async fn generate_documents_section(&self, date: DateTime<Utc>) -> Result<BriefingSection> {
        let start_date = date - chrono::Duration::days(self.config.document_lookback_days);
        
        // Search for recent documents
        let recent_docs = self.storage.search_documents("", 20).await?
            .into_iter()
            .filter(|doc| doc.updated_at >= start_date)
            .filter(|doc| doc.metadata.importance_score >= self.config.importance_threshold)
            .take(self.config.max_documents_per_section)
            .collect::<Vec<_>>();

        if recent_docs.is_empty() {
            return Err(AssistantError::NotFound("No recent documents found".to_string()));
        }

        let content = self.format_documents_overview(&recent_docs);
        let priority = if recent_docs.iter().any(|doc| doc.metadata.importance_score > 0.8) {
            BriefingPriority::High
        } else {
            BriefingPriority::Medium
        };

        let source_documents = recent_docs.iter().map(|doc| doc.id).collect();

        Ok(BriefingSection {
            title: "Recent Documents".to_string(),
            content,
            priority,
            source_documents,
        })
    }

    async fn generate_priority_section(&self, _date: DateTime<Utc>) -> Result<BriefingSection> {
        // Get high-priority pending tasks
        let high_priority_tasks = self.storage.get_tasks_by_status(TaskStatus::Pending).await?
            .into_iter()
            .filter(|task| matches!(task.priority, rusty_ai_common::TaskPriority::Critical | rusty_ai_common::TaskPriority::High))
            .collect::<Vec<_>>();

        if high_priority_tasks.is_empty() {
            return Err(AssistantError::NotFound("No high-priority items found".to_string()));
        }

        let content = self.format_priority_items(&high_priority_tasks);
        let priority = BriefingPriority::Critical;

        let source_documents = high_priority_tasks.iter()
            .map(|_| Uuid::new_v4()) // Placeholder
            .collect();

        Ok(BriefingSection {
            title: "Priority Items".to_string(),
            content,
            priority,
            source_documents,
        })
    }

    async fn generate_upcoming_section(&self, date: DateTime<Utc>) -> Result<BriefingSection> {
        let end_date = date + chrono::Duration::days(7);
        
        let upcoming_tasks = self.storage.get_tasks_by_status(TaskStatus::Pending).await?
            .into_iter()
            .filter(|task| {
                task.due_date.map_or(false, |due| due <= end_date && due >= date)
            })
            .collect::<Vec<_>>();

        if upcoming_tasks.is_empty() {
            return Err(AssistantError::NotFound("No upcoming items found".to_string()));
        }

        let content = self.format_upcoming_items(&upcoming_tasks);
        let priority = BriefingPriority::Medium;

        let source_documents = upcoming_tasks.iter()
            .map(|_| Uuid::new_v4()) // Placeholder
            .collect();

        Ok(BriefingSection {
            title: "Upcoming Items".to_string(),
            content,
            priority,
            source_documents,
        })
    }

    async fn generate_insights_section(&self, date: DateTime<Utc>) -> Result<BriefingSection> {
        let start_date = date - chrono::Duration::days(7);
        
        // Get recent documents for analysis
        let recent_docs = self.storage.search_documents("", 10).await?
            .into_iter()
            .filter(|doc| doc.updated_at >= start_date)
            .collect::<Vec<_>>();

        if recent_docs.is_empty() {
            return Err(AssistantError::NotFound("No recent documents for insights".to_string()));
        }

        let content = self.generate_knowledge_insights(&recent_docs);
        let priority = BriefingPriority::Low;

        let source_documents = recent_docs.iter().map(|doc| doc.id).collect();

        Ok(BriefingSection {
            title: "Knowledge Insights".to_string(),
            content,
            priority,
            source_documents,
        })
    }

    fn format_task_overview(&self, pending_tasks: &[Task], completed_tasks: &[Task]) -> String {
        let mut content = String::new();

        if !pending_tasks.is_empty() {
            content.push_str(&format!("**Pending Tasks ({}):**\n", pending_tasks.len()));
            for task in pending_tasks.iter().take(5) {
                let priority_icon = match task.priority {
                    rusty_ai_common::TaskPriority::Critical => "🔴",
                    rusty_ai_common::TaskPriority::High => "🟡",
                    rusty_ai_common::TaskPriority::Medium => "🟢",
                    rusty_ai_common::TaskPriority::Low => "⚪",
                };
                content.push_str(&format!("- {} {}\n", priority_icon, task.name));
            }
            if pending_tasks.len() > 5 {
                content.push_str(&format!("... and {} more\n", pending_tasks.len() - 5));
            }
            content.push('\n');
        }

        if !completed_tasks.is_empty() {
            content.push_str(&format!("**Recently Completed ({}):**\n", completed_tasks.len()));
            for task in completed_tasks.iter().take(3) {
                content.push_str(&format!("- ✅ {}\n", task.name));
            }
            if completed_tasks.len() > 3 {
                content.push_str(&format!("... and {} more\n", completed_tasks.len() - 3));
            }
        }

        content
    }

    fn format_documents_overview(&self, documents: &[Document]) -> String {
        let mut content = String::new();
        content.push_str(&format!("**Recent Documents ({}):**\n", documents.len()));

        for doc in documents {
            let summary = doc.metadata.summary.as_ref()
                .unwrap_or(&doc.content)
                .chars()
                .take(100)
                .collect::<String>();
            
            content.push_str(&format!(
                "- **{}** ({}): {}{}\n",
                doc.title,
                doc.metadata.file_type,
                summary,
                if summary.len() >= 100 { "..." } else { "" }
            ));
        }

        content
    }

    fn format_priority_items(&self, tasks: &[Task]) -> String {
        let mut content = String::new();
        content.push_str("**High Priority Items:**\n");

        for task in tasks {
            let priority_indicator = if matches!(task.priority, rusty_ai_common::TaskPriority::Critical) {
                "🚨 CRITICAL"
            } else {
                "⚠️ HIGH"
            };

            let due_info = task.due_date
                .map(|due| format!(" (Due: {})", due.format("%Y-%m-%d")))
                .unwrap_or_default();

            content.push_str(&format!(
                "- {} {}{}\n  {}\n",
                priority_indicator,
                task.name,
                due_info,
                task.description
            ));
        }

        content
    }

    fn format_upcoming_items(&self, tasks: &[Task]) -> String {
        let mut content = String::new();
        content.push_str("**Upcoming This Week:**\n");

        for task in tasks {
            let due_date = task.due_date
                .map(|due| due.format("%Y-%m-%d").to_string())
                .unwrap_or("No due date".to_string());

            content.push_str(&format!(
                "- **{}** (Due: {})\n  {}\n",
                task.name,
                due_date,
                task.description
            ));
        }

        content
    }

    fn generate_knowledge_insights(&self, documents: &[Document]) -> String {
        let mut content = String::new();
        content.push_str("**Knowledge Insights:**\n");

        // Analyze document tags for trends
        let mut tag_counts = std::collections::HashMap::new();
        for doc in documents {
            for tag in &doc.metadata.tags {
                *tag_counts.entry(tag.clone()).or_insert(0) += 1;
            }
        }

        // Get top tags
        let mut sorted_tags: Vec<_> = tag_counts.iter().collect();
        sorted_tags.sort_by(|a, b| b.1.cmp(a.1));

        if !sorted_tags.is_empty() {
            content.push_str("**Trending Topics:**\n");
            for (tag, count) in sorted_tags.iter().take(5) {
                content.push_str(&format!("- {}: {} documents\n", tag, count));
            }
            content.push('\n');
        }

        // Document type analysis
        let mut type_counts = std::collections::HashMap::new();
        for doc in documents {
            *type_counts.entry(doc.metadata.file_type.clone()).or_insert(0) += 1;
        }

        if !type_counts.is_empty() {
            content.push_str("**Document Types:**\n");
            for (file_type, count) in type_counts {
                content.push_str(&format!("- {}: {} documents\n", file_type, count));
            }
        }

        content
    }

    pub async fn get_briefing_history(&self, days: i64) -> Result<Vec<DailyBriefing>> {
        let end_date = Utc::now();
        let start_date = end_date - chrono::Duration::days(days);
        
        self.storage.get_briefings_by_date_range(start_date, end_date).await
    }

    pub async fn get_latest_briefing(&self) -> Result<Option<DailyBriefing>> {
        self.storage.get_latest_briefing().await
    }

    pub async fn regenerate_briefing(&self, date: DateTime<Utc>, user_context: &UserContext) -> Result<DailyBriefing> {
        // Delete existing briefing for the date if it exists
        if let Ok(existing_briefings) = self.storage.get_briefings_by_date_range(date, date).await {
            for briefing in existing_briefings {
                // Note: We'd need a delete method in storage for this
                debug!("Would delete existing briefing: {}", briefing.id);
            }
        }

        // Generate new briefing
        self.generate_daily_briefing(date, user_context).await
    }

    pub fn update_config(&mut self, config: BriefingConfig) {
        self.config = config;
        info!("Updated briefing configuration");
    }
}

// Helper function to compare briefing priorities
impl PartialOrd for BriefingPriority {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BriefingPriority {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let self_value = match self {
            BriefingPriority::Critical => 4,
            BriefingPriority::High => 3,
            BriefingPriority::Medium => 2,
            BriefingPriority::Low => 1,
        };
        
        let other_value = match other {
            BriefingPriority::Critical => 4,
            BriefingPriority::High => 3,
            BriefingPriority::Medium => 2,
            BriefingPriority::Low => 1,
        };
        
        self_value.cmp(&other_value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use rusty_ai_common::{DocumentMetadata, UserPreferences, VoiceSettings, NotificationSettings};

    // Mock storage for testing
    struct MockStorage;

    #[async_trait::async_trait]
    impl Storage for MockStorage {
        async fn store_document(&self, _document: &Document) -> Result<()> { Ok(()) }
        async fn get_document(&self, _id: Uuid) -> Result<Option<Document>> { Ok(None) }
        async fn update_document(&self, _document: &Document) -> Result<()> { Ok(()) }
        async fn delete_document(&self, _id: Uuid) -> Result<()> { Ok(()) }
        async fn search_documents(&self, _query: &str, _limit: usize) -> Result<Vec<Document>> { Ok(Vec::new()) }
        async fn get_documents_by_tags(&self, _tags: &[String], _limit: usize) -> Result<Vec<Document>> { Ok(Vec::new()) }
        async fn store_task(&self, _task: &Task) -> Result<()> { Ok(()) }
        async fn get_task(&self, _id: Uuid) -> Result<Option<Task>> { Ok(None) }
        async fn update_task_status(&self, _id: Uuid, _status: TaskStatus) -> Result<()> { Ok(()) }
        async fn get_pending_tasks(&self) -> Result<Vec<Task>> { Ok(Vec::new()) }
        async fn get_tasks_by_status(&self, _status: TaskStatus) -> Result<Vec<Task>> { Ok(Vec::new()) }
        async fn store_briefing(&self, _briefing: &DailyBriefing) -> Result<()> { Ok(()) }
        async fn get_briefing(&self, _id: Uuid) -> Result<Option<DailyBriefing>> { Ok(None) }
        async fn get_latest_briefing(&self) -> Result<Option<DailyBriefing>> { Ok(None) }
        async fn get_briefings_by_date_range(&self, _start: DateTime<Utc>, _end: DateTime<Utc>) -> Result<Vec<DailyBriefing>> { Ok(Vec::new()) }
        async fn cleanup_old_data(&self, _retention_days: i64) -> Result<usize> { Ok(0) }
        async fn health_check(&self) -> Result<super::storage::StorageHealth> { 
            Ok(super::storage::StorageHealth {
                status: super::storage::StorageStatus::Healthy,
                connection_pool_size: None,
                pending_migrations: None,
                disk_usage_mb: None,
                last_backup: None,
            })
        }
    }

    #[tokio::test]
    async fn test_briefing_generation() {
        let storage = Arc::new(MockStorage);
        let generator = BriefingGenerator::new(storage);
        
        let user_context = UserContext {
            user_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            preferences: UserPreferences {
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
            },
            active_plugins: vec![],
            conversation_history: vec![],
        };

        let briefing = generator.generate_daily_briefing(Utc::now(), &user_context).await.unwrap();
        assert!(!briefing.sections.is_empty());
    }
}