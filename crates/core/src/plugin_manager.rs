use rusty_ai_common::{Result, AssistantError, PluginMetadata, PluginConfig, Intent, UserContext, Task};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error};

#[async_trait]
pub trait AssistantPlugin: Send + Sync {
    fn metadata(&self) -> PluginMetadata;
    async fn initialize(&mut self, config: PluginConfig) -> Result<()>;
    async fn handle_intent(&self, intent: Intent, context: &UserContext) -> Result<String>;
    async fn health_check(&self) -> PluginHealth;
    fn can_handle_query(&self, query: &str) -> bool;
    fn can_handle_task(&self, task_name: &str) -> bool;
    async fn process_query(&self, query: String, context: &UserContext) -> Result<String>;
    async fn execute_task(&self, task: &Task) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct PluginHealth {
    pub status: HealthStatus,
    pub message: Option<String>,
    pub last_check: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

pub struct PluginManager {
    plugins: Arc<RwLock<HashMap<String, Box<dyn AssistantPlugin>>>>,
    configs: Arc<RwLock<HashMap<String, PluginConfig>>>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            configs: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub async fn register_plugin(&self, mut plugin: Box<dyn AssistantPlugin>) -> Result<()> {
        let metadata = plugin.metadata();
        let plugin_id = metadata.id.clone();
        
        info!("Registering plugin: {} ({})", metadata.name, metadata.version);
        
        // Get or create default config
        let config = self.configs.read().await
            .get(&plugin_id)
            .cloned()
            .unwrap_or_else(|| PluginConfig {
                enabled: true,
                priority: 0,
                settings: HashMap::new(),
            });
        
        // Initialize plugin
        plugin.initialize(config.clone()).await?;
        
        // Store plugin
        let mut plugins = self.plugins.write().await;
        plugins.insert(plugin_id.clone(), plugin);
        
        // Store config
        let mut configs = self.configs.write().await;
        configs.insert(plugin_id, config);
        
        info!("Plugin registered successfully");
        Ok(())
    }
    
    pub async fn unregister_plugin(&self, plugin_id: &str) -> Result<()> {
        info!("Unregistering plugin: {}", plugin_id);
        
        let mut plugins = self.plugins.write().await;
        if plugins.remove(plugin_id).is_none() {
            return Err(AssistantError::NotFound(format!("Plugin not found: {}", plugin_id)));
        }
        
        let mut configs = self.configs.write().await;
        configs.remove(plugin_id);
        
        info!("Plugin unregistered successfully");
        Ok(())
    }
    
    pub async fn get_plugin(&self, plugin_id: &str) -> Option<Box<dyn AssistantPlugin>> {
        let plugins = self.plugins.read().await;
        plugins.get(plugin_id).map(|p| {
            // This is a simplified approach - in production, you'd need proper cloning
            // or Arc-based sharing of plugins
            unimplemented!("Plugin cloning not implemented")
        })
    }
    
    pub async fn get_active_plugins(&self) -> Vec<Arc<dyn AssistantPlugin>> {
        let plugins = self.plugins.read().await;
        let configs = self.configs.read().await;
        
        let mut active_plugins = Vec::new();
        
        for (id, _plugin) in plugins.iter() {
            if let Some(config) = configs.get(id) {
                if config.enabled {
                    // In production, you'd return Arc references to the plugins
                    // For now, we'll skip this
                    warn!("Plugin sharing not fully implemented");
                }
            }
        }
        
        active_plugins
    }
    
    pub async fn load_plugins(&self) -> Result<()> {
        info!("Loading plugins from directory");
        
        // In a real implementation, this would scan a directory for plugin files
        // and dynamically load them. For MVP, we'll register built-in plugins
        
        // Register built-in plugins here
        // Example: self.register_plugin(Box::new(KnowledgePlugin::new())).await?;
        
        Ok(())
    }
    
    pub async fn unload_all(&self) -> Result<()> {
        info!("Unloading all plugins");
        
        let mut plugins = self.plugins.write().await;
        plugins.clear();
        
        let mut configs = self.configs.write().await;
        configs.clear();
        
        Ok(())
    }
    
    pub async fn health_check_all(&self) -> HashMap<String, PluginHealth> {
        let mut health_results = HashMap::new();
        let plugins = self.plugins.read().await;
        
        for (id, plugin) in plugins.iter() {
            let health = plugin.health_check().await;
            health_results.insert(id.clone(), health);
        }
        
        health_results
    }
    
    pub async fn update_config(&self, plugin_id: &str, config: PluginConfig) -> Result<()> {
        let mut configs = self.configs.write().await;
        configs.insert(plugin_id.to_string(), config.clone());
        
        // Reinitialize plugin with new config if it exists
        let plugins = self.plugins.read().await;
        if let Some(_plugin) = plugins.get(plugin_id) {
            // In production, you'd reinitialize the plugin here
            info!("Plugin {} configuration updated", plugin_id);
        }
        
        Ok(())
    }
}

// Example plugin implementation for testing
pub struct ExamplePlugin {
    metadata: PluginMetadata,
    config: Option<PluginConfig>,
}

impl ExamplePlugin {
    pub fn new() -> Self {
        Self {
            metadata: PluginMetadata {
                id: "example".to_string(),
                name: "Example Plugin".to_string(),
                version: "0.1.0".to_string(),
                description: "An example plugin for testing".to_string(),
                author: "AI Assistant Team".to_string(),
                capabilities: vec!["example".to_string()],
                dependencies: vec![],
            },
            config: None,
        }
    }
}

#[async_trait]
impl AssistantPlugin for ExamplePlugin {
    fn metadata(&self) -> PluginMetadata {
        self.metadata.clone()
    }
    
    async fn initialize(&mut self, config: PluginConfig) -> Result<()> {
        self.config = Some(config);
        Ok(())
    }
    
    async fn handle_intent(&self, _intent: Intent, _context: &UserContext) -> Result<String> {
        Ok("Example plugin response".to_string())
    }
    
    async fn health_check(&self) -> PluginHealth {
        PluginHealth {
            status: HealthStatus::Healthy,
            message: None,
            last_check: chrono::Utc::now(),
        }
    }
    
    fn can_handle_query(&self, _query: &str) -> bool {
        false
    }
    
    fn can_handle_task(&self, _task_name: &str) -> bool {
        false
    }
    
    async fn process_query(&self, _query: String, _context: &UserContext) -> Result<String> {
        Ok("Example query response".to_string())
    }
    
    async fn execute_task(&self, _task: &Task) -> Result<()> {
        Ok(())
    }
}