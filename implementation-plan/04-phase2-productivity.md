# Phase 2: Productivity Suite Implementation

## Overview

Phase 2 builds upon the MVP foundation to deliver comprehensive productivity features including task management, calendar integration, email processing, document analysis, and advanced workflow automation. This phase transforms the basic AI assistant into a powerful productivity companion.

## Feature Scope

### Core Productivity Features
1. **Task Management System**: Intelligent task creation, prioritization, and tracking
2. **Calendar Integration**: Google Calendar sync with smart scheduling
3. **Email Processing**: Automated email summarization and action extraction
4. **Document Analysis**: Advanced document understanding and insights
5. **Meeting Assistant**: Agenda creation, note-taking, and follow-up actions
6. **Workflow Automation**: Custom productivity workflows and triggers
7. **Time Tracking**: Automatic activity monitoring and productivity analytics

### Integration Points
- Google Workspace (Calendar, Gmail, Drive)
- Microsoft Office 365 (optional)
- Slack/Teams messaging platforms
- Popular project management tools (Asana, Trello, Notion)

## Architecture Enhancements

### 1. Productivity Service Layer

Create a new productivity crate structure:

```toml
# Add to workspace Cargo.toml
[workspace]
members = [
    # ... existing crates
    "crates/productivity",
    "crates/integrations",
    "crates/automation"
]

# crates/productivity/Cargo.toml
[package]
name = "productivity"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { workspace = true }
serde = { workspace = true }
anyhow = { workspace = true }
chrono = { version = "0.4", features = ["serde"] }
uuid = { workspace = true }
google-calendar3 = "5.0"
google-gmail1 = "5.0"
oauth2 = "4.4"
rrule = "0.11"
natural = "0.5"
```

### 2. Task Management System

#### Task Models (`crates/productivity/src/tasks/mod.rs`)

```rust
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, Duration};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub due_date: Option<DateTime<Utc>>,
    pub estimated_duration: Option<Duration>,
    pub actual_duration: Option<Duration>,
    pub tags: Vec<String>,
    pub project_id: Option<Uuid>,
    pub parent_task_id: Option<Uuid>,
    pub subtasks: Vec<Uuid>,
    pub dependencies: Vec<Uuid>,
    pub context: TaskContext,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    NotStarted,
    InProgress,
    Blocked,
    Completed,
    Cancelled,
    OnHold,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Low = 1,
    Medium = 2,
    High = 3,
    Critical = 4,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskContext {
    pub location: Option<String>,
    pub energy_level: Option<EnergyLevel>,
    pub required_tools: Vec<String>,
    pub estimated_focus_time: Option<Duration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EnergyLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub status: ProjectStatus,
    pub start_date: Option<DateTime<Utc>>,
    pub target_date: Option<DateTime<Utc>>,
    pub completion_date: Option<DateTime<Utc>>,
    pub tasks: Vec<Uuid>,
    pub milestones: Vec<Milestone>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProjectStatus {
    Planning,
    Active,
    OnHold,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub target_date: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub required_tasks: Vec<Uuid>,
}
```

#### Task Service Implementation

