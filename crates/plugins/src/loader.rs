use crate::{WasmPluginMetadata, ResourceLimits, WasmRuntime, RuntimeConfig};
use rusty_ai_common::{Result, AssistantError};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tokio::fs;
use tracing::{info, warn, error, debug, instrument};
use sha2::{Sha256, Digest};

/// Plugin loader for discovering and loading WebAssembly plugins
pub struct PluginLoader {
    plugin_directory: PathBuf,
    runtime: WasmRuntime,
    loaded_plugins: HashMap<String, LoadedPlugin>,
    plugin_registry: PluginRegistry,
}

/// Information about a loaded plugin
#[derive(Debug, Clone)]
pub struct LoadedPlugin {
    pub metadata: WasmPluginMetadata,
    pub file_path: PathBuf,
    pub checksum: String,
    pub loaded_at: SystemTime,
    pub size: u64,
    pub limits: ResourceLimits,
}

/// Plugin registry for tracking available plugins
#[derive(Debug, Clone)]
pub struct PluginRegistry {
    plugins: HashMap<String, PluginEntry>,
}

/// Entry in the plugin registry
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginEntry {
    pub id: String,
    pub name: String,
    pub version: String,
    pub file_path: PathBuf,
    pub checksum: String,
    pub metadata: WasmPluginMetadata,
    pub status: PluginStatus,
    pub last_updated: SystemTime,
}

/// Plugin status in the registry
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum PluginStatus {
    Available,
    Loaded,
    Error(String),
    Disabled,
}

/// Plugin discovery configuration
#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    /// Search subdirectories
    pub recursive: bool,
    /// File extensions to search for
    pub extensions: Vec<String>,
    /// Maximum file size (in bytes)
    pub max_file_size: u64,
    /// Verify plugin signatures
    pub verify_signatures: bool,
    /// Auto-reload on file changes
    pub auto_reload: bool,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            recursive: true,
            extensions: vec!["wasm".to_string(), "wat".to_string()],
            max_file_size: 100 * 1024 * 1024, // 100MB
            verify_signatures: false,
            auto_reload: false,
        }
    }
}

impl PluginLoader {
    /// Create a new plugin loader
    #[instrument]
    pub fn new(plugin_directory: impl AsRef<Path>, runtime_config: RuntimeConfig) -> Result<Self> {
        let runtime = WasmRuntime::new(runtime_config)?;
        
        Ok(Self {
            plugin_directory: plugin_directory.as_ref().to_path_buf(),
            runtime,
            loaded_plugins: HashMap::new(),
            plugin_registry: PluginRegistry::new(),
        })
    }
    
    /// Discover all plugins in the plugin directory
    #[instrument(skip(self))]
    pub async fn discover_plugins(&mut self, config: DiscoveryConfig) -> Result<Vec<PluginEntry>> {
        info!("Discovering plugins in: {:?}", self.plugin_directory);
        
        if !self.plugin_directory.exists() {
            warn!("Plugin directory does not exist: {:?}", self.plugin_directory);
            return Ok(vec![]);
        }
        
        let mut discovered_plugins = Vec::new();
        
        if config.recursive {
            self.discover_recursive(&self.plugin_directory.clone(), &config, &mut discovered_plugins).await?;
        } else {
            self.discover_directory(&self.plugin_directory, &config, &mut discovered_plugins).await?;
        }
        
        // Update registry
        for plugin in &discovered_plugins {
            self.plugin_registry.add_plugin(plugin.clone());
        }
        
        info!("Discovered {} plugins", discovered_plugins.len());
        Ok(discovered_plugins)
    }
    
    /// Discover plugins recursively
    async fn discover_recursive(
        &self,
        dir: &Path,
        config: &DiscoveryConfig,
        discovered: &mut Vec<PluginEntry>,
    ) -> Result<()> {
        let mut entries = fs::read_dir(dir).await
            .map_err(|e| AssistantError::Plugin(format!("Failed to read directory: {}", e)))?;
        
        while let Some(entry) = entries.next_entry().await
            .map_err(|e| AssistantError::Plugin(format!("Failed to read directory entry: {}", e)))? {
            
            let path = entry.path();
            
            if path.is_dir() {
                self.discover_recursive(&path, config, discovered).await?;
            } else if self.is_plugin_file(&path, config) {
                if let Some(plugin_entry) = self.analyze_plugin_file(&path, config).await? {
                    discovered.push(plugin_entry);
                }
            }
        }
        
        Ok(())
    }
    
    /// Discover plugins in a single directory
    async fn discover_directory(
        &self,
        dir: &Path,
        config: &DiscoveryConfig,
        discovered: &mut Vec<PluginEntry>,
    ) -> Result<()> {
        let mut entries = fs::read_dir(dir).await
            .map_err(|e| AssistantError::Plugin(format!("Failed to read directory: {}", e)))?;
        
        while let Some(entry) = entries.next_entry().await
            .map_err(|e| AssistantError::Plugin(format!("Failed to read directory entry: {}", e)))? {
            
            let path = entry.path();
            
            if path.is_file() && self.is_plugin_file(&path, config) {
                if let Some(plugin_entry) = self.analyze_plugin_file(&path, config).await? {
                    discovered.push(plugin_entry);
                }
            }
        }
        
        Ok(())
    }
    
