# Plugin Architecture - Design and Implementation

## Overview

The plugin architecture enables extensible functionality for the Personal AI Assistant through secure WebAssembly (WASM) modules. This design allows for third-party integrations, custom workflows, and modular feature deployment while maintaining security, performance, and stability.

## Architecture Design

### Core Principles
1. **Security First**: Complete sandboxing of plugin code
2. **Performance**: Near-native execution speed with WASM
3. **Language Agnostic**: Support for Rust, C/C++, AssemblyScript, and more
4. **API Consistency**: Standardized plugin interface across all modules
5. **Hot Loading**: Dynamic plugin loading without system restart
6. **Resource Management**: Memory and CPU limits per plugin

## Plugin System Components

### 1. Plugin Runtime Engine

```toml
# crates/plugins/Cargo.toml
[package]
name = "plugins"
version = "0.1.0"
edition = "2021"

[dependencies]
wasmtime = "17.0"
wasmtime-wasi = "17.0"
tokio = { workspace = true }
serde = { workspace = true }
anyhow = { workspace = true }
uuid = { workspace = true }
sha2 = "0.10"
```

#### Plugin Manager (`crates/plugins/src/manager.rs`)

```rust
use wasmtime::{Engine, Module, Store, Instance, Func, Caller, Config, WasmParams, WasmResults};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, Dir};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};

pub struct PluginManager {
    engine: Engine,
    plugins: Arc<RwLock<HashMap<String, Plugin>>>,
    plugin_registry: Arc<PluginRegistry>,
    security_manager: Arc<SecurityManager>,
    resource_manager: Arc<ResourceManager>,
}

#[derive(Clone)]
pub struct Plugin {
    pub id: String,
    pub manifest: PluginManifest,
    pub module: Module,
    pub instance: Option<Instance>,
    pub store: Arc<Mutex<Store<PluginContext>>>,
    pub status: PluginStatus,
    pub loaded_at: chrono::DateTime<chrono::Utc>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub license: String,
    pub permissions: Vec<Permission>,
    pub api_version: String,
    pub dependencies: Vec<PluginDependency>,
    pub entry_points: Vec<EntryPoint>,
    pub resource_limits: ResourceLimits,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Permission {
    FileSystem { paths: Vec<String>, read_only: bool },
    Network { domains: Vec<String> },
    Database { tables: Vec<String>, operations: Vec<String> },
    KnowledgeBase { read: bool, write: bool },
    UserData { scopes: Vec<String> },
    SystemInfo,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginDependency {
    pub name: String,
    pub version_requirement: String,
    pub optional: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EntryPoint {
    pub name: String,
    pub function: String,
    pub description: String,
    pub parameters: Vec<ParameterDefinition>,
    pub return_type: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ParameterDefinition {
    pub name: String,
    pub param_type: String,
    pub description: String,
    pub required: bool,
    pub default_value: Option<serde_json::Value>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResourceLimits {
    pub max_memory_mb: u32,
    pub max_execution_time_ms: u32,
    pub max_file_operations: u32,
    pub max_network_requests: u32,
}

#[derive(Debug, Clone)]
pub enum PluginStatus {
    Loading,
    Loaded,
    Running,
    Stopped,
    Error(String),
}

pub struct PluginContext {
    pub wasi: WasiCtx,
    pub permissions: Vec<Permission>,
    pub resource_usage: ResourceUsage,
    pub plugin_id: String,
}

#[derive(Debug, Default)]
pub struct ResourceUsage {
    pub memory_used_mb: u32,
    pub execution_time_ms: u32,
    pub file_operations: u32,
    pub network_requests: u32,
}

impl PluginManager {
    pub async fn new() -> Result<Self, PluginError> {
        let mut config = Config::new();
        config.wasm_multi_memory(true);
        config.wasm_module_linking(true);
        config.async_support(true);
        
        let engine = Engine::new(&config)?;
        
        Ok(Self {
            engine,
            plugins: Arc::new(RwLock::new(HashMap::new())),
            plugin_registry: Arc::new(PluginRegistry::new()),
            security_manager: Arc::new(SecurityManager::new()),
            resource_manager: Arc::new(ResourceManager::new()),
        })
    }
    
    pub async fn load_plugin(&self, plugin_path: &Path, verify_signature: bool) -> Result<String, PluginError> {
        // Verify plugin signature if required
        if verify_signature {
            self.security_manager.verify_plugin_signature(plugin_path).await?;
        }
        
        // Load and parse manifest
        let manifest = self.load_plugin_manifest(plugin_path).await?;
        
        // Security validation
        self.security_manager.validate_permissions(&manifest.permissions)?;
        
        // Load WASM module
        let wasm_bytes = std::fs::read(plugin_path.join("plugin.wasm"))?;
        let module = Module::new(&self.engine, &wasm_bytes)?;
        
        // Create plugin context
        let wasi = WasiCtxBuilder::new()
            .inherit_stdio()
            .preopened_dir(
                Dir::open_ambient_dir(plugin_path, cap_std::ambient_authority())?,
                ".",
            )?
            .build();
        
        let plugin_context = PluginContext {
            wasi,
            permissions: manifest.permissions.clone(),
            resource_usage: ResourceUsage::default(),
            plugin_id: manifest.name.clone(),
        };
        
        let store = Store::new(&self.engine, plugin_context);
        
        let plugin = Plugin {
            id: manifest.name.clone(),
            manifest,
            module,
            instance: None,
            store: Arc::new(Mutex::new(store)),
            status: PluginStatus::Loaded,
            loaded_at: chrono::Utc::now(),
            last_error: None,
        };
        
        // Register plugin
        let plugin_id = plugin.id.clone();
        self.plugins.write().await.insert(plugin_id.clone(), plugin);
        
        Ok(plugin_id)
    }
    
    pub async fn instantiate_plugin(&self, plugin_id: &str) -> Result<(), PluginError> {
        let mut plugins = self.plugins.write().await;
        let plugin = plugins.get_mut(plugin_id)
            .ok_or_else(|| PluginError::PluginNotFound(plugin_id.to_string()))?;
        
        let mut store = plugin.store.lock().await;
        
        // Create import functions for the plugin
        let imports = self.create_plugin_imports(&mut *store, &plugin.manifest.permissions).await?;
        
        // Instantiate the module
        let instance = Instance::new(&mut *store, &plugin.module, &imports)?;
        
        // Call initialization function if it exists
        if let Ok(init_func) = instance.get_typed_func::<(), ()>(&mut *store, "_plugin_init") {
            init_func.call(&mut *store, ())?;
        }
        
        plugin.instance = Some(instance);
        plugin.status = PluginStatus::Running;
        
        Ok(())
    }
    
    pub async fn call_plugin_function<P, R>(
        &self,
        plugin_id: &str,
        function_name: &str,
        params: P,
    ) -> Result<R, PluginError>
    where
        P: WasmParams,
        R: WasmResults,
    {
        let plugins = self.plugins.read().await;
        let plugin = plugins.get(plugin_id)
            .ok_or_else(|| PluginError::PluginNotFound(plugin_id.to_string()))?;
        
        let instance = plugin.instance.as_ref()
            .ok_or_else(|| PluginError::PluginNotInstantiated(plugin_id.to_string()))?;
        
        let mut store = plugin.store.lock().await;
        
        // Check resource limits before execution
        self.resource_manager.check_limits(&store.data().resource_usage, &plugin.manifest.resource_limits)?;
        
        // Get the function and call it
        let func = instance.get_typed_func::<P, R>(&mut *store, function_name)
            .map_err(|_| PluginError::FunctionNotFound(function_name.to_string()))?;
        
        let start_time = std::time::Instant::now();
        let result = func.call(&mut *store, params)?;
        let execution_time = start_time.elapsed().as_millis() as u32;
        
        // Update resource usage
        store.data_mut().resource_usage.execution_time_ms += execution_time;
        
        Ok(result)
    }
    
    pub async fn unload_plugin(&self, plugin_id: &str) -> Result<(), PluginError> {
        let mut plugins = self.plugins.write().await;
        
        if let Some(plugin) = plugins.get(plugin_id) {
            // Call cleanup function if it exists
            if let Some(instance) = &plugin.instance {
                let mut store = plugin.store.lock().await;
                if let Ok(cleanup_func) = instance.get_typed_func::<(), ()>(&mut *store, "_plugin_cleanup") {
                    let _ = cleanup_func.call(&mut *store, ());
                }
            }
        }
        
        plugins.remove(plugin_id);
        Ok(())
    }
    
    async fn create_plugin_imports(
        &self,
        store: &mut Store<PluginContext>,
        permissions: &[Permission],
    ) -> Result<Vec<wasmtime::Extern>, PluginError> {
        let mut imports = Vec::new();
        
        // Add WASI imports
        let wasi_imports = wasmtime_wasi::add_to_linker(&mut wasmtime::Linker::new(&self.engine), |ctx| &mut ctx.wasi)?;
        
        // Add custom API imports based on permissions
        for permission in permissions {
            match permission {
                Permission::KnowledgeBase { read, write } => {
                    if *read {
                        let search_func = Func::wrap(
                            store,
                            |caller: Caller<'_, PluginContext>, query_ptr: u32, query_len: u32, results_ptr: u32| -> u32 {
                                // Implementation for knowledge base search
                                self.handle_knowledge_search(caller, query_ptr, query_len, results_ptr)
                            },
                        );
                        imports.push(search_func.into());
                    }
                    
                    if *write {
                        let store_func = Func::wrap(
                            store,
                            |caller: Caller<'_, PluginContext>, doc_ptr: u32, doc_len: u32| -> u32 {
                                // Implementation for knowledge base storage
                                self.handle_knowledge_store(caller, doc_ptr, doc_len)
                            },
                        );
                        imports.push(store_func.into());
                    }
                }
                
                Permission::Network { domains } => {
                    let http_func = Func::wrap(
                        store,
                        |caller: Caller<'_, PluginContext>, url_ptr: u32, url_len: u32, response_ptr: u32| -> u32 {
                            // Implementation for HTTP requests
                            self.handle_http_request(caller, url_ptr, url_len, response_ptr, domains)
                        },
                    );
                    imports.push(http_func.into());
                }
                
                _ => {
                    // Handle other permissions
                }
            }
        }
        
        Ok(imports)
    }
    
    async fn load_plugin_manifest(&self, plugin_path: &Path) -> Result<PluginManifest, PluginError> {
        let manifest_path = plugin_path.join("manifest.json");
        let manifest_content = std::fs::read_to_string(&manifest_path)?;
        let manifest: PluginManifest = serde_json::from_str(&manifest_content)?;
        Ok(manifest)
    }
}

// Host function implementations
impl PluginManager {
    fn handle_knowledge_search(
        &self,
        caller: Caller<'_, PluginContext>,
        query_ptr: u32,
        query_len: u32,
        results_ptr: u32,
    ) -> u32 {
        // Extract query from plugin memory
        let memory = caller.get_export("memory")
            .and_then(|e| e.into_memory())
            .unwrap();
        
        let data = memory.data(&caller);
        let query_bytes = &data[query_ptr as usize..(query_ptr + query_len) as usize];
        let query = String::from_utf8_lossy(query_bytes);
        
        // Perform knowledge base search
        // This would integrate with the actual knowledge base service
        let search_results = format!("{{\"results\": [{{\"title\": \"Sample\", \"content\": \"Sample content for query: {}\"}}]}}", query);
        
        // Write results back to plugin memory
        let results_bytes = search_results.as_bytes();
        let data_mut = memory.data_mut(&mut caller);
        data_mut[results_ptr as usize..results_ptr as usize + results_bytes.len()].copy_from_slice(results_bytes);
        
        results_bytes.len() as u32
    }
    
    fn handle_knowledge_store(
        &self,
        caller: Caller<'_, PluginContext>,
        doc_ptr: u32,
        doc_len: u32,
    ) -> u32 {
        // Implementation for storing documents in knowledge base
        1 // Success
    }
    
    fn handle_http_request(
        &self,
        caller: Caller<'_, PluginContext>,
        url_ptr: u32,
        url_len: u32,
        response_ptr: u32,
        allowed_domains: &[String],
    ) -> u32 {
        // Implementation for HTTP requests with domain validation
        1 // Success
    }
}
```

