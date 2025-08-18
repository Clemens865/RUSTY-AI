use rusty_ai_common::{Result, Intent, Task, TaskStatus, UserContext};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use uuid::Uuid;
use tracing::{info, error, debug};
use super::{plugin_manager::PluginManager, context_manager::ContextManager, storage::Storage};

pub struct Orchestrator {
    plugin_manager: Arc<PluginManager>,
    context_manager: Arc<RwLock<ContextManager>>,
    storage: Arc<dyn Storage + Send + Sync>,
    task_queue: Arc<RwLock<Vec<Task>>>,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl Orchestrator {
    pub fn new(
        plugin_manager: Arc<PluginManager>,
        context_manager: Arc<RwLock<ContextManager>>,
        storage: Arc<dyn Storage + Send + Sync>,
    ) -> Self {
        Self {
            plugin_manager,
            context_manager,
            storage,
            task_queue: Arc::new(RwLock::new(Vec::new())),
            shutdown_tx: None,
        }
    }
    
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing orchestrator");
        
        // Load persisted tasks
        let tasks = self.storage.get_pending_tasks().await?;
        let mut queue = self.task_queue.write().await;
        *queue = tasks;
        
        info!("Orchestrator initialized with {} pending tasks", queue.len());
        Ok(())
    }
    
    pub async fn process_intent(&self, intent: Intent, context: &UserContext) -> Result<String> {
        debug!("Processing intent: {:?}", intent);
        
        match intent {
            Intent::Query { query } => {
                self.handle_query(query, context).await
            },
            Intent::Command { action, parameters } => {
                self.handle_command(action, parameters, context).await
            },
            Intent::Information { topic } => {
                self.handle_information_request(topic, context).await
            },
            Intent::Unknown => {
                Ok("I'm not sure I understand. Could you please rephrase?".to_string())
            }
        }
    }
    
    async fn handle_query(&self, query: String, context: &UserContext) -> Result<String> {
        // Route to appropriate plugin based on query content
        let plugins = self.plugin_manager.get_active_plugins().await;
        
        for plugin in plugins {
            if plugin.can_handle_query(&query) {
                return plugin.process_query(query, context).await;
            }
        }
        
        // Fallback to knowledge base search
        Ok(format!("Searching for information about: {}", query))
    }
    
    async fn handle_command(&self, action: String, parameters: Vec<String>, context: &UserContext) -> Result<String> {
        // Create a task for the command
        let task = Task {
            id: Uuid::new_v4(),
            name: action.clone(),
            description: format!("Execute {} with params: {:?}", action, parameters),
            status: TaskStatus::Pending,
            priority: rusty_ai_common::TaskPriority::Medium,
            due_date: None,
            tags: vec![action.clone()],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        
        // Add to queue
        let mut queue = self.task_queue.write().await;
        queue.push(task.clone());
        
        // Persist task
        self.storage.store_task(&task).await?;
        
        Ok(format!("Command '{}' has been queued for execution", action))
    }
    
    async fn handle_information_request(&self, topic: String, context: &UserContext) -> Result<String> {
        // Search knowledge base for topic
        let documents = self.storage.search_documents(&topic, 5).await?;
        
        if documents.is_empty() {
            Ok(format!("I don't have information about {} yet. Would you like me to research it?", topic))
        } else {
            let summary = documents.iter()
                .map(|d| format!("- {}: {}", d.title, d.metadata.summary.as_ref().unwrap_or(&d.content[..100.min(d.content.len())].to_string())))
                .collect::<Vec<_>>()
                .join("\n");
            
            Ok(format!("Here's what I know about {}:\n{}", topic, summary))
        }
    }
    
    pub async fn execute_pending_tasks(&self) -> Result<()> {
        let mut queue = self.task_queue.write().await;
        let pending_tasks: Vec<Task> = queue.drain(..).filter(|t| t.status == TaskStatus::Pending).collect();
        
        for mut task in pending_tasks {
            info!("Executing task: {}", task.name);
            task.status = TaskStatus::InProgress;
            self.storage.update_task_status(task.id, TaskStatus::InProgress).await?;
            
            // Execute task through appropriate plugin
            match self.execute_task(&task).await {
                Ok(_) => {
                    task.status = TaskStatus::Completed;
                    self.storage.update_task_status(task.id, TaskStatus::Completed).await?;
                    info!("Task {} completed successfully", task.name);
                },
                Err(e) => {
                    task.status = TaskStatus::Failed;
                    self.storage.update_task_status(task.id, TaskStatus::Failed).await?;
                    error!("Task {} failed: {}", task.name, e);
                }
            }
        }
        
        Ok(())
    }
    
    async fn execute_task(&self, task: &Task) -> Result<()> {
        // Route task to appropriate plugin
        let plugins = self.plugin_manager.get_active_plugins().await;
        
        for plugin in plugins {
            if plugin.can_handle_task(&task.name) {
                return plugin.execute_task(task).await;
            }
        }
        
        Err(rusty_ai_common::AssistantError::Plugin(format!("No plugin can handle task: {}", task.name)))
    }
    
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down orchestrator");
        
        // Signal shutdown if channel exists
        if let Some(tx) = &self.shutdown_tx {
            let _ = tx.send(()).await;
        }
        
        Ok(())
    }
}