```rust
use anyhow::Result;
use std::collections::HashMap;
use chrono::{DateTime, Utc, Duration};

pub struct TaskService {
    storage: Arc<TaskStorage>,
    ai_service: Arc<AIService>,
    calendar_service: Arc<CalendarService>,
    scheduler: Arc<TaskScheduler>,
}

impl TaskService {
    pub fn new(
        storage: Arc<TaskStorage>,
        ai_service: Arc<AIService>,
        calendar_service: Arc<CalendarService>,
    ) -> Self {
        let scheduler = Arc::new(TaskScheduler::new());
        Self {
            storage,
            ai_service,
            calendar_service,
            scheduler,
        }
    }
    
    pub async fn create_task_from_text(&self, text: &str, user_context: &UserContext) -> Result<Task> {
        // Use AI to extract task details from natural language
        let task_extraction = self.ai_service.extract_task_details(text).await?;
        
        let task = Task {
            id: Uuid::new_v4(),
            title: task_extraction.title,
            description: task_extraction.description,
            status: TaskStatus::NotStarted,
            priority: task_extraction.priority.unwrap_or(TaskPriority::Medium),
            due_date: task_extraction.due_date,
            estimated_duration: task_extraction.estimated_duration,
            actual_duration: None,
            tags: task_extraction.tags,
            project_id: task_extraction.project_id,
            parent_task_id: None,
            subtasks: Vec::new(),
            dependencies: Vec::new(),
            context: task_extraction.context,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            completed_at: None,
        };
        
        // Auto-schedule the task if it has a deadline
        if task.due_date.is_some() {
            self.scheduler.schedule_task(&task, user_context).await?;
        }
        
        self.storage.store_task(&task).await?;
        Ok(task)
    }
    
    pub async fn break_down_task(&self, task_id: Uuid) -> Result<Vec<Task>> {
        let task = self.storage.get_task(task_id).await?
            .ok_or_else(|| anyhow::anyhow!("Task not found"))?;
        
        // Use AI to break down complex tasks into subtasks
        let subtask_suggestions = self.ai_service.suggest_subtasks(&task).await?;
        
        let mut subtasks = Vec::new();
        for suggestion in subtask_suggestions {
            let subtask = Task {
                id: Uuid::new_v4(),
                title: suggestion.title,
                description: suggestion.description,
                status: TaskStatus::NotStarted,
                priority: suggestion.priority,
                due_date: suggestion.due_date,
                estimated_duration: suggestion.estimated_duration,
                actual_duration: None,
                tags: task.tags.clone(),
                project_id: task.project_id,
                parent_task_id: Some(task_id),
                subtasks: Vec::new(),
                dependencies: Vec::new(),
                context: suggestion.context,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                completed_at: None,
            };
            
            self.storage.store_task(&subtask).await?;
            subtasks.push(subtask);
        }
        
        // Update parent task with subtask references
        let mut updated_task = task.clone();
        updated_task.subtasks = subtasks.iter().map(|t| t.id).collect();
        updated_task.updated_at = Utc::now();
        self.storage.update_task(&updated_task).await?;
        
        Ok(subtasks)
    }
    
    pub async fn get_next_tasks(&self, user_context: &UserContext, limit: usize) -> Result<Vec<Task>> {
        let all_tasks = self.storage.get_pending_tasks().await?;
        
        // Filter and prioritize tasks based on context
        let mut suitable_tasks: Vec<_> = all_tasks.into_iter()
            .filter(|task| self.is_task_suitable(task, user_context))
            .collect();
        
        // Sort by priority, due date, and context match
        suitable_tasks.sort_by(|a, b| {
            let priority_cmp = b.priority.cmp(&a.priority);
            if priority_cmp != std::cmp::Ordering::Equal {
                return priority_cmp;
            }
            
            match (&a.due_date, &b.due_date) {
                (Some(a_due), Some(b_due)) => a_due.cmp(b_due),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            }
        });
        
        Ok(suitable_tasks.into_iter().take(limit).collect())
    }
    
    fn is_task_suitable(&self, task: &Task, context: &UserContext) -> bool {
        // Check if task matches current context (location, energy, available time)
        if let Some(required_energy) = &task.context.energy_level {
            if context.current_energy_level < *required_energy {
                return false;
            }
        }
        
        if let Some(estimated_duration) = task.estimated_duration {
            if context.available_time < estimated_duration {
                return false;
            }
        }
        
        if let Some(required_location) = &task.context.location {
            if context.current_location != *required_location {
                return false;
            }
        }
        
        true
    }
    
    pub async fn complete_task(&self, task_id: Uuid, actual_duration: Option<Duration>) -> Result<Task> {
        let mut task = self.storage.get_task(task_id).await?
            .ok_or_else(|| anyhow::anyhow!("Task not found"))?;
        
        task.status = TaskStatus::Completed;
        task.completed_at = Some(Utc::now());
        task.actual_duration = actual_duration;
        task.updated_at = Utc::now();
        
        // Check if parent task can be completed
        if let Some(parent_id) = task.parent_task_id {
            self.check_parent_completion(parent_id).await?;
        }
        
        // Update project progress if applicable
        if let Some(project_id) = task.project_id {
            self.update_project_progress(project_id).await?;
        }
        
        self.storage.update_task(&task).await?;
        Ok(task)
    }
    
    async fn check_parent_completion(&self, parent_id: Uuid) -> Result<()> {
        let parent_task = self.storage.get_task(parent_id).await?
            .ok_or_else(|| anyhow::anyhow!("Parent task not found"))?;
        
        // Check if all subtasks are completed
        let mut all_completed = true;
        for subtask_id in &parent_task.subtasks {
            let subtask = self.storage.get_task(*subtask_id).await?;
            if let Some(subtask) = subtask {
                if subtask.status != TaskStatus::Completed {
                    all_completed = false;
                    break;
                }
            }
        }
        
        if all_completed {
            self.complete_task(parent_id, None).await?;
        }
        
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct UserContext {
    pub current_location: String,
    pub current_energy_level: EnergyLevel,
    pub available_time: Duration,
    pub preferred_work_hours: (u32, u32), // (start_hour, end_hour)
    pub focus_score: f32, // 0.0 to 1.0
}

#[derive(Debug, Clone)]
pub struct TaskExtraction {
    pub title: String,
    pub description: Option<String>,
    pub priority: Option<TaskPriority>,
    pub due_date: Option<DateTime<Utc>>,
    pub estimated_duration: Option<Duration>,
    pub tags: Vec<String>,
    pub project_id: Option<Uuid>,
    pub context: TaskContext,
}
```