### 2. Plugin Security Manager

```rust
use sha2::{Sha256, Digest};
use std::path::Path;

pub struct SecurityManager {
    trusted_publishers: Vec<String>,
    signature_validator: SignatureValidator,
}

impl SecurityManager {
    pub fn new() -> Self {
        Self {
            trusted_publishers: vec![
                "personal-ai-assistant-official".to_string(),
                "trusted-partner-1".to_string(),
            ],
            signature_validator: SignatureValidator::new(),
        }
    }
    
    pub async fn verify_plugin_signature(&self, plugin_path: &Path) -> Result<(), SecurityError> {
        let signature_path = plugin_path.join("signature.sig");
        let wasm_path = plugin_path.join("plugin.wasm");
        
        if !signature_path.exists() {
            return Err(SecurityError::MissingSignature);
        }
        
        let signature = std::fs::read(&signature_path)?;
        let wasm_bytes = std::fs::read(&wasm_path)?;
        
        // Calculate hash of WASM file
        let mut hasher = Sha256::new();
        hasher.update(&wasm_bytes);
        let hash = hasher.finalize();
        
        // Verify signature
        self.signature_validator.verify(&hash, &signature)?;
        
        Ok(())
    }
    
    pub fn validate_permissions(&self, permissions: &[Permission]) -> Result<(), SecurityError> {
        for permission in permissions {
            match permission {
                Permission::FileSystem { paths, .. } => {
                    // Validate file system access
                    for path in paths {
                        if path.contains("..") || path.starts_with("/") {
                            return Err(SecurityError::InvalidFilePath(path.clone()));
                        }
                    }
                }
                
                Permission::Network { domains } => {
                    // Validate network access
                    for domain in domains {
                        if domain == "*" {
                            return Err(SecurityError::OverbroadNetworkAccess);
                        }
                    }
                }
                
                Permission::Database { operations, .. } => {
                    // Validate database operations
                    if operations.contains(&"DROP".to_string()) || operations.contains(&"DELETE".to_string()) {
                        return Err(SecurityError::DangerousDatabaseOperation);
                    }
                }
                
                _ => {
                    // Other permission validations
                }
            }
        }
        
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SecurityError {
    #[error("Plugin signature is missing")]
    MissingSignature,
    #[error("Invalid signature")]
    InvalidSignature,
    #[error("Invalid file path: {0}")]
    InvalidFilePath(String),
    #[error("Overly broad network access requested")]
    OverbroadNetworkAccess,
    #[error("Dangerous database operation requested")]
    DangerousDatabaseOperation,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

### 3. Plugin Development SDK

#### Rust Plugin Template

```rust
// plugin-template/src/lib.rs
use serde::{Deserialize, Serialize};

