# External API Integrations Guide

## Overview

This guide covers the integration of external APIs and services with the Personal AI Assistant. Each integration is designed with robust error handling, rate limiting, caching, and fallback mechanisms to ensure reliable operation.

## Integration Architecture

### 1. API Client Framework

```rust
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::{sleep, Instant};

pub struct APIClient {
    client: Client,
    base_url: String,
    api_key: Option<String>,
    rate_limiter: Arc<RateLimiter>,
    retry_policy: RetryPolicy,
    cache: Arc<ResponseCache>,
}

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub backoff_multiplier: f64,
}

impl APIClient {
    pub async fn request<T, R>(
        &self,
        method: Method,
        endpoint: &str,
        payload: Option<T>,
    ) -> Result<R, APIError>
    where
        T: Serialize,
        R: for<'de> Deserialize<'de>,
    {
        // Check cache first
        let cache_key = self.generate_cache_key(method.clone(), endpoint, &payload);
        if let Some(cached_response) = self.cache.get(&cache_key).await {
            return Ok(cached_response);
        }
        
        // Apply rate limiting
        self.rate_limiter.wait().await?;
        
        // Execute request with retries
        let response = self.execute_with_retries(method, endpoint, payload).await?;
        
        // Cache successful responses
        if response.status().is_success() {
            let result: R = response.json().await?;
            self.cache.put(cache_key, &result, Duration::from_secs(300)).await;
            Ok(result)
        } else {
            Err(APIError::HttpError(response.status()))
        }
    }
    
    async fn execute_with_retries<T>(
        &self,
        method: Method,
        endpoint: &str,
        payload: Option<T>,
    ) -> Result<Response, APIError>
    where
        T: Serialize,
    {
        let mut attempt = 0;
        let mut delay = self.retry_policy.base_delay;
        
        loop {
            let result = self.execute_request(method.clone(), endpoint, &payload).await;
            
            match result {
                Ok(response) => return Ok(response),
                Err(e) if attempt >= self.retry_policy.max_retries => return Err(e),
                Err(APIError::RateLimited) => {
                    // Exponential backoff for rate limiting
                    sleep(delay).await;
                    delay = std::cmp::min(
                        Duration::from_millis(
                            (delay.as_millis() as f64 * self.retry_policy.backoff_multiplier) as u64
                        ),
                        self.retry_policy.max_delay,
                    );
                    attempt += 1;
                }
                Err(APIError::Temporary(_)) => {
                    sleep(delay).await;
                    delay = std::cmp::min(
                        Duration::from_millis(
                            (delay.as_millis() as f64 * self.retry_policy.backoff_multiplier) as u64
                        ),
                        self.retry_policy.max_delay,
                    );
                    attempt += 1;
                }
                Err(e) => return Err(e), // Don't retry permanent errors
            }
        }
    }
}
```

### 2. Google Services Integration

#### Google Calendar API

```rust
pub struct GoogleCalendarClient {
    client: APIClient,
    oauth_token: Arc<RwLock<OAuthToken>>,
    calendar_id: String,
}

impl GoogleCalendarClient {
    pub async fn get_events(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Vec<CalendarEvent>, GoogleAPIError> {
        let params = vec![
            ("timeMin", start_time.to_rfc3339()),
            ("timeMax", end_time.to_rfc3339()),
            ("singleEvents", "true".to_string()),
            ("orderBy", "startTime".to_string()),
        ];
        
        let response: GoogleCalendarResponse = self.client
            .request(
                Method::GET,
                &format!("calendars/{}/events", self.calendar_id),
                Some(params),
            )
            .await?;
        
        Ok(response.items.into_iter().map(|item| self.convert_event(item)).collect())
    }
    
    pub async fn create_event(&self, event: &CalendarEvent) -> Result<String, GoogleAPIError> {
        let google_event = self.convert_to_google_event(event);
        
        let response: GoogleCalendarEvent = self.client
            .request(
                Method::POST,
                &format!("calendars/{}/events", self.calendar_id),
                Some(google_event),
            )
            .await?;
        
        Ok(response.id)
    }
}
```

#### Gmail API Integration

