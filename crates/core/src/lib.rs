pub mod orchestrator;
pub mod plugin_manager;
pub mod context_manager;
pub mod storage;
pub mod briefing;
pub mod intent;
pub mod database;

use rusty_ai_common::{Result, AssistantError};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct AssistantCore {
    pub orchestrator: Arc<orchestrator::Orchestrator>,
    pub plugin_manager: Arc<plugin_manager::PluginManager>,
    pub context_manager: Arc<RwLock<context_manager::ContextManager>>,
    pub storage: Arc<dyn storage::Storage + Send + Sync>,
    pub intent_classifier: Arc<intent::IntentClassifier>,
    pub briefing_generator: Arc<briefing::BriefingGenerator>,
}

impl AssistantCore {
    pub async fn new(config: CoreConfig) -> Result<Self> {
        let storage = storage::create_storage(&config.storage_config).await?;
        let plugin_manager = Arc::new(plugin_manager::PluginManager::new());
        let context_manager = Arc::new(RwLock::new(context_manager::ContextManager::new()));
        let intent_classifier = Arc::new(intent::IntentClassifier::new());
        let briefing_generator = Arc::new(briefing::BriefingGenerator::new(storage.clone()));
        let orchestrator = Arc::new(orchestrator::Orchestrator::new(
            plugin_manager.clone(),
            context_manager.clone(),
            storage.clone(),
        ));
        
        Ok(Self {
            orchestrator,
            plugin_manager,
            context_manager,
            storage,
            intent_classifier,
            briefing_generator,
        })
    }
    
    pub async fn initialize(&self) -> Result<()> {
        self.plugin_manager.load_plugins().await?;
        self.orchestrator.initialize().await?;
        Ok(())
    }
    
    pub async fn shutdown(&self) -> Result<()> {
        self.orchestrator.shutdown().await?;
        self.plugin_manager.unload_all().await?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct CoreConfig {
    pub storage_config: storage::StorageConfig,
    pub database_config: database::DatabaseConfig,
    pub plugin_directory: String,
    pub max_concurrent_tasks: usize,
}

impl Default for CoreConfig {
    fn default() -> Self {
        Self {
            storage_config: storage::StorageConfig::default(),
            database_config: database::DatabaseConfig::default(),
            plugin_directory: "./plugins".to_string(),
            max_concurrent_tasks: 10,
        }
    }
}