// Plugin API exports
#[no_mangle]
pub extern "C" fn _plugin_init() {
    // Plugin initialization
}

#[no_mangle]
pub extern "C" fn _plugin_cleanup() {
    // Plugin cleanup
}

#[no_mangle]
pub extern "C" fn plugin_info() -> *const u8 {
    let info = PluginInfo {
        name: "Sample Plugin".to_string(),
        version: "1.0.0".to_string(),
        description: "A sample plugin for demonstration".to_string(),
    };
    
    let json = serde_json::to_string(&info).unwrap();
    json.as_ptr()
}

#[no_mangle]
pub extern "C" fn process_text(input_ptr: *const u8, input_len: usize, output_ptr: *mut u8, output_len: usize) -> usize {
    unsafe {
        let input_slice = std::slice::from_raw_parts(input_ptr, input_len);
        let input_text = String::from_utf8_lossy(input_slice);
        
        // Process the text
        let result = format!("Processed: {}", input_text);
        let result_bytes = result.as_bytes();
        
        if result_bytes.len() <= output_len {
            let output_slice = std::slice::from_raw_parts_mut(output_ptr, output_len);
            output_slice[..result_bytes.len()].copy_from_slice(result_bytes);
            result_bytes.len()
        } else {
            0 // Buffer too small
        }
    }
}