```rust
pub struct GmailClient {
    client: APIClient,
    oauth_token: Arc<RwLock<OAuthToken>>,
}

impl GmailClient {
    pub async fn get_messages(
        &self,
        query: &str,
        max_results: u32,
    ) -> Result<Vec<GmailMessage>, GoogleAPIError> {
        let params = vec![
            ("q", query.to_string()),
            ("maxResults", max_results.to_string()),
        ];
        
        let response: GmailMessagesResponse = self.client
            .request(Method::GET, "messages", Some(params))
            .await?;
        
        // Fetch full message details in parallel
        let messages = self.fetch_message_details(response.messages).await?;
        Ok(messages)
    }
    
    async fn fetch_message_details(
        &self,
        message_refs: Vec<GmailMessageRef>,
    ) -> Result<Vec<GmailMessage>, GoogleAPIError> {
        let futures = message_refs.into_iter().map(|msg_ref| {
            let client = &self.client;
            async move {
                client
                    .request::<(), GmailMessage>(
                        Method::GET,
                        &format!("messages/{}", msg_ref.id),
                        None,
                    )
                    .await
            }
        });
        
        let results = futures::future::join_all(futures).await;
        let mut messages = Vec::new();
        
        for result in results {
            match result {
                Ok(message) => messages.push(message),
                Err(e) => tracing::warn!("Failed to fetch message: {}", e),
            }
        }
        
        Ok(messages)
    }
}
```

### 3. Financial API Integrations

#### Plaid Banking Integration

```rust
pub struct PlaidClient {
    client: APIClient,
    client_id: String,
    secret: String,
    environment: PlaidEnvironment,
}

impl PlaidClient {
    pub async fn exchange_public_token(
        &self,
        public_token: &str,
    ) -> Result<PlaidTokenExchangeResponse, PlaidError> {
        let request = PlaidTokenExchangeRequest {
            public_token: public_token.to_string(),
            client_id: self.client_id.clone(),
            secret: self.secret.clone(),
        };
        
        self.client
            .request(Method::POST, "link/token/exchange", Some(request))
            .await
            .map_err(PlaidError::from)
    }
    
    pub async fn get_accounts(
        &self,
        access_token: &str,
    ) -> Result<PlaidAccountsResponse, PlaidError> {
        let request = PlaidAccountsRequest {
            access_token: access_token.to_string(),
            client_id: self.client_id.clone(),
            secret: self.secret.clone(),
        };
        
        self.client
            .request(Method::POST, "accounts/get", Some(request))
            .await
            .map_err(PlaidError::from)
    }
    
    pub async fn get_transactions(
        &self,
        access_token: &str,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<PlaidTransactionsResponse, PlaidError> {
        let request = PlaidTransactionsRequest {
            access_token: access_token.to_string(),
            client_id: self.client_id.clone(),
            secret: self.secret.clone(),
            start_date,
            end_date,
            count: Some(500),
            offset: Some(0),
        };
        
        self.client
            .request(Method::POST, "transactions/get", Some(request))
            .await
            .map_err(PlaidError::from)
    }
}
```

### 4. Health Platform Integrations

#### Apple Health Integration

```rust
pub struct AppleHealthClient {
    // Note: Apple Health requires iOS app with HealthKit entitlements
    // This is a conceptual implementation for when such integration becomes available
}

// For now, implement file-based data import
pub struct HealthDataImporter {
    supported_formats: Vec<HealthDataFormat>,
}

#[derive(Debug, Clone)]
pub enum HealthDataFormat {
    AppleHealthExport,
    GoogleFitExport,
    FitbitCSV,
    GarminConnect,
    MyFitnessPalExport,
}

impl HealthDataImporter {
    pub async fn import_health_data(
        &self,
        file_path: &Path,
        format: HealthDataFormat,
    ) -> Result<Vec<HealthMetric>, HealthImportError> {
        match format {
            HealthDataFormat::AppleHealthExport => {
                self.parse_apple_health_export(file_path).await
            }
            HealthDataFormat::GoogleFitExport => {
                self.parse_google_fit_export(file_path).await
            }
            HealthDataFormat::FitbitCSV => {
                self.parse_fitbit_csv(file_path).await
            }
            _ => Err(HealthImportError::UnsupportedFormat),
        }
    }
    
    async fn parse_apple_health_export(
        &self,
        file_path: &Path,
    ) -> Result<Vec<HealthMetric>, HealthImportError> {
        // Parse Apple Health XML export
        let content = tokio::fs::read_to_string(file_path).await?;
        let document = roxmltree::Document::parse(&content)?;
        
        let mut metrics = Vec::new();
        
        for record in document.descendants().filter(|n| n.tag_name().name() == "Record") {
            if let Some(metric) = self.parse_health_record(record)? {
                metrics.push(metric);
            }
        }
        
        Ok(metrics)
    }
}
```

