use rusty_ai_common::{Result, AssistantError};
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Mutex};
use tracing::{info, warn, error, debug, instrument};
use wasmtime::*;
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiView};

pub mod runtime;
pub mod loader;
pub mod security;
pub mod communication;
pub mod example_plugin;

pub use runtime::*;
pub use loader::*;
pub use security::*;
pub use communication::*;

/// Plugin execution limits and resource constraints
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// Maximum memory allocation in bytes (default: 64MB)
    pub max_memory: u64,
    /// Maximum execution time for a single operation (default: 30s)
    pub max_execution_time: Duration,
    /// Maximum number of WASI file descriptors (default: 10)
    pub max_file_descriptors: u32,
    /// Maximum number of network connections (default: 5)
    pub max_network_connections: u32,
    /// CPU time limit per operation (default: 10s)
    pub cpu_time_limit: Duration,
    /// Maximum fuel units for Wasmtime execution (default: 1M)
    pub max_fuel: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory: 64 * 1024 * 1024, // 64MB
            max_execution_time: Duration::from_secs(30),
            max_file_descriptors: 10,
            max_network_connections: 5,
            cpu_time_limit: Duration::from_secs(10),
            max_fuel: 1_000_000,
        }
    }
}

/// Plugin metadata extracted from WASM module
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WasmPluginMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub license: String,
    pub capabilities: Vec<String>,
    pub dependencies: Vec<String>,
    pub api_version: String,
    pub checksum: String,
}

/// Plugin execution context
#[derive(Debug)]
pub struct PluginContext {
    pub user_id: String,
    pub session_id: String,
    pub request_id: String,
    pub metadata: HashMap<String, String>,
    pub started_at: Instant,
}

/// WebAssembly plugin trait for sandboxed execution
#[async_trait]
pub trait WasmPlugin: Send + Sync {
    /// Get plugin metadata
    fn metadata(&self) -> &WasmPluginMetadata;
    
    /// Initialize the plugin with configuration
    async fn initialize(&mut self, config: serde_json::Value) -> Result<()>;
    
    /// Execute a plugin function with input data
    async fn execute(&self, function: &str, input: &[u8], context: &PluginContext) -> Result<Vec<u8>>;
    
    /// Check if plugin can handle a specific capability
    fn can_handle(&self, capability: &str) -> bool;
    
    /// Get plugin health status
    async fn health_check(&self) -> Result<PluginHealth>;
    
    /// Cleanup resources
    async fn cleanup(&mut self) -> Result<()>;
}

/// Plugin health status
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginHealth {
    pub status: HealthStatus,
    pub message: Option<String>,
    pub last_check: chrono::DateTime<chrono::Utc>,
    pub execution_count: u64,
    pub error_count: u64,
    pub average_execution_time: Duration,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

/// WASI context for plugin execution
pub struct PluginWasiCtx {
    wasi: WasiCtx,
    limits: ResourceLimits,
}

impl PluginWasiCtx {
    pub fn new(limits: ResourceLimits) -> Result<Self> {
        let wasi = WasiCtxBuilder::new()
            .inherit_stdio()
            .build();
            
        Ok(Self { wasi, limits })
    }
}

impl WasiView for PluginWasiCtx {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi
    }
}

/// Main WebAssembly plugin manager
pub struct WasmPluginManager {
    engine: Engine,
    plugins: Arc<RwLock<HashMap<String, Arc<Mutex<Box<dyn WasmPlugin>>>>>>,
    default_limits: ResourceLimits,
    plugin_directory: PathBuf,
}

impl WasmPluginManager {
    /// Create a new WebAssembly plugin manager
    pub fn new(plugin_directory: impl AsRef<Path>) -> Result<Self> {
        let mut config = Config::new();
        
        // Enable WebAssembly features
        config.wasm_component_model(true);
        config.async_support(true);
        config.consume_fuel(true);
        
        // Security settings
        config.wasm_multi_memory(false);
        config.wasm_threads(false);
        config.wasm_reference_types(false);
        config.wasm_simd(false);
        config.wasm_bulk_memory(false);
        
        let engine = Engine::new(&config)
            .map_err(|e| AssistantError::Plugin(format!("Failed to create Wasmtime engine: {}", e)))?;
        
        Ok(Self {
            engine,
            plugins: Arc::new(RwLock::new(HashMap::new())),
            default_limits: ResourceLimits::default(),
            plugin_directory: plugin_directory.as_ref().to_path_buf(),
        })
    }
    
    /// Load a plugin from a WASM file
    #[instrument(skip(self, wasm_bytes))]
    pub async fn load_plugin(&self, plugin_id: &str, wasm_bytes: &[u8]) -> Result<()> {
        info!("Loading WebAssembly plugin: {}", plugin_id);
        
        // Validate plugin before loading
        self.validate_plugin(wasm_bytes).await?;
        
        // Create plugin instance
        let plugin = WasmPluginInstance::new(
            &self.engine,
            wasm_bytes,
            self.default_limits.clone(),
        ).await?;
        
        // Register plugin
        let mut plugins = self.plugins.write().await;
        plugins.insert(plugin_id.to_string(), Arc::new(Mutex::new(Box::new(plugin))));
        
        info!("Plugin loaded successfully: {}", plugin_id);
        Ok(())
    }
    
