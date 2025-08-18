use crate::{WasmPlugin, WasmPluginMetadata, PluginContext, PluginHealth, HealthStatus, ResourceLimits};
use rusty_ai_common::{Result, AssistantError};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{info, debug, warn, error, instrument};

/// Example WebAssembly plugin demonstrating the plugin interface
pub struct ExampleWasmPlugin {
    metadata: WasmPluginMetadata,
    config: Arc<RwLock<Option<serde_json::Value>>>,
    state: Arc<RwLock<PluginState>>,
    execution_stats: Arc<RwLock<ExecutionStats>>,
    limits: ResourceLimits,
}

/// Internal plugin state
#[derive(Debug, Default)]
struct PluginState {
    initialized: bool,
    function_registry: HashMap<String, FunctionInfo>,
    data_store: HashMap<String, serde_json::Value>,
    last_activity: Option<Instant>,
}

/// Information about available plugin functions
#[derive(Debug, Clone)]
struct FunctionInfo {
    name: String,
    description: String,
    input_schema: Option<serde_json::Value>,
    output_schema: Option<serde_json::Value>,
    execution_count: u64,
    total_execution_time: Duration,
}

/// Execution statistics for the plugin
#[derive(Debug, Default)]
struct ExecutionStats {
    total_calls: u64,
    successful_calls: u64,
    failed_calls: u64,
    total_execution_time: Duration,
    last_error: Option<String>,
    last_execution: Option<Instant>,
}