### 3. Calendar Integration

#### Calendar Service (`crates/productivity/src/calendar.rs`)

```rust
use google_calendar3::{CalendarHub, oauth2, hyper, hyper_rustls, Error};
use chrono::{DateTime, Utc, Duration};
use std::collections::HashMap;

pub struct CalendarService {
    hub: CalendarHub<hyper_rustls::HttpsConnector<hyper::client::HttpConnector>>,
    calendar_id: String,
}

impl CalendarService {
    pub async fn new(
        client_id: &str,
        client_secret: &str,
        refresh_token: &str,
        calendar_id: &str,
    ) -> Result<Self> {
        let secret = oauth2::ApplicationSecret {
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            auth_uri: "https://accounts.google.com/o/oauth2/auth".to_string(),
            token_uri: "https://oauth2.googleapis.com/token".to_string(),
            ..Default::default()
        };
        
        let auth = oauth2::InstalledFlowAuthenticator::builder(
            secret,
            oauth2::InstalledFlowReturnMethod::HTTPRedirect,
        ).build().await?;
        
        let hub = CalendarHub::new(
            hyper::Client::builder().build(
                hyper_rustls::HttpsConnectorBuilder::new()
                    .with_native_roots()
                    .https_or_http()
                    .enable_http1()
                    .build()
            ),
            auth,
        );
        
        Ok(Self {
            hub,
            calendar_id: calendar_id.to_string(),
        })
    }
    
    pub async fn get_events(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Vec<CalendarEvent>> {
        let result = self.hub
            .events()
            .list(&self.calendar_id)
            .time_min(start_time.to_rfc3339())
            .time_max(end_time.to_rfc3339())
            .single_events(true)
            .order_by("startTime")
            .doit()
            .await?;
        
        let mut events = Vec::new();
        if let Some(items) = result.1.items {
            for item in items {
                if let Some(event) = self.convert_to_calendar_event(item) {
                    events.push(event);
                }
            }
        }
        
        Ok(events)
    }
    
    pub async fn create_event(&self, event: &CalendarEvent) -> Result<String> {
        let google_event = self.convert_from_calendar_event(event);
        
        let result = self.hub
            .events()
            .insert(google_event, &self.calendar_id)
            .doit()
            .await?;
        
        Ok(result.1.id.unwrap_or_default())
    }
    
    pub async fn find_free_slots(
        &self,
        duration: Duration,
        start_search: DateTime<Utc>,
        end_search: DateTime<Utc>,
        working_hours: (u32, u32), // (start_hour, end_hour)
    ) -> Result<Vec<TimeSlot>> {
        let events = self.get_events(start_search, end_search).await?;
        
        let mut free_slots = Vec::new();
        let mut current_time = start_search;
        
        while current_time + duration <= end_search {
            // Check if current time is within working hours
            let hour = current_time.hour();
            if hour < working_hours.0 || hour >= working_hours.1 {
                current_time = current_time + Duration::hours(1);
                continue;
            }
            
            // Check if there's a conflict with existing events
            let slot_end = current_time + duration;
            let has_conflict = events.iter().any(|event| {
                event.start_time < slot_end && event.end_time > current_time
            });
            
            if !has_conflict {
                free_slots.push(TimeSlot {
                    start: current_time,
                    end: slot_end,
                    duration,
                });
            }
            
            current_time = current_time + Duration::minutes(15); // 15-minute intervals
        }
        
        Ok(free_slots)
    }
    
    pub async fn schedule_task_automatically(
        &self,
        task: &Task,
        preferred_time: Option<DateTime<Utc>>,
        working_hours: (u32, u32),
    ) -> Result<Option<CalendarEvent>> {
        let duration = task.estimated_duration.unwrap_or(Duration::hours(1));
        
        let search_start = preferred_time.unwrap_or_else(|| Utc::now());
        let search_end = task.due_date.unwrap_or(search_start + Duration::days(7));
        
        let free_slots = self.find_free_slots(
            duration,
            search_start,
            search_end,
            working_hours,
        ).await?;
        
        if let Some(slot) = free_slots.first() {
            let event = CalendarEvent {
                id: None,
                title: format!("Work on: {}", task.title),
                description: task.description.clone(),
                start_time: slot.start,
                end_time: slot.end,
                location: task.context.location.clone(),
                attendees: Vec::new(),
                event_type: CalendarEventType::Task,
                task_id: Some(task.id),
            };
            
            let event_id = self.create_event(&event).await?;
            let mut created_event = event;
            created_event.id = Some(event_id);
            
            Ok(Some(created_event))
        } else {
            Ok(None)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub location: Option<String>,
    pub attendees: Vec<String>,
    pub event_type: CalendarEventType,
    pub task_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CalendarEventType {
    Meeting,
    Task,
    Reminder,
    Break,
    Personal,
}

#[derive(Debug, Clone)]
pub struct TimeSlot {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub duration: Duration,
}
```