### 5. AI/ML Service Integrations

#### OpenAI API Integration

```rust
pub struct OpenAIClient {
    client: APIClient,
    api_key: String,
}

impl OpenAIClient {
    pub async fn chat_completion(
        &self,
        messages: Vec<ChatMessage>,
        model: &str,
    ) -> Result<ChatCompletionResponse, OpenAIError> {
        let request = ChatCompletionRequest {
            model: model.to_string(),
            messages,
            max_tokens: Some(1000),
            temperature: Some(0.7),
            top_p: Some(1.0),
            frequency_penalty: Some(0.0),
            presence_penalty: Some(0.0),
        };
        
        self.client
            .request(Method::POST, "chat/completions", Some(request))
            .await
            .map_err(OpenAIError::from)
    }
    
    pub async fn create_embedding(
        &self,
        input: &str,
        model: &str,
    ) -> Result<EmbeddingResponse, OpenAIError> {
        let request = EmbeddingRequest {
            input: input.to_string(),
            model: model.to_string(),
        };
        
        self.client
            .request(Method::POST, "embeddings", Some(request))
            .await
            .map_err(OpenAIError::from)
    }
}
```

### 6. Integration Management

#### Integration Registry

```rust
pub struct IntegrationRegistry {
    integrations: HashMap<String, Box<dyn Integration>>,
    health_checker: HealthChecker,
    metrics_collector: MetricsCollector,
}

#[async_trait]
pub trait Integration: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    async fn health_check(&self) -> Result<HealthStatus, IntegrationError>;
    async fn test_connection(&self) -> Result<(), IntegrationError>;
    fn get_rate_limits(&self) -> RateLimits;
    fn get_dependencies(&self) -> Vec<String>;
}

impl IntegrationRegistry {
    pub async fn register_integration<T>(&mut self, integration: T) -> Result<(), RegistryError>
    where
        T: Integration + 'static,
    {
        let name = integration.name().to_string();
        
        // Test the integration
        integration.test_connection().await
            .map_err(|e| RegistryError::IntegrationTestFailed(name.clone(), e))?;
        
        // Register for health monitoring
        self.health_checker.register(&name, Box::new(integration)).await?;
        
        self.integrations.insert(name, Box::new(integration));
        Ok(())
    }
    
    pub async fn get_integration_status(&self, name: &str) -> Option<IntegrationStatus> {
        if let Some(integration) = self.integrations.get(name) {
            match integration.health_check().await {
                Ok(health) => Some(IntegrationStatus {
                    name: name.to_string(),
                    healthy: true,
                    last_check: Utc::now(),
                    health_details: Some(health),
                    error: None,
                }),
                Err(e) => Some(IntegrationStatus {
                    name: name.to_string(),
                    healthy: false,
                    last_check: Utc::now(),
                    health_details: None,
                    error: Some(e.to_string()),
                }),
            }
        } else {
            None
        }
    }
}
```

### 7. Error Handling and Resilience

#### Circuit Breaker Pattern

