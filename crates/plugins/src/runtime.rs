use crate::{ResourceLimits, PluginWasiCtx};
use rusty_ai_common::{Result, AssistantError};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{debug, warn, error, instrument};
use wasmtime::*;

/// WebAssembly runtime configuration
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Enable async support
    pub async_support: true,
    /// Enable fuel consumption tracking
    pub consume_fuel: bool,
    /// Enable epoch-based interruption
    pub epoch_interruption: bool,
    /// Maximum number of concurrent instances
    pub max_instances: usize,
    /// Memory pool size
    pub memory_pool_size: usize,
    /// Enable compilation cache
    pub compilation_cache: bool,
    /// Cache directory path
    pub cache_directory: Option<std::path::PathBuf>,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            async_support: true,
            consume_fuel: true,
            epoch_interruption: true,
            max_instances: 100,
            memory_pool_size: 512 * 1024 * 1024, // 512MB
            compilation_cache: true,
            cache_directory: None,
        }
    }
}

/// WebAssembly runtime manager with pooling and resource management
pub struct WasmRuntime {
    engine: Engine,
    config: RuntimeConfig,
    instance_pool: Arc<Mutex<Vec<PooledInstance>>>,
    active_instances: Arc<Mutex<usize>>,
    epoch_handle: EpochHandle,
}

/// Pooled WebAssembly instance for reuse
struct PooledInstance {
    store: Store<PluginWasiCtx>,
    instance: Instance,
    module_id: String,
    last_used: Instant,
    usage_count: u64,
}

impl WasmRuntime {
    /// Create a new WebAssembly runtime
    #[instrument]
    pub fn new(config: RuntimeConfig) -> Result<Self> {
        let mut wasmtime_config = Config::new();
        
        // Configure WebAssembly features
        wasmtime_config.async_support(config.async_support);
        wasmtime_config.consume_fuel(config.consume_fuel);
        wasmtime_config.epoch_interruption(config.epoch_interruption);
        
        // Security settings - disable potentially dangerous features
        wasmtime_config.wasm_multi_memory(false);
        wasmtime_config.wasm_threads(false);
        wasmtime_config.wasm_reference_types(false);
        wasmtime_config.wasm_simd(false);
        wasmtime_config.wasm_bulk_memory(false);
        wasmtime_config.wasm_multi_value(true); // Safe feature
        wasmtime_config.wasm_component_model(true);
        
        // Memory settings
        wasmtime_config.memory_reservation(config.memory_pool_size);
        wasmtime_config.memory_reservation_for_growth(16 * 1024 * 1024); // 16MB
        
        // Compilation cache
        if config.compilation_cache {
            if let Some(cache_dir) = &config.cache_directory {
                if let Err(e) = wasmtime_config.cache_config_load(cache_dir) {
                    warn!("Failed to load compilation cache: {}", e);
                }
            }
        }
        
        // Optimization settings
        wasmtime_config.cranelift_opt_level(OptLevel::Speed);
        
        let engine = Engine::new(&wasmtime_config)
            .map_err(|e| AssistantError::Plugin(format!("Failed to create Wasmtime engine: {}", e)))?;
        
        let epoch_handle = engine.epoch_handle();
        
        Ok(Self {
            engine,
            config,
            instance_pool: Arc::new(Mutex::new(Vec::new())),
            active_instances: Arc::new(Mutex::new(0)),
            epoch_handle,
        })
    }
    
    /// Get or create a WebAssembly instance
    #[instrument(skip(self, wasm_bytes))]
    pub async fn get_instance(
        &self,
        module_id: &str,
        wasm_bytes: &[u8],
        limits: ResourceLimits,
    ) -> Result<InstanceHandle> {
        // Check if we have a pooled instance available
        if let Some(pooled) = self.try_get_pooled_instance(module_id).await? {
            return Ok(InstanceHandle::new(pooled, self.epoch_handle.clone()));
        }
        
        // Create new instance if none available in pool
        self.create_new_instance(module_id, wasm_bytes, limits).await
    }
    
