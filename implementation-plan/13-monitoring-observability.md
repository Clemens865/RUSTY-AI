# Monitoring & Observability Implementation

## Overview

Comprehensive monitoring and observability strategy for the Personal AI Assistant, including metrics collection, distributed tracing, logging, alerting, and performance monitoring.

## Observability Stack

### 1. Metrics Collection with Prometheus

```rust
use prometheus::{Counter, Histogram, Gauge, Registry, Encoder, TextEncoder};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct MetricsCollector {
    registry: Registry,
    // Request metrics
    http_requests_total: Counter,
    http_request_duration: Histogram,
    active_connections: Gauge,
    
    // Voice processing metrics
    voice_processing_duration: Histogram,
    voice_transcription_accuracy: Histogram,
    
    // Knowledge base metrics
    knowledge_search_duration: Histogram,
    knowledge_search_results: Histogram,
    
    // System metrics
    memory_usage: Gauge,
    cpu_usage: Gauge,
    
    // Business metrics
    user_interactions_total: Counter,
    plugin_executions_total: Counter,
}

impl MetricsCollector {
    pub fn new() -> Result<Self, prometheus::Error> {
        let registry = Registry::new();
        
        let http_requests_total = Counter::new(
            "http_requests_total",
            "Total number of HTTP requests"
        )?;
        registry.register(Box::new(http_requests_total.clone()))?;
        
        let http_request_duration = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "http_request_duration_seconds",
                "HTTP request duration in seconds"
            ).buckets(vec![0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0])
        )?;
        registry.register(Box::new(http_request_duration.clone()))?;
        
        let voice_processing_duration = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "voice_processing_duration_seconds",
                "Voice processing duration in seconds"
            ).buckets(vec![0.1, 0.2, 0.5, 1.0, 2.0, 5.0, 10.0])
        )?;
        registry.register(Box::new(voice_processing_duration.clone()))?;
        
        // ... register other metrics
        
        Ok(Self {
            registry,
            http_requests_total,
            http_request_duration,
            voice_processing_duration,
            // ... other metrics
        })
    }
    
    pub fn record_http_request(&self, method: &str, status: u16, duration: f64) {
        self.http_requests_total
            .with_label_values(&[method, &status.to_string()])
            .inc();
        self.http_request_duration.observe(duration);
    }
    
    pub fn record_voice_processing(&self, duration: f64, accuracy: f64) {
        self.voice_processing_duration.observe(duration);
        self.voice_transcription_accuracy.observe(accuracy);
    }
    
    pub fn export_metrics(&self) -> Result<String, prometheus::Error> {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;
        Ok(String::from_utf8(buffer).unwrap())
    }
}

// Middleware for automatic HTTP metrics collection
pub struct MetricsMiddleware {
    metrics: Arc<MetricsCollector>,
}

impl MetricsMiddleware {
    pub fn new(metrics: Arc<MetricsCollector>) -> Self {
        Self { metrics }
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for MetricsMiddleware
where
    S: Send + Sync,
{
    type Rejection = StatusCode;
    
    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let start_time = std::time::Instant::now();
        parts.extensions.insert(start_time);
        Ok(MetricsMiddleware::new(/* get from state */))
    }
}
```

### 2. Distributed Tracing with Jaeger