```rust
pub struct CircuitBreaker {
    state: Arc<Mutex<CircuitBreakerState>>,
    failure_threshold: u32,
    recovery_timeout: Duration,
    half_open_max_calls: u32,
}

#[derive(Debug, Clone)]
enum CircuitBreakerState {
    Closed { failure_count: u32 },
    Open { opened_at: Instant },
    HalfOpen { success_count: u32, failure_count: u32 },
}

impl CircuitBreaker {
    pub async fn call<F, R, E>(&self, operation: F) -> Result<R, CircuitBreakerError<E>>
    where
        F: FnOnce() -> Result<R, E>,
        E: std::error::Error,
    {
        let can_proceed = {
            let mut state = self.state.lock().await;
            match &*state {
                CircuitBreakerState::Closed { .. } => true,
                CircuitBreakerState::Open { opened_at } => {
                    if opened_at.elapsed() > self.recovery_timeout {
                        *state = CircuitBreakerState::HalfOpen {
                            success_count: 0,
                            failure_count: 0,
                        };
                        true
                    } else {
                        false
                    }
                }
                CircuitBreakerState::HalfOpen { success_count, .. } => {
                    *success_count < self.half_open_max_calls
                }
            }
        };
        
        if !can_proceed {
            return Err(CircuitBreakerError::CircuitOpen);
        }
        
        match operation() {
            Ok(result) => {
                self.on_success().await;
                Ok(result)
            }
            Err(e) => {
                self.on_failure().await;
                Err(CircuitBreakerError::OperationFailed(e))
            }
        }
    }
    
    async fn on_success(&self) {
        let mut state = self.state.lock().await;
        match &*state {
            CircuitBreakerState::HalfOpen { success_count, .. } => {
                if *success_count + 1 >= self.half_open_max_calls {
                    *state = CircuitBreakerState::Closed { failure_count: 0 };
                } else {
                    if let CircuitBreakerState::HalfOpen { success_count, failure_count } = &mut *state {
                        *success_count += 1;
                    }
                }
            }
            CircuitBreakerState::Closed { failure_count } => {
                *failure_count = 0;
            }
            _ => {}
        }
    }
    
    async fn on_failure(&self) {
        let mut state = self.state.lock().await;
        match &*state {
            CircuitBreakerState::Closed { failure_count } => {
                if *failure_count + 1 >= self.failure_threshold {
                    *state = CircuitBreakerState::Open {
                        opened_at: Instant::now(),
                    };
                } else {
                    if let CircuitBreakerState::Closed { failure_count } = &mut *state {
                        *failure_count += 1;
                    }
                }
            }
            CircuitBreakerState::HalfOpen { failure_count, .. } => {
                *state = CircuitBreakerState::Open {
                    opened_at: Instant::now(),
                };
            }
            _ => {}
        }
    }
}
```

### 8. Webhook Integration

```rust
pub struct WebhookManager {
    endpoints: HashMap<String, WebhookEndpoint>,
    signature_verifier: SignatureVerifier,
    event_processor: Arc<EventProcessor>,
}

#[derive(Debug, Clone)]
pub struct WebhookEndpoint {
    pub service: String,
    pub url: String,
    pub secret: String,
    pub events: Vec<String>,
    pub retry_policy: RetryPolicy,
}

impl WebhookManager {
    pub async fn handle_webhook(
        &self,
        service: &str,
        headers: &HeaderMap,
        body: &[u8],
    ) -> Result<WebhookResponse, WebhookError> {
        let endpoint = self.endpoints.get(service)
            .ok_or(WebhookError::UnknownService)?;
        
        // Verify signature
        self.signature_verifier.verify(service, headers, body, &endpoint.secret)?;
        
        // Parse webhook payload
        let event = self.parse_webhook_event(service, body)?;
        
        // Process event
        self.event_processor.process_webhook_event(event).await?;
        
        Ok(WebhookResponse::Success)
    }
    
    pub async fn register_webhook(
        &self,
        service: &str,
        callback_url: &str,
        events: Vec<String>,
    ) -> Result<String, WebhookError> {
        match service {
            "plaid" => self.register_plaid_webhook(callback_url, events).await,
            "stripe" => self.register_stripe_webhook(callback_url, events).await,
            _ => Err(WebhookError::UnsupportedService),
        }
    }
}
```

This integration guide provides a comprehensive framework for connecting with external services while maintaining reliability, security, and performance standards throughout the Personal AI Assistant ecosystem.