impl ExampleWasmPlugin {
    /// Create a new example plugin
    pub fn new() -> Self {
        let metadata = WasmPluginMetadata {
            id: "example_wasm_plugin".to_string(),
            name: "Example WebAssembly Plugin".to_string(),
            version: "1.0.0".to_string(),
            description: "A demonstration WebAssembly plugin showing best practices".to_string(),
            author: "Personal AI Assistant Team".to_string(),
            license: "MIT".to_string(),
            capabilities: vec![
                "text_processing".to_string(),
                "data_analysis".to_string(),
                "utility_functions".to_string(),
            ],
            dependencies: vec![],
            api_version: "1.0".to_string(),
            checksum: "".to_string(),
        };
        
        let mut state = PluginState::default();
        
        // Register available functions
        state.function_registry.insert("hello".to_string(), FunctionInfo {
            name: "hello".to_string(),
            description: "Returns a greeting message".to_string(),
            input_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string"}
                }
            })),
            output_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "message": {"type": "string"}
                }
            })),
            execution_count: 0,
            total_execution_time: Duration::from_secs(0),
        });
        
        state.function_registry.insert("echo".to_string(), FunctionInfo {
            name: "echo".to_string(),
            description: "Echoes back the input message".to_string(),
            input_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "message": {"type": "string"}
                }
            })),
            output_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "echo": {"type": "string"}
                }
            })),
            execution_count: 0,
            total_execution_time: Duration::from_secs(0),
        });
        
        state.function_registry.insert("analyze_text".to_string(), FunctionInfo {
            name: "analyze_text".to_string(),
            description: "Analyzes text and returns statistics".to_string(),
            input_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "text": {"type": "string"}
                }
            })),
            output_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "word_count": {"type": "number"},
                    "character_count": {"type": "number"},
                    "lines": {"type": "number"}
                }
            })),
            execution_count: 0,
            total_execution_time: Duration::from_secs(0),
        });
        
        Self {
            metadata,
            config: Arc::new(RwLock::new(None)),
            state: Arc::new(RwLock::new(state)),
            execution_stats: Arc::new(RwLock::new(ExecutionStats::default())),
            limits: ResourceLimits::default(),
        }
    }
    
    /// Execute a specific function
    #[instrument(skip(self, input))]
    async fn execute_function(&self, function: &str, input: &[u8]) -> Result<Vec<u8>> {
        let start_time = Instant::now();
        
        // Parse input as JSON
        let input_json: serde_json::Value = serde_json::from_slice(input)
            .map_err(|e| AssistantError::Plugin(format!("Invalid JSON input: {}", e)))?;
        
        let result = match function {
            "hello" => self.handle_hello(input_json).await?,
            "echo" => self.handle_echo(input_json).await?,
            "analyze_text" => self.handle_analyze_text(input_json).await?,
            "list_functions" => self.handle_list_functions().await?,
            "get_stats" => self.handle_get_stats().await?,
            _ => return Err(AssistantError::Plugin(format!("Unknown function: {}", function))),
        };
        
        let execution_time = start_time.elapsed();
        
        // Update function stats
        self.update_function_stats(function, execution_time, true).await;
        
        // Serialize result
        let output = serde_json::to_vec(&result)
            .map_err(|e| AssistantError::Plugin(format!("Failed to serialize output: {}", e)))?;
        
        debug!("Function '{}' executed in {:?}", function, execution_time);
        Ok(output)
    }
    
    /// Handle hello function
    async fn handle_hello(&self, input: serde_json::Value) -> Result<serde_json::Value> {
        let name = input.get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("World");
        
        Ok(serde_json::json!({
            "message": format!("Hello, {}! This is the Example WebAssembly Plugin.", name)
        }))
    }
    
    /// Handle echo function
    async fn handle_echo(&self, input: serde_json::Value) -> Result<serde_json::Value> {
        let message = input.get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        
        Ok(serde_json::json!({
            "echo": message
        }))
    }
    
    /// Handle text analysis function
    async fn handle_analyze_text(&self, input: serde_json::Value) -> Result<serde_json::Value> {
        let text = input.get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        
        let word_count = text.split_whitespace().count();
        let character_count = text.chars().count();
        let lines = text.lines().count();
        
        Ok(serde_json::json!({
            "word_count": word_count,
            "character_count": character_count,
            "lines": lines,
            "analysis_time": chrono::Utc::now().to_rfc3339()
        }))
    }
    
    /// Handle list functions request
    async fn handle_list_functions(&self) -> Result<serde_json::Value> {
        let state = self.state.read().await;
        let functions: Vec<_> = state.function_registry.values()
            .map(|func| serde_json::json!({
                "name": func.name,
                "description": func.description,
                "input_schema": func.input_schema,
                "output_schema": func.output_schema,
                "execution_count": func.execution_count,
                "average_execution_time": func.total_execution_time.as_millis() as f64 / func.execution_count.max(1) as f64
            }))
            .collect();
        
        Ok(serde_json::json!({
            "functions": functions
        }))
    }
    
    /// Handle get stats request
    async fn handle_get_stats(&self) -> Result<serde_json::Value> {
        let stats = self.execution_stats.read().await;
        let state = self.state.read().await;
        
        Ok(serde_json::json!({
            "total_calls": stats.total_calls,
            "successful_calls": stats.successful_calls,
            "failed_calls": stats.failed_calls,
            "success_rate": if stats.total_calls > 0 { 
                stats.successful_calls as f64 / stats.total_calls as f64 
            } else { 
                0.0 
            },
            "average_execution_time": if stats.total_calls > 0 {
                stats.total_execution_time.as_millis() as f64 / stats.total_calls as f64
            } else {
                0.0
            },
            "last_error": stats.last_error,
            "last_execution": stats.last_execution.map(|t| t.elapsed().as_secs()),
            "initialized": state.initialized,
            "functions_available": state.function_registry.len(),
            "data_items_stored": state.data_store.len()
        }))
    }
    
    /// Update function execution statistics
    async fn update_function_stats(&self, function: &str, execution_time: Duration, success: bool) {
        // Update global stats
        {
            let mut stats = self.execution_stats.write().await;
            stats.total_calls += 1;
            if success {
                stats.successful_calls += 1;
            } else {
                stats.failed_calls += 1;
            }
            stats.total_execution_time += execution_time;
            stats.last_execution = Some(Instant::now());
        }
        
        // Update function-specific stats
        {
            let mut state = self.state.write().await;
            if let Some(func_info) = state.function_registry.get_mut(function) {
                func_info.execution_count += 1;
                func_info.total_execution_time += execution_time;
            }
            state.last_activity = Some(Instant::now());
        }
    }
    
    /// Store data in plugin state
    async fn store_data(&self, key: &str, value: serde_json::Value) -> Result<()> {
        let mut state = self.state.write().await;
        state.data_store.insert(key.to_string(), value);
        Ok(())
    }
    
    /// Retrieve data from plugin state
    async fn get_data(&self, key: &str) -> Option<serde_json::Value> {
        let state = self.state.read().await;
        state.data_store.get(key).cloned()
    }
    
    /// Validate function input against schema
    fn validate_input(&self, function: &str, input: &serde_json::Value) -> Result<()> {
        // In a real implementation, you would validate against the JSON schema
        // For this example, we'll do basic validation
        
        match function {
            "hello" => {
                if let Some(name) = input.get("name") {
                    if !name.is_string() {
                        return Err(AssistantError::Plugin("Name must be a string".to_string()));
                    }
                }
            }
            "echo" => {
                if let Some(message) = input.get("message") {
                    if !message.is_string() {
                        return Err(AssistantError::Plugin("Message must be a string".to_string()));
                    }
                }
            }
            "analyze_text" => {
                if let Some(text) = input.get("text") {
                    if !text.is_string() {
                        return Err(AssistantError::Plugin("Text must be a string".to_string()));
                    }
                } else {
                    return Err(AssistantError::Plugin("Text parameter is required".to_string()));
                }
            }
            _ => {}
        }
        
        Ok(())
    }
}