    /// Try to get an instance from the pool
    async fn try_get_pooled_instance(&self, module_id: &str) -> Result<Option<PooledInstance>> {
        let mut pool = self.instance_pool.lock().await;
        
        // Find and remove a matching instance from the pool
        if let Some(pos) = pool.iter().position(|inst| inst.module_id == module_id) {
            let mut instance = pool.remove(pos);
            instance.usage_count += 1;
            instance.last_used = Instant::now();
            
            debug!("Reusing pooled instance for module: {}", module_id);
            return Ok(Some(instance));
        }
        
        Ok(None)
    }
    
    /// Create a new WebAssembly instance
    async fn create_new_instance(
        &self,
        module_id: &str,
        wasm_bytes: &[u8],
        limits: ResourceLimits,
    ) -> Result<InstanceHandle> {
        // Check instance limit
        {
            let active_count = *self.active_instances.lock().await;
            if active_count >= self.config.max_instances {
                return Err(AssistantError::Plugin(
                    "Maximum number of active instances reached".to_string()
                ));
            }
        }
        
        // Compile module
        let module = Module::new(&self.engine, wasm_bytes)
            .map_err(|e| AssistantError::Plugin(format!("Failed to compile module: {}", e)))?;
        
        // Create WASI context with limits
        let wasi_ctx = PluginWasiCtx::new(limits.clone())?;
        let mut store = Store::new(&self.engine, wasi_ctx);
        
        // Configure store
        store.set_fuel(limits.max_fuel)
            .map_err(|e| AssistantError::Plugin(format!("Failed to set fuel: {}", e)))?;
        
        store.set_epoch_deadline(1);
        
        // Create linker with WASI
        let mut linker = Linker::new(&self.engine);
        wasmtime_wasi::add_to_linker_async(&mut linker)
            .map_err(|e| AssistantError::Plugin(format!("Failed to add WASI to linker: {}", e)))?;
        
        // Instantiate module
        let instance = linker.instantiate_async(&mut store, &module).await
            .map_err(|e| AssistantError::Plugin(format!("Failed to instantiate module: {}", e)))?;
        
        let pooled_instance = PooledInstance {
            store,
            instance,
            module_id: module_id.to_string(),
            last_used: Instant::now(),
            usage_count: 1,
        };
        
        // Increment active instance count
        {
            let mut active_count = self.active_instances.lock().await;
            *active_count += 1;
        }
        
        debug!("Created new instance for module: {}", module_id);
        Ok(InstanceHandle::new(pooled_instance, self.epoch_handle.clone()))
    }
    
    /// Return an instance to the pool
    pub async fn return_instance(&self, instance: PooledInstance) -> Result<()> {
        // Check if instance should be pooled or discarded
        if instance.usage_count < 1000 && // Max reuse count
           instance.last_used.elapsed() < Duration::from_hours(1) // Max age
        {
            let mut pool = self.instance_pool.lock().await;
            pool.push(instance);
            
            // Cleanup old instances if pool is too large
            if pool.len() > self.config.max_instances / 2 {
                pool.retain(|inst| inst.last_used.elapsed() < Duration::from_minutes(30));
            }
        } else {
            debug!("Discarding instance due to age or usage limits");
        }
        
        // Decrement active instance count
        {
            let mut active_count = self.active_instances.lock().await;
            *active_count = active_count.saturating_sub(1);
        }
        
        Ok(())
    }
    
    /// Interrupt all running instances (for emergency shutdown)
    pub fn interrupt_all(&self) {
        warn!("Interrupting all WebAssembly instances");
        self.epoch_handle.increment_epoch();
    }
    
    /// Get runtime statistics
    pub async fn get_stats(&self) -> RuntimeStats {
        let pool = self.instance_pool.lock().await;
        let active_count = *self.active_instances.lock().await;
        
        RuntimeStats {
            active_instances: active_count,
            pooled_instances: pool.len(),
            total_capacity: self.config.max_instances,
            memory_pool_size: self.config.memory_pool_size,
        }
    }
    