### 4. Email Processing System

#### Email Service (`crates/productivity/src/email.rs`)

```rust
use google_gmail1::{GmailHub, oauth2, hyper, hyper_rustls};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Email {
    pub id: String,
    pub thread_id: String,
    pub subject: String,
    pub from: String,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub body: String,
    pub received_at: DateTime<Utc>,
    pub is_read: bool,
    pub labels: Vec<String>,
    pub priority: EmailPriority,
    pub extracted_actions: Vec<EmailAction>,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmailPriority {
    Low,
    Normal,
    High,
    Urgent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailAction {
    pub action_type: ActionType,
    pub description: String,
    pub due_date: Option<DateTime<Utc>>,
    pub priority: TaskPriority,
    pub extracted_from: String, // Text snippet that suggested this action
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionType {
    Reply,
    Schedule,
    CreateTask,
    SetReminder,
    Forward,
    Research,
    Call,
    Review,
}

pub struct EmailService {
    hub: GmailHub<hyper_rustls::HttpsConnector<hyper::client::HttpConnector>>,
    ai_service: Arc<AIService>,
    task_service: Arc<TaskService>,
}

impl EmailService {
    pub async fn new(
        client_id: &str,
        client_secret: &str,
        refresh_token: &str,
        ai_service: Arc<AIService>,
        task_service: Arc<TaskService>,
    ) -> Result<Self> {
        let secret = oauth2::ApplicationSecret {
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            auth_uri: "https://accounts.google.com/o/oauth2/auth".to_string(),
            token_uri: "https://oauth2.googleapis.com/token".to_string(),
            ..Default::default()
        };
        
        let auth = oauth2::InstalledFlowAuthenticator::builder(
            secret,
            oauth2::InstalledFlowReturnMethod::HTTPRedirect,
        ).build().await?;
        
        let hub = GmailHub::new(
            hyper::Client::builder().build(
                hyper_rustls::HttpsConnectorBuilder::new()
                    .with_native_roots()
                    .https_or_http()
                    .enable_http1()
                    .build()
            ),
            auth,
        );
        
        Ok(Self {
            hub,
            ai_service,
            task_service,
        })
    }
    
    pub async fn process_new_emails(&self) -> Result<Vec<Email>> {
        // Get unread emails
        let query = "is:unread -in:spam -in:trash";
        let message_list = self.hub
            .users()
            .messages_list("me")
            .q(query)
            .doit()
            .await?;
        
        let mut processed_emails = Vec::new();
        
        if let Some(messages) = message_list.1.messages {
            for message in messages.iter().take(20) { // Process up to 20 emails at once
                if let Some(id) = &message.id {
                    if let Ok(email) = self.process_single_email(id).await {
                        processed_emails.push(email);
                    }
                }
            }
        }
        
        Ok(processed_emails)
    }
    
    async fn process_single_email(&self, message_id: &str) -> Result<Email> {
        let message = self.hub
            .users()
            .messages_get("me", message_id)
            .doit()
            .await?;
        
        let gmail_message = message.1;
        
        // Extract email metadata
        let headers = gmail_message.payload.as_ref()
            .and_then(|p| p.headers.as_ref())
            .unwrap_or(&Vec::new());
        
        let subject = self.extract_header_value(headers, "Subject")
            .unwrap_or("(No Subject)".to_string());
        let from = self.extract_header_value(headers, "From")
            .unwrap_or("Unknown".to_string());
        let to = self.extract_header_value(headers, "To")
            .map(|to_str| vec![to_str])
            .unwrap_or_default();
        let cc = self.extract_header_value(headers, "Cc")
            .map(|cc_str| vec![cc_str])
            .unwrap_or_default();
        
        // Extract email body
        let body = self.extract_email_body(&gmail_message.payload)?;
        
        // Analyze email content with AI
        let analysis = self.ai_service.analyze_email(&subject, &body, &from).await?;
        
        let email = Email {
            id: message_id.to_string(),
            thread_id: gmail_message.thread_id.unwrap_or_default(),
            subject,
            from,
            to,
            cc,
            body,
            received_at: Utc::now(), // This would be parsed from the actual timestamp
            is_read: false,
            labels: gmail_message.label_ids.unwrap_or_default(),
            priority: analysis.priority,
            extracted_actions: analysis.actions,
            summary: analysis.summary,
        };
        
        // Auto-create tasks for actionable emails
        for action in &email.extracted_actions {
            if matches!(action.action_type, ActionType::CreateTask | ActionType::Schedule | ActionType::SetReminder) {
                let task_text = format!("{}: {}", action.action_type.to_string(), action.description);
                let task = self.task_service.create_task_from_text(&task_text, &Default::default()).await?;
                
                // Link task to email for reference
                // This would be stored in a task-email mapping table
            }
        }
        
        Ok(email)
    }
    
    pub async fn generate_email_summary(&self, emails: &[Email]) -> Result<String> {
        let high_priority_emails: Vec<_> = emails.iter()
            .filter(|e| matches!(e.priority, EmailPriority::High | EmailPriority::Urgent))
            .collect();
        
        let actionable_emails: Vec<_> = emails.iter()
            .filter(|e| !e.extracted_actions.is_empty())
            .collect();
        
        let mut summary_parts = Vec::new();
        
        if !high_priority_emails.is_empty() {
            summary_parts.push(format!(
                "**High Priority Emails ({}):**\n{}",
                high_priority_emails.len(),
                high_priority_emails.iter()
                    .map(|e| format!("• {} from {}", e.subject, e.from))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }
        
        if !actionable_emails.is_empty() {
            summary_parts.push(format!(
                "**Emails Requiring Action ({}):**\n{}",
                actionable_emails.len(),
                actionable_emails.iter()
                    .map(|e| format!("• {}: {} action(s)", e.subject, e.extracted_actions.len()))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }
        
        if summary_parts.is_empty() {
            Ok("No urgent emails requiring immediate attention.".to_string())
        } else {
            Ok(summary_parts.join("\n\n"))
        }
    }
    
    fn extract_header_value(&self, headers: &[MessagePartHeader], name: &str) -> Option<String> {
        headers.iter()
            .find(|h| h.name.as_deref() == Some(name))
            .and_then(|h| h.value.clone())
    }
    
    fn extract_email_body(&self, payload: &Option<MessagePart>) -> Result<String> {
        // This would implement proper email body extraction
        // handling multipart messages, HTML, etc.
        Ok("Email body extraction not implemented".to_string())
    }
}

#[derive(Debug, Clone)]
pub struct EmailAnalysis {
    pub priority: EmailPriority,
    pub actions: Vec<EmailAction>,
    pub summary: Option<String>,
    pub sentiment: EmailSentiment,
    pub category: EmailCategory,
}

#[derive(Debug, Clone)]
pub enum EmailSentiment {
    Positive,
    Neutral,
    Negative,
    Urgent,
}

#[derive(Debug, Clone)]
pub enum EmailCategory {
    Work,
    Personal,
    Newsletter,
    Promotion,
    Social,
    Finance,
    Travel,
    Shopping,
    Support,
}
```