impl Default for ExampleWasmPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WasmPlugin for ExampleWasmPlugin {
    fn metadata(&self) -> &WasmPluginMetadata {
        &self.metadata
    }
    
    #[instrument(skip(self, config))]
    async fn initialize(&mut self, config: serde_json::Value) -> Result<()> {
        info!("Initializing Example WebAssembly Plugin");
        
        // Store configuration
        {
            let mut config_guard = self.config.write().await;
            *config_guard = Some(config.clone());
        }
        
        // Apply configuration
        if let Some(limits_config) = config.get("resource_limits") {
            if let Some(max_memory) = limits_config.get("max_memory") {
                if let Some(memory) = max_memory.as_u64() {
                    self.limits.max_memory = memory;
                }
            }
        }
        
        // Mark as initialized
        {
            let mut state = self.state.write().await;
            state.initialized = true;
            state.last_activity = Some(Instant::now());
        }
        
        info!("Plugin initialized successfully with config: {}", config);
        Ok(())
    }
    
    #[instrument(skip(self, input, context))]
    async fn execute(&self, function: &str, input: &[u8], context: &PluginContext) -> Result<Vec<u8>> {
        debug!("Executing function '{}' for user {}", function, context.user_id);
        
        // Check if plugin is initialized
        {
            let state = self.state.read().await;
            if !state.initialized {
                return Err(AssistantError::Plugin("Plugin not initialized".to_string()));
            }
        }
        
        // Parse and validate input
        let input_json: serde_json::Value = serde_json::from_slice(input)
            .map_err(|e| AssistantError::Plugin(format!("Invalid JSON input: {}", e)))?;
        
        self.validate_input(function, &input_json)?;
        
        // Execute function
        match self.execute_function(function, input).await {
            Ok(result) => {
                self.update_function_stats(function, context.started_at.elapsed(), true).await;
                Ok(result)
            }
            Err(e) => {
                self.update_function_stats(function, context.started_at.elapsed(), false).await;
                
                // Store error for stats
                {
                    let mut stats = self.execution_stats.write().await;
                    stats.last_error = Some(e.to_string());
                }
                
                Err(e)
            }
        }
    }
    
    fn can_handle(&self, capability: &str) -> bool {
        self.metadata.capabilities.contains(&capability.to_string())
    }
    
    async fn health_check(&self) -> Result<PluginHealth> {
        let stats = self.execution_stats.read().await;
        let state = self.state.read().await;
        
        let status = if !state.initialized {
            HealthStatus::Unhealthy
        } else if stats.failed_calls > stats.successful_calls {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };
        
        let message = match status {
            HealthStatus::Unhealthy => Some("Plugin not initialized".to_string()),
            HealthStatus::Degraded => Some(format!("High failure rate: {}/{}", stats.failed_calls, stats.total_calls)),
            HealthStatus::Healthy => None,
            HealthStatus::Unknown => Some("Unknown status".to_string()),
        };
        
        Ok(PluginHealth {
            status,
            message,
            last_check: chrono::Utc::now(),
            execution_count: stats.total_calls,
            error_count: stats.failed_calls,
            average_execution_time: if stats.total_calls > 0 {
                stats.total_execution_time / stats.total_calls as u32
            } else {
                Duration::from_secs(0)
            },
        })
    }
    