```rust
use opentelemetry::{global, trace::Tracer, KeyValue};
use opentelemetry_jaeger::PipelineBuilder;
use tracing::{info, instrument, Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;

pub struct TracingService {
    tracer: Box<dyn Tracer + Send + Sync>,
}

impl TracingService {
    pub async fn new(service_name: &str, jaeger_endpoint: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let tracer = opentelemetry_jaeger::new_agent_pipeline()
            .with_service_name(service_name)
            .with_endpoint(jaeger_endpoint)
            .install_batch(opentelemetry::runtime::Tokio)?;
        
        // Set global tracer
        global::set_text_map_propagator(opentelemetry_jaeger::Propagator::new());
        
        Ok(Self {
            tracer: Box::new(tracer),
        })
    }
    
    #[instrument(name = "voice_processing", skip(self, audio_data))]
    pub async fn trace_voice_processing(
        &self,
        audio_data: Vec<f32>,
        user_id: &str,
    ) -> Result<VoiceInteraction, VoiceError> {
        let span = Span::current();
        span.set_attribute(KeyValue::new("user_id", user_id.to_string()));
        span.set_attribute(KeyValue::new("audio_length", audio_data.len() as i64));
        
        // Create child span for STT
        let stt_result = self.trace_speech_to_text(&audio_data).await?;
        
        // Create child span for intent processing
        let intent_result = self.trace_intent_processing(&stt_result.text).await?;
        
        span.set_attribute(KeyValue::new("transcription_confidence", stt_result.confidence as f64));
        span.set_attribute(KeyValue::new("intent_type", format!("{:?}", intent_result.intent)));
        
        Ok(VoiceInteraction {
            transcript: stt_result.text,
            intent: intent_result.intent,
            confidence: stt_result.confidence,
            // ... other fields
        })
    }
    
    #[instrument(name = "speech_to_text", skip(self, audio_data))]
    async fn trace_speech_to_text(&self, audio_data: &[f32]) -> Result<TranscriptionResult, VoiceError> {
        let span = Span::current();
        span.add_event("Starting Whisper processing", vec![]);
        
        // Actual STT processing here
        let start_time = std::time::Instant::now();
        let result = self.stt_service.transcribe(audio_data).await?;
        let duration = start_time.elapsed();
        
        span.set_attribute(KeyValue::new("processing_duration_ms", duration.as_millis() as i64));
        span.set_attribute(KeyValue::new("transcript_length", result.text.len() as i64));
        
        Ok(result)
    }
    
    #[instrument(name = "knowledge_search", skip(self))]
    pub async fn trace_knowledge_search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<Document>, KnowledgeError> {
        let span = Span::current();
        span.set_attribute(KeyValue::new("query", query.to_string()));
        span.set_attribute(KeyValue::new("limit", limit as i64));
        
        let start_time = std::time::Instant::now();
        let results = self.knowledge_base.search_documents(query, limit, 0.3).await?;
        let duration = start_time.elapsed();
        
        span.set_attribute(KeyValue::new("results_count", results.len() as i64));
        span.set_attribute(KeyValue::new("search_duration_ms", duration.as_millis() as i64));
        
        Ok(results)
    }
}
```

### 3. Structured Logging

```rust
use tracing::{info, warn, error, debug, instrument, Span};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use tracing_appender::{rolling, non_blocking};
use serde_json::json;

pub struct LoggingService {
    _guard: tracing_appender::non_blocking::WorkerGuard,
}

impl LoggingService {
    pub fn init(log_level: &str, log_dir: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // File appender with rotation
        let file_appender = rolling::daily(log_dir, "assistant.log");
        let (non_blocking, guard) = non_blocking(file_appender);
        
        // Create subscriber with multiple layers
        tracing_subscriber::registry()
            .with(EnvFilter::new(log_level))
            .with(
                tracing_subscriber::fmt::layer()
                    .with_target(false)
                    .with_thread_ids(true)
                    .with_file(true)
                    .with_line_number(true)
                    .json()
                    .with_writer(non_blocking)
            )
            .with(
                tracing_subscriber::fmt::layer()
                    .with_target(false)
                    .pretty()
                    .with_writer(std::io::stdout)
            )
            .init();
        
        Ok(Self { _guard: guard })
    }
}

// Structured logging macros
#[macro_export]
macro_rules! log_user_action {
    ($user_id:expr, $action:expr, $($key:expr => $value:expr),*) => {
        info!(
            user_id = $user_id,
            action = $action,
            $($key = $value,)*
            "User action logged"
        );
    };
}

#[macro_export]
macro_rules! log_performance {
    ($operation:expr, $duration_ms:expr, $($key:expr => $value:expr),*) => {
        info!(
            operation = $operation,
            duration_ms = $duration_ms,
            $($key = $value,)*
            "Performance metric logged"
        );
    };
}

// Usage examples
pub async fn example_logging() {
    log_user_action!(
        "user123",
        "voice_interaction",
        "transcript" => "create reminder",
        "intent" => "command",
        "confidence" => 0.95
    );
    
    log_performance!(
        "knowledge_search",
        150,
        "query" => "artificial intelligence",
        "results_count" => 5,
        "cache_hit" => true
    );
}
```

### 4. Health Checks and Readiness Probes