    /// Check if a file is a potential plugin file
    fn is_plugin_file(&self, path: &Path, config: &DiscoveryConfig) -> bool {
        if let Some(extension) = path.extension() {
            if let Some(ext_str) = extension.to_str() {
                return config.extensions.iter().any(|e| e == ext_str);
            }
        }
        false
    }
    
    /// Analyze a plugin file and create a plugin entry
    async fn analyze_plugin_file(&self, path: &Path, config: &DiscoveryConfig) -> Result<Option<PluginEntry>> {
        debug!("Analyzing plugin file: {:?}", path);
        
        // Check file size
        let metadata = fs::metadata(path).await
            .map_err(|e| AssistantError::Plugin(format!("Failed to read file metadata: {}", e)))?;
        
        if metadata.len() > config.max_file_size {
            warn!("Plugin file too large: {:?} ({} bytes)", path, metadata.len());
            return Ok(None);
        }
        
        // Read and validate file
        let wasm_bytes = fs::read(path).await
            .map_err(|e| AssistantError::Plugin(format!("Failed to read plugin file: {}", e)))?;
        
        // Calculate checksum
        let checksum = self.calculate_checksum(&wasm_bytes);
        
        // Try to extract metadata (this is a simplified approach)
        let plugin_metadata = self.extract_plugin_metadata(&wasm_bytes).await?;
        
        let plugin_entry = PluginEntry {
            id: plugin_metadata.id.clone(),
            name: plugin_metadata.name.clone(),
            version: plugin_metadata.version.clone(),
            file_path: path.to_path_buf(),
            checksum,
            metadata: plugin_metadata,
            status: PluginStatus::Available,
            last_updated: metadata.modified().unwrap_or(SystemTime::now()),
        };
        
        Ok(Some(plugin_entry))
    }
    
    /// Extract metadata from WebAssembly binary
    async fn extract_plugin_metadata(&self, wasm_bytes: &[u8]) -> Result<WasmPluginMetadata> {
        // In a production system, you would:
        // 1. Parse the WebAssembly binary
        // 2. Look for custom sections with metadata
        // 3. Use component model introspection
        // 4. Call metadata export functions
        
        // For this implementation, we'll use defaults with basic validation
        
        // Validate it's a valid WebAssembly module
        match wasmtime::Module::new(&wasmtime::Engine::default(), wasm_bytes) {
            Ok(_) => {
                // Create default metadata - in production this would be extracted from the module
                Ok(WasmPluginMetadata {
                    id: format!("plugin_{}", uuid::Uuid::new_v4()),
                    name: "WebAssembly Plugin".to_string(),
                    version: "0.1.0".to_string(),
                    description: "A WebAssembly plugin".to_string(),
                    author: "Unknown".to_string(),
                    license: "Unknown".to_string(),
                    capabilities: vec!["general".to_string()],
                    dependencies: vec![],
                    api_version: "1.0".to_string(),
                    checksum: self.calculate_checksum(wasm_bytes),
                })
            }
            Err(e) => Err(AssistantError::Plugin(format!("Invalid WebAssembly module: {}", e))),
        }
    }
    
    /// Calculate SHA-256 checksum of plugin bytes
    fn calculate_checksum(&self, bytes: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        hex::encode(hasher.finalize())
    }
    
    /// Load a specific plugin by ID
    #[instrument(skip(self))]
    pub async fn load_plugin(&mut self, plugin_id: &str, limits: Option<ResourceLimits>) -> Result<()> {
        info!("Loading plugin: {}", plugin_id);
        
        let plugin_entry = self.plugin_registry.get_plugin(plugin_id)
            .ok_or_else(|| AssistantError::NotFound(format!("Plugin not found: {}", plugin_id)))?;
        
        // Check if already loaded
        if self.loaded_plugins.contains_key(plugin_id) {
            warn!("Plugin already loaded: {}", plugin_id);
            return Ok(());
        }
        
        // Read plugin file
        let wasm_bytes = fs::read(&plugin_entry.file_path).await
            .map_err(|e| AssistantError::Plugin(format!("Failed to read plugin file: {}", e)))?;
        
        // Verify checksum
        let current_checksum = self.calculate_checksum(&wasm_bytes);
        if current_checksum != plugin_entry.checksum {
            return Err(AssistantError::Plugin(
                format!("Plugin checksum mismatch for {}: expected {}, got {}", 
                    plugin_id, plugin_entry.checksum, current_checksum)
            ));
        }
        
        // Use provided limits or defaults
        let resource_limits = limits.unwrap_or_default();
        
        // Load plugin into runtime
        let _instance = self.runtime.get_instance(plugin_id, &wasm_bytes, resource_limits.clone()).await?;
        
        // Create loaded plugin record
        let loaded_plugin = LoadedPlugin {
            metadata: plugin_entry.metadata.clone(),
            file_path: plugin_entry.file_path.clone(),
            checksum: current_checksum,
            loaded_at: SystemTime::now(),
            size: wasm_bytes.len() as u64,
            limits: resource_limits,
        };
        
        self.loaded_plugins.insert(plugin_id.to_string(), loaded_plugin);
        self.plugin_registry.update_status(plugin_id, PluginStatus::Loaded);
        
        info!("Plugin loaded successfully: {}", plugin_id);
        Ok(())
    }
    