    /// Cleanup expired instances from pool
    pub async fn cleanup_pool(&self) {
        let mut pool = self.instance_pool.lock().await;
        let initial_size = pool.len();
        
        pool.retain(|instance| {
            instance.last_used.elapsed() < Duration::from_minutes(30)
        });
        
        let cleaned = initial_size - pool.len();
        if cleaned > 0 {
            debug!("Cleaned {} expired instances from pool", cleaned);
        }
    }
}

/// Handle for a WebAssembly instance
pub struct InstanceHandle {
    instance: Option<PooledInstance>,
    epoch_handle: EpochHandle,
    start_time: Instant,
}

impl InstanceHandle {
    fn new(instance: PooledInstance, epoch_handle: EpochHandle) -> Self {
        Self {
            instance: Some(instance),
            epoch_handle,
            start_time: Instant::now(),
        }
    }
    
    /// Execute a function on the instance
    pub async fn call_function<P, R>(
        &mut self,
        function_name: &str,
        params: P,
    ) -> Result<R>
    where
        P: wasmtime::WasmParams,
        R: wasmtime::WasmResults,
    {
        let instance = self.instance.as_mut()
            .ok_or_else(|| AssistantError::Plugin("Instance handle is invalid".to_string()))?;
        
        let func = instance.instance
            .get_typed_func::<P, R>(&mut instance.store, function_name)
            .map_err(|e| AssistantError::Plugin(format!("Function '{}' not found: {}", function_name, e)))?;
        
        // Set execution timeout
        let timeout_duration = Duration::from_secs(30);
        let deadline = self.start_time + timeout_duration;
        
        // Execute with timeout
        let result = tokio::time::timeout(timeout_duration, async {
            func.call_async(&mut instance.store, params).await
        }).await;
        
        match result {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(e)) => Err(AssistantError::Plugin(format!("Function execution failed: {}", e))),
            Err(_) => {
                // Timeout occurred - interrupt the instance
                self.epoch_handle.increment_epoch();
                Err(AssistantError::Plugin("Function execution timeout".to_string()))
            }
        }
    }
    
    /// Get reference to the store
    pub fn store_mut(&mut self) -> Result<&mut Store<PluginWasiCtx>> {
        let instance = self.instance.as_mut()
            .ok_or_else(|| AssistantError::Plugin("Instance handle is invalid".to_string()))?;
        Ok(&mut instance.store)
    }
    
    /// Get reference to the instance
    pub fn instance(&self) -> Result<&Instance> {
        let instance = self.instance.as_ref()
            .ok_or_else(|| AssistantError::Plugin("Instance handle is invalid".to_string()))?;
        Ok(&instance.instance)
    }
    
    /// Get execution time
    pub fn execution_time(&self) -> Duration {
        self.start_time.elapsed()
    }
}

impl Drop for InstanceHandle {
    fn drop(&mut self) {
        if let Some(instance) = self.instance.take() {
            // In a real implementation, you'd return this to the runtime pool
            // For now, we'll just log the cleanup
            debug!("Dropping instance handle for module: {}", instance.module_id);
        }
    }
}

/// Runtime statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct RuntimeStats {
    pub active_instances: usize,
    pub pooled_instances: usize,
    pub total_capacity: usize,
    pub memory_pool_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_runtime_creation() {
        let config = RuntimeConfig::default();
        let runtime = WasmRuntime::new(config).unwrap();
        
        let stats = runtime.get_stats().await;
        assert_eq!(stats.active_instances, 0);
        assert_eq!(stats.pooled_instances, 0);
    }
    
    #[tokio::test]
    async fn test_pool_cleanup() {
        let config = RuntimeConfig::default();
        let runtime = WasmRuntime::new(config).unwrap();
        
        runtime.cleanup_pool().await;
        
        let stats = runtime.get_stats().await;
        assert_eq!(stats.pooled_instances, 0);
    }
}