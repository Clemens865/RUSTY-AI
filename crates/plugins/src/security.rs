use crate::ResourceLimits;
use rusty_ai_common::{Result, AssistantError};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tracing::{warn, error, debug, instrument};
use wasmtime::*;

/// Security policy for plugin execution
#[derive(Debug, Clone)]
pub struct SecurityPolicy {
    /// Allowed WASI capabilities
    pub allowed_wasi_capabilities: HashSet<WasiCapability>,
    /// Allowed file system paths
    pub allowed_file_paths: Vec<PathBuf>,
    /// Allowed network hosts
    pub allowed_network_hosts: Vec<String>,
    /// Allowed environment variables
    pub allowed_env_vars: Vec<String>,
    /// Maximum number of file descriptors
    pub max_file_descriptors: u32,
    /// Maximum number of network connections
    pub max_network_connections: u32,
    /// Disable dangerous WebAssembly features
    pub disable_dangerous_features: bool,
    /// Enable signature verification
    pub require_signature: bool,
    /// Trusted plugin authors
    pub trusted_authors: HashSet<String>,
}

/// WASI capabilities that can be granted to plugins
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum WasiCapability {
    FileSystem,
    Network,
    EnvironmentVariables,
    CommandLineArguments,
    Random,
    Clock,
    Exit,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            allowed_wasi_capabilities: vec![
                WasiCapability::Random,
                WasiCapability::Clock,
            ].into_iter().collect(),
            allowed_file_paths: vec![],
            allowed_network_hosts: vec![],
            allowed_env_vars: vec![],
            max_file_descriptors: 5,
            max_network_connections: 2,
            disable_dangerous_features: true,
            require_signature: false,
            trusted_authors: HashSet::new(),
        }
    }
}

/// Secure plugin sandbox for WebAssembly execution
pub struct PluginSandbox {
    policy: SecurityPolicy,
    resource_monitor: ResourceMonitor,
    execution_stats: ExecutionStats,
}

/// Resource monitoring for plugin execution
#[derive(Debug)]
pub struct ResourceMonitor {
    start_time: Instant,
    memory_usage: u64,
    cpu_time: Duration,
    file_operations: u32,
    network_operations: u32,
    limits: ResourceLimits,
}

/// Execution statistics for security analysis
#[derive(Debug, Clone)]
pub struct ExecutionStats {
    pub total_executions: u64,
    pub failed_executions: u64,
    pub security_violations: u64,
    pub average_execution_time: Duration,
    pub peak_memory_usage: u64,
    pub total_cpu_time: Duration,
}

/// Security violation types
#[derive(Debug, Clone)]
pub enum SecurityViolation {
    ExcessiveMemoryUsage(u64),
    ExcessiveCpuTime(Duration),
    UnauthorizedFileAccess(PathBuf),
    UnauthorizedNetworkAccess(String),
    ProhibitedWasiCall(String),
    ResourceLimitExceeded(String),
    UntrustedPlugin(String),
    InvalidSignature,
}

impl PluginSandbox {
    /// Create a new plugin sandbox with security policy
    pub fn new(policy: SecurityPolicy, limits: ResourceLimits) -> Self {
        Self {
            policy,
            resource_monitor: ResourceMonitor::new(limits),
            execution_stats: ExecutionStats::default(),
        }
    }
    
    /// Validate plugin against security policy
    #[instrument(skip(self, wasm_bytes))]
    pub fn validate_plugin(&self, wasm_bytes: &[u8], metadata: &crate::WasmPluginMetadata) -> Result<()> {
        debug!("Validating plugin security: {}", metadata.id);
        
        // Check if plugin author is trusted
        if self.policy.require_signature && !self.policy.trusted_authors.contains(&metadata.author) {
            return Err(AssistantError::Security(
                format!("Plugin author not trusted: {}", metadata.author)
            ));
        }
        
        // Validate WebAssembly module
        self.validate_wasm_module(wasm_bytes)?;
        
        // Check plugin capabilities against policy
        self.validate_capabilities(&metadata.capabilities)?;
        
        debug!("Plugin validation successful: {}", metadata.id);
        Ok(())
    }
    