```rust
use axum::{response::Json, http::StatusCode};
use serde_json::json;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct HealthChecker {
    database: Arc<DatabaseHealthCheck>,
    qdrant: Arc<QdrantHealthCheck>,
    external_apis: Arc<ExternalAPIHealthCheck>,
    voice_service: Arc<VoiceServiceHealthCheck>,
}

impl HealthChecker {
    pub async fn health_check(&self) -> Result<Json<HealthStatus>, StatusCode> {
        let mut checks = vec![];
        let start_time = Instant::now();
        
        // Database health
        let db_health = self.database.check().await;
        checks.push(("database", db_health));
        
        // Qdrant health
        let qdrant_health = self.qdrant.check().await;
        checks.push(("qdrant", qdrant_health));
        
        // External APIs health
        let api_health = self.external_apis.check().await;
        checks.push(("external_apis", api_health));
        
        // Voice service health
        let voice_health = self.voice_service.check().await;
        checks.push(("voice_service", voice_health));
        
        let overall_healthy = checks.iter().all(|(_, status)| status.healthy);
        let check_duration = start_time.elapsed();
        
        let health_status = HealthStatus {
            healthy: overall_healthy,
            timestamp: chrono::Utc::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            checks: checks.into_iter().map(|(name, status)| {
                (name.to_string(), status)
            }).collect(),
            duration_ms: check_duration.as_millis() as u64,
        };
        
        if overall_healthy {
            Ok(Json(health_status))
        } else {
            Err(StatusCode::SERVICE_UNAVAILABLE)
        }
    }
    
    pub async fn readiness_check(&self) -> Result<Json<ReadinessStatus>, StatusCode> {
        // Quick checks for readiness
        let database_ready = self.database.quick_check().await;
        let qdrant_ready = self.qdrant.quick_check().await;
        
        let ready = database_ready && qdrant_ready;
        
        let readiness_status = ReadinessStatus {
            ready,
            timestamp: chrono::Utc::now(),
            checks: vec![
                ("database".to_string(), database_ready),
                ("qdrant".to_string(), qdrant_ready),
            ],
        };
        
        if ready {
            Ok(Json(readiness_status))
        } else {
            Err(StatusCode::SERVICE_UNAVAILABLE)
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct HealthStatus {
    pub healthy: bool,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub version: String,
    pub checks: std::collections::HashMap<String, ComponentHealth>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ComponentHealth {
    pub healthy: bool,
    pub response_time_ms: u64,
    pub error: Option<String>,
    pub details: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ReadinessStatus {
    pub ready: bool,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub checks: Vec<(String, bool)>,
}
```

### 5. Alerting Configuration

#### Prometheus Alert Rules

```yaml
# prometheus/alert-rules.yml
groups:
- name: personal-ai-assistant
  rules:
  # High error rate
  - alert: HighErrorRate
    expr: |
      (
        sum(rate(http_requests_total{status=~"5.."}[5m]))
        /
        sum(rate(http_requests_total[5m]))
      ) > 0.05
    for: 2m
    labels:
      severity: critical
    annotations:
      summary: "High error rate detected"
      description: "Error rate is {{ $value | humanizePercentage }} over the last 5 minutes"
  
  # High response time
  - alert: HighResponseTime
    expr: |
      histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m])) > 1.0
    for: 3m
    labels:
      severity: warning
    annotations:
      summary: "High response time detected"
      description: "95th percentile response time is {{ $value }}s"
  
  # Voice processing issues
  - alert: VoiceProcessingSlowdown
    expr: |
      histogram_quantile(0.90, rate(voice_processing_duration_seconds_bucket[10m])) > 5.0
    for: 5m
    labels:
      severity: warning
    annotations:
      summary: "Voice processing is slow"
      description: "90th percentile voice processing time is {{ $value }}s"
  
  # Memory usage
  - alert: HighMemoryUsage
    expr: |
      (memory_usage_bytes / memory_limit_bytes) > 0.9
    for: 5m
    labels:
      severity: critical
    annotations:
      summary: "High memory usage"
      description: "Memory usage is {{ $value | humanizePercentage }}"
  
  # Database connection issues
  - alert: DatabaseConnectionFailure
    expr: |
      up{job="personal-ai-assistant"} == 0
    for: 1m
    labels:
      severity: critical
    annotations:
      summary: "Database connection failure"
      description: "Cannot connect to database"
```

#### Grafana Dashboards