    /// Load a plugin from file
    #[instrument(skip(self))]
    pub async fn load_plugin_from_file(&self, plugin_id: &str, wasm_path: impl AsRef<Path>) -> Result<()> {
        let wasm_bytes = tokio::fs::read(wasm_path.as_ref()).await
            .map_err(|e| AssistantError::Plugin(format!("Failed to read plugin file: {}", e)))?;
        
        self.load_plugin(plugin_id, &wasm_bytes).await
    }
    
    /// Execute a plugin function
    #[instrument(skip(self, input))]
    pub async fn execute_plugin(
        &self,
        plugin_id: &str,
        function: &str,
        input: &[u8],
        context: PluginContext,
    ) -> Result<Vec<u8>> {
        let plugins = self.plugins.read().await;
        
        let plugin = plugins
            .get(plugin_id)
            .ok_or_else(|| AssistantError::NotFound(format!("Plugin not found: {}", plugin_id)))?;
        
        let plugin_guard = plugin.lock().await;
        
        // Execute with timeout
        let execution_future = plugin_guard.execute(function, input, &context);
        
        match tokio::time::timeout(self.default_limits.max_execution_time, execution_future).await {
            Ok(result) => result,
            Err(_) => Err(AssistantError::Plugin(
                format!("Plugin execution timeout: {}", plugin_id)
            )),
        }
    }
    
    /// Get plugin metadata
    pub async fn get_plugin_metadata(&self, plugin_id: &str) -> Result<WasmPluginMetadata> {
        let plugins = self.plugins.read().await;
        
        let plugin = plugins
            .get(plugin_id)
            .ok_or_else(|| AssistantError::NotFound(format!("Plugin not found: {}", plugin_id)))?;
        
        let plugin_guard = plugin.lock().await;
        Ok(plugin_guard.metadata().clone())
    }
    
    /// List all loaded plugins
    pub async fn list_plugins(&self) -> Vec<String> {
        let plugins = self.plugins.read().await;
        plugins.keys().cloned().collect()
    }
    
    /// Unload a plugin
    #[instrument(skip(self))]
    pub async fn unload_plugin(&self, plugin_id: &str) -> Result<()> {
        info!("Unloading plugin: {}", plugin_id);
        
        let mut plugins = self.plugins.write().await;
        
        if let Some(plugin) = plugins.remove(plugin_id) {
            let mut plugin_guard = plugin.lock().await;
            plugin_guard.cleanup().await?;
            info!("Plugin unloaded: {}", plugin_id);
        }
        
        Ok(())
    }
    
    /// Perform health check on all plugins
    pub async fn health_check_all(&self) -> HashMap<String, PluginHealth> {
        let mut results = HashMap::new();
        let plugins = self.plugins.read().await;
        
        for (id, plugin) in plugins.iter() {
            let plugin_guard = plugin.lock().await;
            match plugin_guard.health_check().await {
                Ok(health) => {
                    results.insert(id.clone(), health);
                }
                Err(e) => {
                    warn!("Health check failed for plugin {}: {}", id, e);
                    results.insert(id.clone(), PluginHealth {
                        status: HealthStatus::Unhealthy,
                        message: Some(e.to_string()),
                        last_check: chrono::Utc::now(),
                        execution_count: 0,
                        error_count: 1,
                        average_execution_time: Duration::from_secs(0),
                    });
                }
            }
        }
        
        results
    }
    
    /// Validate plugin security and structure
    async fn validate_plugin(&self, wasm_bytes: &[u8]) -> Result<()> {
        // Basic validation - check if it's valid WebAssembly
        Module::new(&self.engine, wasm_bytes)
            .map_err(|e| AssistantError::Plugin(format!("Invalid WebAssembly module: {}", e)))?;
        
        // Additional security validations can be added here
        // - Check for prohibited imports
        // - Validate exports
        // - Scan for malicious patterns
        
        Ok(())
    }
    
    /// Set resource limits for plugin execution
    pub fn set_default_limits(&mut self, limits: ResourceLimits) {
        self.default_limits = limits;
    }
    
    /// Get current resource limits
    pub fn get_default_limits(&self) -> &ResourceLimits {
        &self.default_limits
    }
}

/// Concrete WebAssembly plugin instance
pub struct WasmPluginInstance {
    store: Store<PluginWasiCtx>,
    instance: Instance,
    metadata: WasmPluginMetadata,
    execution_stats: ExecutionStats,
}