// Host API imports
extern "C" {
    fn knowledge_search(query_ptr: *const u8, query_len: usize, results_ptr: *mut u8, results_len: usize) -> usize;
    fn knowledge_store(doc_ptr: *const u8, doc_len: usize) -> u32;
    fn http_request(url_ptr: *const u8, url_len: usize, response_ptr: *mut u8, response_len: usize) -> usize;
}

// Plugin helper functions
pub fn search_knowledge(query: &str) -> Result<String, PluginError> {
    let query_bytes = query.as_bytes();
    let mut results = vec![0u8; 4096]; // 4KB buffer
    
    unsafe {
        let result_len = knowledge_search(
            query_bytes.as_ptr(),
            query_bytes.len(),
            results.as_mut_ptr(),
            results.len(),
        );
        
        if result_len > 0 {
            results.truncate(result_len);
            String::from_utf8(results).map_err(|_| PluginError::InvalidUtf8)
        } else {
            Err(PluginError::SearchFailed)
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub description: String,
}

#[derive(Debug)]
pub enum PluginError {
    InvalidUtf8,
    SearchFailed,
    BufferTooSmall,
}

// Manifest template
const MANIFEST_JSON: &str = r#"
{
  "name": "sample-plugin",
  "version": "1.0.0",
  "description": "A sample plugin for demonstration",
  "author": "Plugin Developer",
  "license": "MIT",
  "api_version": "1.0",
  "permissions": [
    {
      "KnowledgeBase": {
        "read": true,
        "write": false
      }
    }
  ],
  "entry_points": [
    {
      "name": "process_text",
      "function": "process_text",
      "description": "Process input text and return modified version",
      "parameters": [
        {
          "name": "text",
          "param_type": "string",
          "description": "Text to process",
          "required": true
        }
      ],
      "return_type": "string"
    }
  ],
  "resource_limits": {
    "max_memory_mb": 64,
    "max_execution_time_ms": 5000,
    "max_file_operations": 100,
    "max_network_requests": 10
  }
}
"#;
```

### 4. Plugin Registry and Marketplace

```rust
pub struct PluginRegistry {
    storage: Arc<PluginStorage>,
    marketplace: Arc<PluginMarketplace>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub category: PluginCategory,
    pub tags: Vec<String>,
    pub download_url: String,
    pub signature_url: String,
    pub documentation_url: Option<String>,
    pub source_code_url: Option<String>,
    pub license: String,
    pub price: Option<Price>,
    pub ratings: PluginRatings,
    pub compatibility: CompatibilityInfo,
    pub last_updated: DateTime<Utc>,
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginCategory {
    Productivity,
    Health,
    Finance,
    Entertainment,
    Utilities,
    Development,
    Education,
    Social,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Price {
    pub amount: f64,
    pub currency: String,
    pub billing_type: BillingType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BillingType {
    OneTime,
    Monthly,
    Yearly,
    PayPerUse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginRatings {
    pub average_rating: f32,
    pub total_ratings: u32,
    pub distribution: [u32; 5], // 1-5 star distribution
}

impl PluginRegistry {
    pub async fn search_plugins(&self, query: &str, category: Option<PluginCategory>) -> Result<Vec<RegistryEntry>, RegistryError> {
        self.storage.search_plugins(query, category).await
    }
    
    pub async fn get_plugin_details(&self, plugin_id: &str) -> Result<RegistryEntry, RegistryError> {
        self.storage.get_plugin(plugin_id).await
    }
    
    pub async fn install_plugin(&self, plugin_id: &str, plugin_manager: &PluginManager) -> Result<String, RegistryError> {
        let plugin_entry = self.get_plugin_details(plugin_id).await?;
        
        // Download plugin
        let plugin_data = self.marketplace.download_plugin(&plugin_entry.download_url).await?;
        let signature = self.marketplace.download_signature(&plugin_entry.signature_url).await?;
        
        // Verify signature
        self.verify_plugin_integrity(&plugin_data, &signature)?;
        
        // Extract plugin to temporary directory
        let temp_dir = self.extract_plugin(&plugin_data).await?;
        
        // Load plugin through plugin manager
        let loaded_plugin_id = plugin_manager.load_plugin(&temp_dir, true).await?;
        
        // Update local registry
        self.storage.mark_plugin_installed(plugin_id).await?;
        
        Ok(loaded_plugin_id)
    }
}
```

### 5. Plugin Development Tools

#### Plugin Builder (`build-plugin.rs`)

```rust
use std::process::Command;
use std::path::Path;

pub struct PluginBuilder {
    project_path: std::path::PathBuf,
    target: String,
}

impl PluginBuilder {
    pub fn new<P: AsRef<Path>>(project_path: P) -> Self {
        Self {
            project_path: project_path.as_ref().to_path_buf(),
            target: "wasm32-wasi".to_string(),
        }
    }
    
    pub fn build(&self) -> Result<(), BuildError> {
        // Build the Rust project to WASM
        let output = Command::new("cargo")
            .args(&[
                "build",
                "--target", &self.target,
                "--release",
                "--manifest-path", &self.project_path.join("Cargo.toml").to_string_lossy(),
            ])
            .output()?;
        
        if !output.status.success() {
            return Err(BuildError::CompilationFailed(
                String::from_utf8_lossy(&output.stderr).to_string()
            ));
        }
        
        // Optimize WASM with wasm-opt
        let wasm_path = self.project_path
            .join("target")
            .join(&self.target)
            .join("release")
            .join(format!("{}.wasm", self.get_crate_name()?));
        
        let optimized_path = self.project_path.join("plugin.wasm");
        
        let output = Command::new("wasm-opt")
            .args(&[
                "-Oz", // Optimize for size
                &wasm_path.to_string_lossy(),
                "-o", &optimized_path.to_string_lossy(),
            ])
            .output()?;
        
        if !output.status.success() {
            return Err(BuildError::OptimizationFailed(
                String::from_utf8_lossy(&output.stderr).to_string()
            ));
        }
        
        // Validate manifest
        self.validate_manifest()?;
        
        // Create plugin package
        self.create_package()?;
        
        println!("Plugin built successfully: {}", optimized_path.display());
        Ok(())
    }
    
    fn get_crate_name(&self) -> Result<String, BuildError> {
        let cargo_toml = std::fs::read_to_string(self.project_path.join("Cargo.toml"))?;
        // Parse cargo.toml to get crate name
        // Simplified implementation
        for line in cargo_toml.lines() {
            if line.starts_with("name") {
                let name = line.split('=').nth(1)
                    .ok_or(BuildError::InvalidCargoToml)?
                    .trim()
                    .trim_matches('"');
                return Ok(name.to_string());
            }
        }
        Err(BuildError::CrateNameNotFound)
    }
    
    fn validate_manifest(&self) -> Result<(), BuildError> {
        let manifest_path = self.project_path.join("manifest.json");
        let manifest_content = std::fs::read_to_string(&manifest_path)?;
        let _manifest: PluginManifest = serde_json::from_str(&manifest_content)?;
        
        // Additional validation logic
        println!("Manifest validation passed");
        Ok(())
    }
    
    fn create_package(&self) -> Result<(), BuildError> {
        // Create a plugin package (ZIP file) containing:
        // - plugin.wasm
        // - manifest.json
        // - README.md (if exists)
        // - LICENSE (if exists)
        
        let package_path = self.project_path.join("plugin.zip");
        // ZIP creation logic would go here
        
        println!("Plugin package created: {}", package_path.display());
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("Compilation failed: {0}")]
    CompilationFailed(String),
    #[error("Optimization failed: {0}")]
    OptimizationFailed(String),
    #[error("Invalid Cargo.toml")]
    InvalidCargoToml,
    #[error("Crate name not found")]
    CrateNameNotFound,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
```

## API Integration

### Plugin Management Endpoints

```rust
pub fn create_plugin_routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/plugins", get(list_plugins).post(install_plugin))
        .route("/api/v1/plugins/:id", get(get_plugin).delete(uninstall_plugin))
        .route("/api/v1/plugins/:id/enable", post(enable_plugin))
        .route("/api/v1/plugins/:id/disable", post(disable_plugin))
        .route("/api/v1/plugins/:id/execute", post(execute_plugin))
        .route("/api/v1/plugins/registry/search", get(search_plugin_registry))
        .route("/api/v1/plugins/upload", post(upload_plugin))
}

#[derive(Deserialize)]
pub struct InstallPluginRequest {
    pub plugin_id: String,
    pub source: PluginSource,
    pub auto_enable: Option<bool>,
}

#[derive(Deserialize)]
pub enum PluginSource {
    Registry,
    Url(String),
    Upload,
}

pub async fn install_plugin(
    State(state): State<AppState>,
    Json(request): Json<InstallPluginRequest>,
) -> Result<Json<PluginInstallResult>, StatusCode> {
    let user_id = get_authenticated_user_id()?;
    
    // Check user permissions for plugin installation
    if !state.auth.check_permission(user_id, "install_plugins").await
        .map_err(|_| StatusCode::FORBIDDEN)? {
        return Err(StatusCode::FORBIDDEN);
    }
    
    let result = match request.source {
        PluginSource::Registry => {
            state.plugins.registry
                .install_plugin(&request.plugin_id, &state.plugins.manager)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        }
        PluginSource::Url(url) => {
            state.plugins.manager
                .install_from_url(&url)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        }
        PluginSource::Upload => {
            return Err(StatusCode::BAD_REQUEST); // Should use upload endpoint
        }
    };
    
    // Auto-enable if requested
    if request.auto_enable.unwrap_or(false) {
        state.plugins.manager
            .instantiate_plugin(&result)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    
    Ok(Json(PluginInstallResult {
        plugin_id: result,
        status: "installed".to_string(),
        enabled: request.auto_enable.unwrap_or(false),
    }))
}

#[derive(Serialize)]
pub struct PluginInstallResult {
    pub plugin_id: String,
    pub status: String,
    pub enabled: bool,
}
```

## Security Best Practices

### 1. Plugin Sandboxing

```rust
impl PluginSandbox {
    pub fn create_sandbox(&self, plugin_manifest: &PluginManifest) -> Result<SandboxConfig, SecurityError> {
        SandboxConfig {
            // Memory limits
            max_memory: plugin_manifest.resource_limits.max_memory_mb * 1024 * 1024,
            
            // File system isolation
            allowed_paths: plugin_manifest.permissions.iter()
                .filter_map(|p| match p {
                    Permission::FileSystem { paths, .. } => Some(paths.clone()),
                    _ => None,
                })
                .flatten()
                .collect(),
            
            // Network restrictions
            allowed_domains: plugin_manifest.permissions.iter()
                .filter_map(|p| match p {
                    Permission::Network { domains } => Some(domains.clone()),
                    _ => None,
                })
                .flatten()
                .collect(),
            
            // Execution time limits
            max_execution_time: Duration::from_millis(
                plugin_manifest.resource_limits.max_execution_time_ms as u64
            ),
        }
    }
}
```

### 2. Plugin Verification

```rust
pub struct PluginVerifier {
    trusted_signers: Vec<PublicKey>,
}

impl PluginVerifier {
    pub fn verify_plugin(&self, plugin_data: &[u8], signature: &[u8]) -> Result<(), VerificationError> {
        // Verify digital signature
        let hash = sha2::Sha256::digest(plugin_data);
        
        for public_key in &self.trusted_signers {
            if self.verify_signature(&hash, signature, public_key)? {
                return Ok(());
            }
        }
        
        Err(VerificationError::InvalidSignature)
    }
    
    pub fn scan_for_malware(&self, wasm_bytes: &[u8]) -> Result<ScanResult, VerificationError> {
        // Static analysis of WASM for malicious patterns
        let module = walrus::Module::from_buffer(wasm_bytes)?;
        
        // Check for suspicious imports
        for import in module.imports.iter() {
            if self.is_suspicious_import(&import.name) {
                return Ok(ScanResult::Suspicious {
                    reason: format!("Suspicious import: {}", import.name),
                });
            }
        }
        
        // Check for obfuscation patterns
        if self.detect_obfuscation(&module) {
            return Ok(ScanResult::Suspicious {
                reason: "Code obfuscation detected".to_string(),
            });
        }
        
        Ok(ScanResult::Clean)
    }
}
```

## Performance Optimization

### 1. Plugin Caching

```rust
pub struct PluginCache {
    compiled_modules: Arc<RwLock<HashMap<String, Module>>>,
    instance_pool: Arc<RwLock<HashMap<String, Vec<Instance>>>>,
}

impl PluginCache {
    pub async fn get_or_compile_module(&self, plugin_id: &str, wasm_bytes: &[u8], engine: &Engine) -> Result<Module, PluginError> {
        {
            let cache = self.compiled_modules.read().await;
            if let Some(module) = cache.get(plugin_id) {
                return Ok(module.clone());
            }
        }
        
        // Compile module
        let module = Module::new(engine, wasm_bytes)?;
        
        // Cache it
        let mut cache = self.compiled_modules.write().await;
        cache.insert(plugin_id.to_string(), module.clone());
        
        Ok(module)
    }
}
```

### 2. Resource Monitoring

```rust
pub struct ResourceMonitor {
    metrics: Arc<RwLock<HashMap<String, PluginMetrics>>>,
}

#[derive(Debug, Default)]
pub struct PluginMetrics {
    pub memory_usage: u64,
    pub cpu_time: Duration,
    pub function_calls: u64,
    pub network_requests: u64,
    pub last_activity: DateTime<Utc>,
}

impl ResourceMonitor {
    pub async fn track_resource_usage(&self, plugin_id: &str, usage: ResourceUsage) {
        let mut metrics = self.metrics.write().await;
        let plugin_metrics = metrics.entry(plugin_id.to_string()).or_default();
        
        plugin_metrics.memory_usage = usage.memory_used_mb as u64 * 1024 * 1024;
        plugin_metrics.cpu_time += Duration::from_millis(usage.execution_time_ms as u64);
        plugin_metrics.last_activity = Utc::now();
    }
    
    pub async fn get_resource_report(&self) -> HashMap<String, PluginMetrics> {
        self.metrics.read().await.clone()
    }
}
```

This plugin architecture provides a secure, performant, and extensible foundation for the Personal AI Assistant, enabling third-party developers to create powerful integrations while maintaining system security and stability.