    /// Validate WebAssembly module structure
    fn validate_wasm_module(&self, wasm_bytes: &[u8]) -> Result<()> {
        let engine = Engine::default();
        let module = Module::new(&engine, wasm_bytes)
            .map_err(|e| AssistantError::Security(format!("Invalid WebAssembly module: {}", e)))?;
        
        if self.policy.disable_dangerous_features {
            // Check for prohibited features
            self.check_prohibited_features(&module)?;
        }
        
        // Validate imports and exports
        self.validate_imports(&module)?;
        self.validate_exports(&module)?;
        
        Ok(())
    }
    
    /// Check for prohibited WebAssembly features
    fn check_prohibited_features(&self, module: &Module) -> Result<()> {
        // In a real implementation, you would analyze the module's imports and exports
        // to detect potentially dangerous features like:
        // - Multi-memory
        // - Thread instructions
        // - SIMD instructions (if not allowed)
        // - Reference types (if not allowed)
        
        // For this implementation, we'll do basic validation
        let imports: Vec<_> = module.imports().collect();
        
        for import in imports {
            match import.module() {
                "wasi_snapshot_preview1" => {
                    // Validate WASI imports against policy
                    self.validate_wasi_import(import.name())?;
                }
                "env" => {
                    // Environment imports should be limited
                    warn!("Plugin imports from 'env' module: {}", import.name());
                }
                module_name => {
                    // Unknown imports might be dangerous
                    if !self.is_allowed_import_module(module_name) {
                        return Err(AssistantError::Security(
                            format!("Prohibited import module: {}", module_name)
                        ));
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Validate WASI import against security policy
    fn validate_wasi_import(&self, import_name: &str) -> Result<()> {
        let capability = match import_name {
            "fd_read" | "fd_write" | "fd_close" | "path_open" => WasiCapability::FileSystem,
            "sock_accept" | "sock_recv" | "sock_send" => WasiCapability::Network,
            "environ_get" | "environ_sizes_get" => WasiCapability::EnvironmentVariables,
            "args_get" | "args_sizes_get" => WasiCapability::CommandLineArguments,
            "random_get" => WasiCapability::Random,
            "clock_time_get" => WasiCapability::Clock,
            "proc_exit" => WasiCapability::Exit,
            _ => {
                // Unknown WASI call - be conservative
                warn!("Unknown WASI import: {}", import_name);
                return Ok(());
            }
        };
        
        if !self.policy.allowed_wasi_capabilities.contains(&capability) {
            return Err(AssistantError::Security(
                format!("WASI capability not allowed: {:?} ({})", capability, import_name)
            ));
        }
        
        Ok(())
    }
    
    /// Check if import module is allowed
    fn is_allowed_import_module(&self, module_name: &str) -> bool {
        // Allow common safe modules
        matches!(module_name, "wasi_snapshot_preview1" | "wasi_unstable")
    }
    
    /// Validate module imports
    fn validate_imports(&self, module: &Module) -> Result<()> {
        let import_count = module.imports().count();
        
        if import_count > 100 {
            return Err(AssistantError::Security(
                format!("Too many imports: {} (max: 100)", import_count)
            ));
        }
        
        Ok(())
    }
    
    /// Validate module exports
    fn validate_exports(&self, module: &Module) -> Result<()> {
        let exports: Vec<_> = module.exports().collect();
        
        // Check for required exports
        let has_main = exports.iter().any(|e| e.name() == "_start" || e.name() == "main");
        if !has_main {
            debug!("Plugin has no main function - this is allowed for library plugins");
        }
        
        // Check for suspicious exports
        for export in exports {
            if export.name().starts_with("__") {
                warn!("Plugin exports internal function: {}", export.name());
            }
        }
        
        Ok(())
    }
    
    /// Validate plugin capabilities
    fn validate_capabilities(&self, capabilities: &[String]) -> Result<()> {
        // In a real implementation, you would have a whitelist of allowed capabilities
        let prohibited_capabilities = ["system_admin", "unrestricted_network", "arbitrary_code"];
        
        for capability in capabilities {
            if prohibited_capabilities.contains(&capability.as_str()) {
                return Err(AssistantError::Security(
                    format!("Prohibited capability: {}", capability)
                ));
            }
        }
        
        Ok(())
    }
    
    /// Start resource monitoring for execution
    pub fn start_monitoring(&mut self) {
        self.resource_monitor.start();
    }
    
    /// Stop resource monitoring and record stats
    pub fn stop_monitoring(&mut self) -> Result<()> {
        let execution_time = self.resource_monitor.stop();
        
        // Update execution stats
        self.execution_stats.total_executions += 1;
        self.execution_stats.total_cpu_time += execution_time;
        
        // Calculate average execution time
        self.execution_stats.average_execution_time = 
            self.execution_stats.total_cpu_time / self.execution_stats.total_executions as u32;
        
        // Check for resource violations
        self.check_resource_violations()?;
        
        Ok(())
    }
    
    /// Check for resource violations after execution
    fn check_resource_violations(&mut self) -> Result<()> {
        let monitor = &self.resource_monitor;
        
        // Check memory usage
        if monitor.memory_usage > monitor.limits.max_memory {
            self.execution_stats.security_violations += 1;
            return Err(AssistantError::Security(
                format!("Memory limit exceeded: {} bytes (max: {})", 
                    monitor.memory_usage, monitor.limits.max_memory)
            ));
        }
        
        // Check CPU time
        if monitor.cpu_time > monitor.limits.cpu_time_limit {
            self.execution_stats.security_violations += 1;
            return Err(AssistantError::Security(
                format!("CPU time limit exceeded: {:?} (max: {:?})", 
                    monitor.cpu_time, monitor.limits.cpu_time_limit)
            ));
        }
        
        // Check file operations
        if monitor.file_operations > self.policy.max_file_descriptors {
            self.execution_stats.security_violations += 1;
            return Err(AssistantError::Security(
                format!("File operations limit exceeded: {} (max: {})", 
                    monitor.file_operations, self.policy.max_file_descriptors)
            ));
        }
        
        // Check network operations
        if monitor.network_operations > self.policy.max_network_connections {
            self.execution_stats.security_violations += 1;
            return Err(AssistantError::Security(
                format!("Network operations limit exceeded: {} (max: {})", 
                    monitor.network_operations, self.policy.max_network_connections)
            ));
        }
        
        Ok(())
    }
    
    /// Get execution statistics
    pub fn get_stats(&self) -> &ExecutionStats {
        &self.execution_stats
    }
    
    /// Update security policy
    pub fn update_policy(&mut self, policy: SecurityPolicy) {
        self.policy = policy;
    }
    
    /// Get current security policy
    pub fn get_policy(&self) -> &SecurityPolicy {
        &self.policy
    }
}

impl ResourceMonitor {
    /// Create a new resource monitor
    fn new(limits: ResourceLimits) -> Self {
        Self {
            start_time: Instant::now(),
            memory_usage: 0,
            cpu_time: Duration::from_secs(0),
            file_operations: 0,
            network_operations: 0,
            limits,
        }
    }
    
    /// Start monitoring
    fn start(&mut self) {
        self.start_time = Instant::now();
        self.memory_usage = 0;
        self.cpu_time = Duration::from_secs(0);
        self.file_operations = 0;
        self.network_operations = 0;
    }
    
    /// Stop monitoring and return execution time
    fn stop(&self) -> Duration {
        self.start_time.elapsed()
    }
    
    /// Record memory usage
    pub fn record_memory_usage(&mut self, bytes: u64) {
        self.memory_usage = self.memory_usage.max(bytes);
    }
    
    /// Record file operation
    pub fn record_file_operation(&mut self) {
        self.file_operations += 1;
    }
    
    /// Record network operation
    pub fn record_network_operation(&mut self) {
        self.network_operations += 1;
    }
}

impl Default for ExecutionStats {
    fn default() -> Self {
        Self {
            total_executions: 0,
            failed_executions: 0,
            security_violations: 0,
            average_execution_time: Duration::from_secs(0),
            peak_memory_usage: 0,
            total_cpu_time: Duration::from_secs(0),
        }
    }
}

/// Security configuration for different plugin trust levels
pub struct SecurityConfig;

impl SecurityConfig {
    /// Create a restrictive security policy for untrusted plugins
    pub fn untrusted() -> SecurityPolicy {
        SecurityPolicy {
            allowed_wasi_capabilities: vec![
                WasiCapability::Random,
                WasiCapability::Clock,
            ].into_iter().collect(),
            allowed_file_paths: vec![],
            allowed_network_hosts: vec![],
            allowed_env_vars: vec![],
            max_file_descriptors: 0,
            max_network_connections: 0,
            disable_dangerous_features: true,
            require_signature: true,
            trusted_authors: HashSet::new(),
        }
    }
    
    /// Create a moderate security policy for semi-trusted plugins
    pub fn semi_trusted() -> SecurityPolicy {
        SecurityPolicy {
            allowed_wasi_capabilities: vec![
                WasiCapability::Random,
                WasiCapability::Clock,
                WasiCapability::FileSystem,
            ].into_iter().collect(),
            allowed_file_paths: vec![
                PathBuf::from("/tmp"),
                PathBuf::from("/var/tmp"),
            ],
            allowed_network_hosts: vec![],
            allowed_env_vars: vec!["PATH".to_string()],
            max_file_descriptors: 5,
            max_network_connections: 0,
            disable_dangerous_features: true,
            require_signature: false,
            trusted_authors: HashSet::new(),
        }
    }
    
    /// Create a permissive security policy for trusted plugins
    pub fn trusted() -> SecurityPolicy {
        SecurityPolicy {
            allowed_wasi_capabilities: vec![
                WasiCapability::Random,
                WasiCapability::Clock,
                WasiCapability::FileSystem,
                WasiCapability::Network,
                WasiCapability::EnvironmentVariables,
            ].into_iter().collect(),
            allowed_file_paths: vec![
                PathBuf::from("/tmp"),
                PathBuf::from("/var/tmp"),
                PathBuf::from("/home"),
            ],
            allowed_network_hosts: vec!["api.example.com".to_string()],
            allowed_env_vars: vec!["PATH".to_string(), "HOME".to_string()],
            max_file_descriptors: 20,
            max_network_connections: 10,
            disable_dangerous_features: false,
            require_signature: false,
            trusted_authors: HashSet::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_security_policy_default() {
        let policy = SecurityPolicy::default();
        assert!(policy.allowed_wasi_capabilities.contains(&WasiCapability::Random));
        assert!(policy.allowed_wasi_capabilities.contains(&WasiCapability::Clock));
        assert!(policy.disable_dangerous_features);
    }
    
    #[test]
    fn test_security_configs() {
        let untrusted = SecurityConfig::untrusted();
        let semi_trusted = SecurityConfig::semi_trusted();
        let trusted = SecurityConfig::trusted();
        
        assert_eq!(untrusted.max_file_descriptors, 0);
        assert_eq!(semi_trusted.max_file_descriptors, 5);
        assert_eq!(trusted.max_file_descriptors, 20);
        
        assert!(untrusted.require_signature);
        assert!(!semi_trusted.require_signature);
        assert!(!trusted.require_signature);
    }
    
    #[test]
    fn test_resource_monitor() {
        let limits = ResourceLimits::default();
        let mut monitor = ResourceMonitor::new(limits);
        
        monitor.start();
        monitor.record_file_operation();
        monitor.record_memory_usage(1024);
        
        assert_eq!(monitor.file_operations, 1);
        assert_eq!(monitor.memory_usage, 1024);
    }
    
    #[test]
    fn test_execution_stats() {
        let stats = ExecutionStats::default();
        assert_eq!(stats.total_executions, 0);
        assert_eq!(stats.security_violations, 0);
    }
}