```json
{
  "dashboard": {
    "title": "Personal AI Assistant - Overview",
    "panels": [
      {
        "title": "Request Rate",
        "type": "stat",
        "targets": [
          {
            "expr": "sum(rate(http_requests_total[5m]))",
            "legendFormat": "Requests/sec"
          }
        ]
      },
      {
        "title": "Error Rate",
        "type": "stat",
        "targets": [
          {
            "expr": "sum(rate(http_requests_total{status=~\"5..\"}[5m])) / sum(rate(http_requests_total[5m]))",
            "legendFormat": "Error Rate"
          }
        ]
      },
      {
        "title": "Response Time",
        "type": "graph",
        "targets": [
          {
            "expr": "histogram_quantile(0.50, rate(http_request_duration_seconds_bucket[5m]))",
            "legendFormat": "50th percentile"
          },
          {
            "expr": "histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))",
            "legendFormat": "95th percentile"
          },
          {
            "expr": "histogram_quantile(0.99, rate(http_request_duration_seconds_bucket[5m]))",
            "legendFormat": "99th percentile"
          }
        ]
      },
      {
        "title": "Voice Processing Performance",
        "type": "graph",
        "targets": [
          {
            "expr": "histogram_quantile(0.95, rate(voice_processing_duration_seconds_bucket[10m]))",
            "legendFormat": "Processing Time (95th percentile)"
          },
          {
            "expr": "rate(voice_transcription_accuracy_sum[10m]) / rate(voice_transcription_accuracy_count[10m])",
            "legendFormat": "Average Accuracy"
          }
        ]
      }
    ]
  }
}
```

### 6. Log Aggregation with ELK Stack

#### Logstash Configuration

```ruby
# logstash/pipeline/personal-ai-assistant.conf
input {
  beats {
    port => 5044
  }
}

filter {
  if [fields][service] == "personal-ai-assistant" {
    json {
      source => "message"
    }
    
    # Parse timestamp
    date {
      match => [ "timestamp", "ISO8601" ]
    }
    
    # Extract user ID for filtering
    if [user_id] {
      mutate {
        add_field => { "[@metadata][user_id]" => "%{user_id}" }
      }
    }
    
    # Categorize log levels
    if [level] == "ERROR" {
      mutate {
        add_tag => [ "error" ]
      }
    }
    
    # Extract performance metrics
    if [duration_ms] {
      mutate {
        convert => { "duration_ms" => "integer" }
      }
    }
  }
}

output {
  elasticsearch {
    hosts => ["elasticsearch:9200"]
    index => "personal-ai-assistant-%{+YYYY.MM.dd}"
  }
  
  # Send errors to alert system
  if "error" in [tags] {
    http {
      url => "http://alertmanager:9093/api/v1/alerts"
      http_method => "post"
      format => "json"
      mapping => {
        "alerts" => [{
          "labels" => {
            "alertname" => "ApplicationError",
            "service" => "personal-ai-assistant",
            "severity" => "warning",
            "user_id" => "%{[@metadata][user_id]}"
          },
          "annotations" => {
            "summary" => "Application error detected",
            "description" => "%{message}"
          }
        }]
      }
    }
  }
}
```

### 7. Performance Monitoring

```rust
use std::time::{Duration, Instant};
use tokio::time::interval;
use sysinfo::{System, SystemExt, ProcessExt};

pub struct PerformanceMonitor {
    system: System,
    metrics_collector: Arc<MetricsCollector>,
    process_id: u32,
}

impl PerformanceMonitor {
    pub fn new(metrics_collector: Arc<MetricsCollector>) -> Self {
        let system = System::new_all();
        let process_id = std::process::id();
        
        Self {
            system,
            metrics_collector,
            process_id,
        }
    }
    
    pub async fn start_monitoring(&mut self) {
        let mut interval = interval(Duration::from_secs(10));
        
        loop {
            interval.tick().await;
            self.collect_system_metrics().await;
        }
    }
    
    async fn collect_system_metrics(&mut self) {
        self.system.refresh_all();
        
        // CPU usage
        let cpu_usage = self.system.global_cpu_info().cpu_usage();
        self.metrics_collector.set_cpu_usage(cpu_usage as f64);
        
        // Memory usage
        let total_memory = self.system.total_memory();
        let used_memory = self.system.used_memory();
        let memory_usage_percent = (used_memory as f64 / total_memory as f64) * 100.0;
        self.metrics_collector.set_memory_usage(memory_usage_percent);
        
        // Process-specific metrics
        if let Some(process) = self.system.process(sysinfo::Pid::from(self.process_id as usize)) {
            let process_memory = process.memory();
            let process_cpu = process.cpu_usage();
            
            self.metrics_collector.set_process_memory(process_memory as f64);
            self.metrics_collector.set_process_cpu(process_cpu as f64);
        }
        
        // Disk usage
        for disk in self.system.disks() {
            let disk_usage = (disk.available_space() as f64 / disk.total_space() as f64) * 100.0;
            self.metrics_collector.set_disk_usage(disk.name().to_string_lossy().as_ref(), disk_usage);
        }
    }
}
```

This comprehensive monitoring and observability implementation provides full visibility into the Personal AI Assistant's performance, health, and behavior across all environments.