### 5. Meeting Assistant

#### Meeting Service (`crates/productivity/src/meetings.rs`)

```rust
use std::collections::HashMap;
use chrono::{DateTime, Utc, Duration};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meeting {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub attendees: Vec<Attendee>,
    pub location: Option<String>,
    pub meeting_link: Option<String>,
    pub agenda: Option<MeetingAgenda>,
    pub notes: Vec<MeetingNote>,
    pub action_items: Vec<ActionItem>,
    pub recording_url: Option<String>,
    pub status: MeetingStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attendee {
    pub email: String,
    pub name: Option<String>,
    pub role: AttendeeRole,
    pub status: AttendeeStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AttendeeRole {
    Organizer,
    Required,
    Optional,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AttendeeStatus {
    Accepted,
    Declined,
    Tentative,
    NoResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingAgenda {
    pub items: Vec<AgendaItem>,
    pub estimated_duration: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgendaItem {
    pub title: String,
    pub description: Option<String>,
    pub estimated_duration: Duration,
    pub presenter: Option<String>,
    pub attachments: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingNote {
    pub id: Uuid,
    pub content: String,
    pub author: String,
    pub timestamp: DateTime<Utc>,
    pub note_type: NoteType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NoteType {
    General,
    Decision,
    Action,
    Question,
    Important,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionItem {
    pub id: Uuid,
    pub description: String,
    pub assignee: String,
    pub due_date: Option<DateTime<Utc>>,
    pub priority: TaskPriority,
    pub status: ActionItemStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionItemStatus {
    Open,
    InProgress,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MeetingStatus {
    Scheduled,
    InProgress,
    Completed,
    Cancelled,
    Postponed,
}

pub struct MeetingService {
    storage: Arc<MeetingStorage>,
    calendar_service: Arc<CalendarService>,
    ai_service: Arc<AIService>,
    task_service: Arc<TaskService>,
}

impl MeetingService {
    pub async fn prepare_meeting(&self, meeting_id: Uuid) -> Result<MeetingPreparation> {
        let meeting = self.storage.get_meeting(meeting_id).await?
            .ok_or_else(|| anyhow::anyhow!("Meeting not found"))?;
        
        // Generate agenda if not exists
        let agenda = if meeting.agenda.is_none() {
            Some(self.generate_agenda(&meeting).await?)
        } else {
            meeting.agenda
        };
        
        // Gather relevant documents and context
        let context_documents = self.gather_meeting_context(&meeting).await?;
        
        // Generate meeting briefing
        let briefing = self.ai_service.generate_meeting_briefing(
            &meeting,
            &context_documents,
        ).await?;
        
        Ok(MeetingPreparation {
            meeting,
            agenda,
            context_documents,
            briefing,
            suggested_questions: self.generate_suggested_questions(&meeting).await?,
        })
    }
    
    async fn generate_agenda(&self, meeting: &Meeting) -> Result<MeetingAgenda> {
        // Use AI to generate agenda based on meeting title, description, and attendees
        let agenda_suggestions = self.ai_service.suggest_meeting_agenda(
            &meeting.title,
            meeting.description.as_deref(),
            &meeting.attendees,
            meeting.end_time - meeting.start_time,
        ).await?;
        
        Ok(MeetingAgenda {
            items: agenda_suggestions.items,
            estimated_duration: agenda_suggestions.total_duration,
        })
    }
    
    async fn gather_meeting_context(&self, meeting: &Meeting) -> Result<Vec<Document>> {
        let mut search_terms = vec![meeting.title.clone()];
        
        if let Some(description) = &meeting.description {
            search_terms.push(description.clone());
        }
        
        // Add attendee organizations/companies as search terms
        for attendee in &meeting.attendees {
            if let Some(domain) = attendee.email.split('@').nth(1) {
                search_terms.push(domain.to_string());
            }
        }
        
        let mut all_documents = Vec::new();
        for term in search_terms {
            let docs = self.knowledge_base.search_documents(&term, 5, 0.4).await?;
            all_documents.extend(docs);
        }
        
        // Remove duplicates and sort by relevance
        all_documents.sort_by(|a, b| a.id.cmp(&b.id));
        all_documents.dedup_by(|a, b| a.id == b.id);
        
        Ok(all_documents.into_iter().take(10).collect())
    }
    
    pub async fn take_notes_automatically(&self, meeting_id: Uuid, audio_transcript: &str) -> Result<Vec<MeetingNote>> {
        let analysis = self.ai_service.analyze_meeting_transcript(audio_transcript).await?;
        
        let mut notes = Vec::new();
        
        // Create structured notes from transcript analysis
        for segment in analysis.key_segments {
            let note = MeetingNote {
                id: Uuid::new_v4(),
                content: segment.content,
                author: "AI Assistant".to_string(),
                timestamp: segment.timestamp,
                note_type: segment.note_type,
            };
            
            self.storage.store_meeting_note(meeting_id, &note).await?;
            notes.push(note);
        }
        
        // Extract action items
        for action in analysis.action_items {
            let action_item = ActionItem {
                id: Uuid::new_v4(),
                description: action.description,
                assignee: action.assignee,
                due_date: action.due_date,
                priority: action.priority,
                status: ActionItemStatus::Open,
                created_at: Utc::now(),
            };
            
            self.storage.store_action_item(meeting_id, &action_item).await?;
            
            // Create corresponding task
            let task_text = format!("Action from meeting '{}': {}", 
                meeting_id, action_item.description);
            self.task_service.create_task_from_text(&task_text, &Default::default()).await?;
        }
        
        Ok(notes)
    }
    
    pub async fn generate_meeting_summary(&self, meeting_id: Uuid) -> Result<MeetingSummary> {
        let meeting = self.storage.get_meeting(meeting_id).await?
            .ok_or_else(|| anyhow::anyhow!("Meeting not found"))?;
        
        let notes = self.storage.get_meeting_notes(meeting_id).await?;
        let action_items = self.storage.get_action_items(meeting_id).await?;
        
        let key_decisions = notes.iter()
            .filter(|n| matches!(n.note_type, NoteType::Decision))
            .collect::<Vec<_>>();
        
        let important_points = notes.iter()
            .filter(|n| matches!(n.note_type, NoteType::Important))
            .collect::<Vec<_>>();
        
        Ok(MeetingSummary {
            meeting_title: meeting.title,
            date: meeting.start_time,
            duration: meeting.end_time - meeting.start_time,
            attendee_count: meeting.attendees.len(),
            key_decisions: key_decisions.iter().map(|n| n.content.clone()).collect(),
            important_points: important_points.iter().map(|n| n.content.clone()).collect(),
            action_items_count: action_items.len(),
            next_steps: action_items.iter()
                .filter(|a| matches!(a.status, ActionItemStatus::Open))
                .map(|a| format!("{}: {}", a.assignee, a.description))
                .collect(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct MeetingPreparation {
    pub meeting: Meeting,
    pub agenda: Option<MeetingAgenda>,
    pub context_documents: Vec<Document>,
    pub briefing: String,
    pub suggested_questions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingSummary {
    pub meeting_title: String,
    pub date: DateTime<Utc>,
    pub duration: Duration,
    pub attendee_count: usize,
    pub key_decisions: Vec<String>,
    pub important_points: Vec<String>,
    pub action_items_count: usize,
    pub next_steps: Vec<String>,
}
```