    /// Unload a plugin
    #[instrument(skip(self))]
    pub async fn unload_plugin(&mut self, plugin_id: &str) -> Result<()> {
        info!("Unloading plugin: {}", plugin_id);
        
        if let Some(_loaded_plugin) = self.loaded_plugins.remove(plugin_id) {
            self.plugin_registry.update_status(plugin_id, PluginStatus::Available);
            info!("Plugin unloaded successfully: {}", plugin_id);
        } else {
            warn!("Plugin not loaded: {}", plugin_id);
        }
        
        Ok(())
    }
    
    /// Get list of loaded plugins
    pub fn get_loaded_plugins(&self) -> Vec<&LoadedPlugin> {
        self.loaded_plugins.values().collect()
    }
    
    /// Get plugin registry
    pub fn get_registry(&self) -> &PluginRegistry {
        &self.plugin_registry
    }
    
    /// Reload a plugin (unload and load again)
    pub async fn reload_plugin(&mut self, plugin_id: &str) -> Result<()> {
        info!("Reloading plugin: {}", plugin_id);
        
        let limits = self.loaded_plugins.get(plugin_id)
            .map(|p| p.limits.clone());
        
        self.unload_plugin(plugin_id).await?;
        self.load_plugin(plugin_id, limits).await?;
        
        Ok(())
    }
    
    /// Get runtime reference
    pub fn runtime(&self) -> &WasmRuntime {
        &self.runtime
    }
}

impl PluginRegistry {
    /// Create a new plugin registry
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
        }
    }
    
    /// Add a plugin to the registry
    pub fn add_plugin(&mut self, plugin: PluginEntry) {
        self.plugins.insert(plugin.id.clone(), plugin);
    }
    
    /// Get a plugin by ID
    pub fn get_plugin(&self, plugin_id: &str) -> Option<&PluginEntry> {
        self.plugins.get(plugin_id)
    }
    
    /// Update plugin status
    pub fn update_status(&mut self, plugin_id: &str, status: PluginStatus) {
        if let Some(plugin) = self.plugins.get_mut(plugin_id) {
            plugin.status = status;
        }
    }
    
    /// List all plugins
    pub fn list_plugins(&self) -> Vec<&PluginEntry> {
        self.plugins.values().collect()
    }
    
    /// List plugins by status
    pub fn list_plugins_by_status(&self, status: PluginStatus) -> Vec<&PluginEntry> {
        self.plugins.values()
            .filter(|plugin| plugin.status == status)
            .collect()
    }
    
    /// Remove a plugin from registry
    pub fn remove_plugin(&mut self, plugin_id: &str) -> Option<PluginEntry> {
        self.plugins.remove(plugin_id)
    }
    
    /// Clear all plugins
    pub fn clear(&mut self) {
        self.plugins.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_plugin_loader_creation() {
        let temp_dir = tempdir().unwrap();
        let runtime_config = RuntimeConfig::default();
        
        let loader = PluginLoader::new(temp_dir.path(), runtime_config).unwrap();
        assert_eq!(loader.get_loaded_plugins().len(), 0);
    }
    
    #[tokio::test]
    async fn test_discovery_config_default() {
        let config = DiscoveryConfig::default();
        assert!(config.recursive);
        assert!(config.extensions.contains(&"wasm".to_string()));
    }
    
    #[test]
    fn test_plugin_registry() {
        let mut registry = PluginRegistry::new();
        assert_eq!(registry.list_plugins().len(), 0);
        
        registry.update_status("test", PluginStatus::Loaded);
        // Status update on non-existent plugin should not panic
    }
    
    #[test]
    fn test_checksum_calculation() {
        let temp_dir = tempdir().unwrap();
        let runtime_config = RuntimeConfig::default();
        let loader = PluginLoader::new(temp_dir.path(), runtime_config).unwrap();
        
        let test_data = b"test data";
        let checksum1 = loader.calculate_checksum(test_data);
        let checksum2 = loader.calculate_checksum(test_data);
        
        assert_eq!(checksum1, checksum2);
        assert_eq!(checksum1.len(), 64); // SHA-256 produces 64 character hex string
    }
}