    async fn cleanup(&mut self) -> Result<()> {
        info!("Cleaning up Example WebAssembly Plugin");
        
        {
            let mut state = self.state.write().await;
            state.initialized = false;
            state.function_registry.clear();
            state.data_store.clear();
        }
        
        {
            let mut config = self.config.write().await;
            *config = None;
        }
        
        info!("Plugin cleanup completed");
        Ok(())
    }
}

/// Helper function to create a sample WebAssembly plugin binary
/// This would typically be compiled from Rust or another language to WebAssembly
pub fn create_sample_wasm_plugin() -> Vec<u8> {
    // This is a minimal WebAssembly module that exports a simple function
    // In practice, you would compile actual plugin code to WebAssembly
    
    // WAT (WebAssembly Text) representation:
    // (module
    //   (func $hello (export "hello") (result i32)
    //     i32.const 42
    //   )
    // )
    
    vec![
        0x00, 0x61, 0x73, 0x6d, // WebAssembly magic number
        0x01, 0x00, 0x00, 0x00, // Version 1
        0x01, 0x05, 0x01, 0x60, 0x00, 0x01, 0x7f, // Type section
        0x03, 0x02, 0x01, 0x00, // Function section
        0x07, 0x09, 0x01, 0x05, 0x68, 0x65, 0x6c, 0x6c, 0x6f, 0x00, 0x00, // Export section
        0x0a, 0x06, 0x01, 0x04, 0x00, 0x41, 0x2a, 0x0b, // Code section
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_example_plugin_creation() {
        let plugin = ExampleWasmPlugin::new();
        assert_eq!(plugin.metadata().id, "example_wasm_plugin");
        assert_eq!(plugin.metadata().version, "1.0.0");
    }
    
    #[tokio::test]
    async fn test_plugin_initialization() {
        let mut plugin = ExampleWasmPlugin::new();
        let config = serde_json::json!({
            "resource_limits": {
                "max_memory": 1024000
            }
        });
        
        let result = plugin.initialize(config).await;
        assert!(result.is_ok());
        
        let state = plugin.state.read().await;
        assert!(state.initialized);
    }
    
    #[tokio::test]
    async fn test_hello_function() {
        let mut plugin = ExampleWasmPlugin::new();
        plugin.initialize(serde_json::json!({})).await.unwrap();
        
        let context = PluginContext {
            user_id: "test_user".to_string(),
            session_id: "test_session".to_string(),
            request_id: "test_request".to_string(),
            metadata: HashMap::new(),
            started_at: Instant::now(),
        };
        
        let input = serde_json::json!({"name": "Test"});
        let input_bytes = serde_json::to_vec(&input).unwrap();
        
        let result = plugin.execute("hello", &input_bytes, &context).await.unwrap();
        let output: serde_json::Value = serde_json::from_slice(&result).unwrap();
        
        assert!(output.get("message").unwrap().as_str().unwrap().contains("Hello, Test"));
    }
    
    #[tokio::test]
    async fn test_analyze_text_function() {
        let mut plugin = ExampleWasmPlugin::new();
        plugin.initialize(serde_json::json!({})).await.unwrap();
        
        let context = PluginContext {
            user_id: "test_user".to_string(),
            session_id: "test_session".to_string(),
            request_id: "test_request".to_string(),
            metadata: HashMap::new(),
            started_at: Instant::now(),
        };
        
        let input = serde_json::json!({"text": "Hello world\nThis is a test"});
        let input_bytes = serde_json::to_vec(&input).unwrap();
        
        let result = plugin.execute("analyze_text", &input_bytes, &context).await.unwrap();
        let output: serde_json::Value = serde_json::from_slice(&result).unwrap();
        
        assert_eq!(output.get("word_count").unwrap().as_u64().unwrap(), 5);
        assert_eq!(output.get("lines").unwrap().as_u64().unwrap(), 2);
    }
    
    #[tokio::test]
    async fn test_health_check() {
        let plugin = ExampleWasmPlugin::new();
        let health = plugin.health_check().await.unwrap();
        
        // Plugin should be unhealthy before initialization
        assert_eq!(health.status, HealthStatus::Unhealthy);
    }
    
    #[test]
    fn test_sample_wasm_creation() {
        let wasm_bytes = create_sample_wasm_plugin();
        assert!(!wasm_bytes.is_empty());
        assert_eq!(&wasm_bytes[0..4], &[0x00, 0x61, 0x73, 0x6d]); // WebAssembly magic number
    }
}