#[derive(Debug)]
struct ExecutionStats {
    execution_count: u64,
    error_count: u64,
    total_execution_time: Duration,
}

impl WasmPluginInstance {
    /// Create a new WebAssembly plugin instance
    pub async fn new(
        engine: &Engine,
        wasm_bytes: &[u8],
        limits: ResourceLimits,
    ) -> Result<Self> {
        let module = Module::new(engine, wasm_bytes)
            .map_err(|e| AssistantError::Plugin(format!("Failed to compile module: {}", e)))?;
        
        let wasi_ctx = PluginWasiCtx::new(limits.clone())?;
        let mut store = Store::new(engine, wasi_ctx);
        
        // Set fuel limit for execution control
        store.set_fuel(limits.max_fuel)
            .map_err(|e| AssistantError::Plugin(format!("Failed to set fuel: {}", e)))?;
        
        // Enable epoch interruption for timeout handling
        store.set_epoch_deadline(1);
        
        let mut linker = Linker::new(engine);
        wasmtime_wasi::add_to_linker_async(&mut linker)
            .map_err(|e| AssistantError::Plugin(format!("Failed to add WASI to linker: {}", e)))?;
        
        let instance = linker.instantiate_async(&mut store, &module).await
            .map_err(|e| AssistantError::Plugin(format!("Failed to instantiate module: {}", e)))?;
        
        // Extract metadata from the plugin
        let metadata = Self::extract_metadata(&mut store, &instance).await?;
        
        Ok(Self {
            store,
            instance,
            metadata,
            execution_stats: ExecutionStats {
                execution_count: 0,
                error_count: 0,
                total_execution_time: Duration::from_secs(0),
            },
        })
    }
    
    /// Extract metadata from the WebAssembly module
    async fn extract_metadata(
        store: &mut Store<PluginWasiCtx>,
        instance: &Instance,
    ) -> Result<WasmPluginMetadata> {
        // Try to call the metadata export function
        if let Ok(metadata_func) = instance.get_typed_func::<(), i32>(store, "get_metadata") {
            // This is a simplified approach - in production you'd have a more robust
            // metadata extraction system using component model or memory exports
            Ok(WasmPluginMetadata {
                id: "unknown".to_string(),
                name: "WebAssembly Plugin".to_string(),
                version: "0.1.0".to_string(),
                description: "A WebAssembly plugin".to_string(),
                author: "Unknown".to_string(),
                license: "Unknown".to_string(),
                capabilities: vec![],
                dependencies: vec![],
                api_version: "1.0".to_string(),
                checksum: "".to_string(),
            })
        } else {
            Err(AssistantError::Plugin("Plugin missing metadata export".to_string()))
        }
    }
}

#[async_trait]
impl WasmPlugin for WasmPluginInstance {
    fn metadata(&self) -> &WasmPluginMetadata {
        &self.metadata
    }
    
    async fn initialize(&mut self, _config: serde_json::Value) -> Result<()> {
        // Call plugin initialization function if available
        if let Ok(init_func) = self.instance.get_typed_func::<(), ()>(&mut self.store, "initialize") {
            init_func.call_async(&mut self.store, ()).await
                .map_err(|e| AssistantError::Plugin(format!("Plugin initialization failed: {}", e)))?;
        }
        Ok(())
    }
    
    async fn execute(&self, function: &str, input: &[u8], _context: &PluginContext) -> Result<Vec<u8>> {
        // This is a simplified implementation
        // In production, you'd implement proper function calling with input/output handling
        
        let start_time = Instant::now();
        
        // For now, return empty success response
        let execution_time = start_time.elapsed();
        
        debug!("Plugin function '{}' executed in {:?}", function, execution_time);
        
        Ok(vec![])
    }
    
    fn can_handle(&self, capability: &str) -> bool {
        self.metadata.capabilities.contains(&capability.to_string())
    }
    
    async fn health_check(&self) -> Result<PluginHealth> {
        Ok(PluginHealth {
            status: HealthStatus::Healthy,
            message: None,
            last_check: chrono::Utc::now(),
            execution_count: self.execution_stats.execution_count,
            error_count: self.execution_stats.error_count,
            average_execution_time: if self.execution_stats.execution_count > 0 {
                self.execution_stats.total_execution_time / self.execution_stats.execution_count as u32
            } else {
                Duration::from_secs(0)
            },
        })
    }
    
    async fn cleanup(&mut self) -> Result<()> {
        // Cleanup resources
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_plugin_manager_creation() {
        let temp_dir = tempdir().unwrap();
        let manager = WasmPluginManager::new(temp_dir.path()).unwrap();
        assert_eq!(manager.list_plugins().await.len(), 0);
    }
    
    #[tokio::test]
    async fn test_resource_limits_default() {
        let limits = ResourceLimits::default();
        assert_eq!(limits.max_memory, 64 * 1024 * 1024);
        assert_eq!(limits.max_execution_time, Duration::from_secs(30));
    }
}