## API Endpoints for Productivity Features

### 1. Task Management Endpoints

```rust
// Task management routes
pub fn create_productivity_routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/tasks", post(create_task).get(list_tasks))
        .route("/api/v1/tasks/:id", get(get_task).put(update_task).delete(delete_task))
        .route("/api/v1/tasks/:id/complete", post(complete_task))
        .route("/api/v1/tasks/:id/subtasks", post(break_down_task))
        .route("/api/v1/tasks/next", get(get_next_tasks))
        .route("/api/v1/projects", post(create_project).get(list_projects))
        .route("/api/v1/projects/:id", get(get_project).put(update_project))
        .route("/api/v1/calendar/events", get(get_calendar_events).post(create_calendar_event))
        .route("/api/v1/calendar/schedule-task/:task_id", post(schedule_task))
        .route("/api/v1/emails/process", post(process_emails))
        .route("/api/v1/emails/summary", get(get_email_summary))
        .route("/api/v1/meetings/:id/prepare", get(prepare_meeting))
        .route("/api/v1/meetings/:id/notes", post(add_meeting_note))
        .route("/api/v1/meetings/:id/summary", get(get_meeting_summary))
}

#[derive(Deserialize)]
pub struct CreateTaskRequest {
    pub text: String,
    pub project_id: Option<Uuid>,
    pub due_date: Option<DateTime<Utc>>,
    pub priority: Option<TaskPriority>,
}

pub async fn create_task(
    State(state): State<AppState>,
    Json(request): Json<CreateTaskRequest>,
) -> Result<Json<Task>, StatusCode> {
    let user_context = UserContext::default(); // This would come from authentication
    
    let task = state.productivity.task_service
        .create_task_from_text(&request.text, &user_context)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(task))
}

pub async fn get_next_tasks(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<Task>>, StatusCode> {
    let limit = params.get("limit")
        .and_then(|l| l.parse().ok())
        .unwrap_or(10);
    
    let user_context = UserContext::default(); // This would come from user preferences
    
    let tasks = state.productivity.task_service
        .get_next_tasks(&user_context, limit)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(tasks))
}
```

## Performance Optimizations

### 1. Background Processing

```rust
use tokio::time::{interval, Duration};

pub struct ProductivityScheduler {
    task_service: Arc<TaskService>,
    email_service: Arc<EmailService>,
    calendar_service: Arc<CalendarService>,
}

impl ProductivityScheduler {
    pub async fn start_background_tasks(&self) {
        // Process emails every 5 minutes
        let email_service = self.email_service.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(300)); // 5 minutes
            loop {
                interval.tick().await;
                if let Err(e) = email_service.process_new_emails().await {
                    tracing::error!("Failed to process emails: {}", e);
                }
            }
        });
        
        // Update task priorities every hour
        let task_service = self.task_service.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(3600)); // 1 hour
            loop {
                interval.tick().await;
                if let Err(e) = task_service.recalculate_priorities().await {
                    tracing::error!("Failed to recalculate task priorities: {}", e);
                }
            }
        });
        
        // Sync calendar events every 15 minutes
        let calendar_service = self.calendar_service.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(900)); // 15 minutes
            loop {
                interval.tick().await;
                if let Err(e) = calendar_service.sync_events().await {
                    tracing::error!("Failed to sync calendar events: {}", e);
                }
            }
        });
    }
}
```

### 2. Caching Layer

```rust
use tokio::sync::RwLock;
use std::collections::HashMap;
use chrono::{DateTime, Utc};

pub struct ProductivityCache {
    task_cache: RwLock<HashMap<Uuid, (Task, DateTime<Utc>)>>,
    project_cache: RwLock<HashMap<Uuid, (Project, DateTime<Utc>)>>,
    email_summaries: RwLock<HashMap<String, (String, DateTime<Utc>)>>,
    cache_ttl: Duration,
}

impl ProductivityCache {
    pub fn new(cache_ttl: Duration) -> Self {
        Self {
            task_cache: RwLock::new(HashMap::new()),
            project_cache: RwLock::new(HashMap::new()),
            email_summaries: RwLock::new(HashMap::new()),
            cache_ttl,
        }
    }
    
    pub async fn get_task(&self, id: Uuid) -> Option<Task> {
        let cache = self.task_cache.read().await;
        if let Some((task, timestamp)) = cache.get(&id) {
            if Utc::now() - *timestamp < self.cache_ttl {
                return Some(task.clone());
            }
        }
        None
    }
    
    pub async fn cache_task(&self, task: Task) {
        let mut cache = self.task_cache.write().await;
        cache.insert(task.id, (task, Utc::now()));
    }
    
    pub async fn invalidate_task(&self, id: Uuid) {
        let mut cache = self.task_cache.write().await;
        cache.remove(&id);
    }
}
```

## Testing Strategy

### 1. Integration Tests

```rust
#[cfg(test)]
mod productivity_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_task_creation_and_scheduling() {
        let test_env = setup_test_environment().await;
        
        // Create a task from natural language
        let task_text = "Review the quarterly report by Friday at 3 PM";
        let task = test_env.task_service
            .create_task_from_text(task_text, &test_context())
            .await
            .unwrap();
        
        assert_eq!(task.title, "Review the quarterly report");
        assert!(task.due_date.is_some());
        
        // Test automatic scheduling
        let scheduled_event = test_env.calendar_service
            .schedule_task_automatically(&task, None, (9, 17))
            .await
            .unwrap();
        
        assert!(scheduled_event.is_some());
    }
    
    #[tokio::test]
    async fn test_email_processing() {
        let test_env = setup_test_environment().await;
        
        // Mock email processing
        let emails = test_env.email_service
            .process_new_emails()
            .await
            .unwrap();
        
        // Verify action extraction
        for email in emails {
            if !email.extracted_actions.is_empty() {
                assert!(email.summary.is_some());
            }
        }
    }
    
    #[tokio::test]
    async fn test_meeting_preparation() {
        let test_env = setup_test_environment().await;
        
        let meeting_id = create_test_meeting(&test_env).await;
        
        let preparation = test_env.meeting_service
            .prepare_meeting(meeting_id)
            .await
            .unwrap();
        
        assert!(preparation.agenda.is_some());
        assert!(!preparation.context_documents.is_empty());
        assert!(!preparation.briefing.is_empty());
    }
}
```

## Performance Benchmarks

### Target Metrics for Phase 2
- Task creation from text: < 200ms
- Email processing per email: < 500ms
- Calendar event scheduling: < 300ms
- Meeting preparation: < 2 seconds
- Background email sync: < 30 seconds for 100 emails

## Security Considerations

### 1. OAuth Token Management

```rust
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};

pub struct TokenManager {
    encryption_key: LessSafeKey,
}

impl TokenManager {
    pub fn encrypt_token(&self, token: &str) -> Result<Vec<u8>, EncryptionError> {
        // Encrypt OAuth tokens before storing
        let mut data = token.as_bytes().to_vec();
        let nonce = Nonce::assume_unique_for_key([0u8; 12]); // Use proper nonce generation
        self.encryption_key.seal_in_place_append_tag(nonce, Aad::empty(), &mut data)?;
        Ok(data)
    }
    
    pub fn decrypt_token(&self, encrypted_data: &[u8]) -> Result<String, EncryptionError> {
        // Decrypt OAuth tokens when needed
        let mut data = encrypted_data.to_vec();
        let nonce = Nonce::assume_unique_for_key([0u8; 12]);
        let decrypted = self.encryption_key.open_in_place(nonce, Aad::empty(), &mut data)?;
        Ok(String::from_utf8(decrypted.to_vec())?)
    }
}
```

### 2. Data Privacy

```rust
pub struct PrivacyConfig {
    pub store_email_content: bool,
    pub encrypt_task_details: bool,
    pub anonymize_meeting_transcripts: bool,
    pub retention_days: u32,
}

impl Default for PrivacyConfig {
    fn default() -> Self {
        Self {
            store_email_content: false, // Only store metadata by default
            encrypt_task_details: true,
            anonymize_meeting_transcripts: true,
            retention_days: 365,
        }
    }
}
```

Phase 2 implementation significantly extends the Personal AI Assistant's capabilities, providing a comprehensive productivity suite that integrates seamlessly with existing workflows while maintaining strong privacy and security standards. The modular design allows for incremental deployment and easy customization based on user preferences.