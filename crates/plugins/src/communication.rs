use rusty_ai_common::{Result, AssistantError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, RwLock};
use tracing::{debug, warn, error, instrument};
use uuid::Uuid;

/// Plugin communication interface for bidirectional messaging
pub struct PluginCommunication {
    channels: Arc<RwLock<HashMap<String, PluginChannel>>>,
    message_router: MessageRouter,
    serializer: MessageSerializer,
}

/// Communication channel for a specific plugin
#[derive(Debug)]
pub struct PluginChannel {
    plugin_id: String,
    sender: mpsc::UnboundedSender<PluginMessage>,
    receiver: Arc<RwLock<mpsc::UnboundedReceiver<PluginMessage>>>,
    active: bool,
}

/// Message router for handling plugin-to-plugin and plugin-to-host communication
pub struct MessageRouter {
    routes: Arc<RwLock<HashMap<String, Vec<String>>>>,
    broadcast_subscribers: Arc<RwLock<Vec<String>>>,
}

/// Message serializer for converting between different formats
pub struct MessageSerializer;

/// Plugin message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMessage {
    pub id: String,
    pub sender: String,
    pub recipient: Option<String>, // None for broadcast
    pub message_type: MessageType,
    pub payload: MessagePayload,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub reply_to: Option<String>,
}

/// Types of messages that can be sent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    Request,
    Response,
    Event,
    Broadcast,
    Error,
    System,
}

/// Message payload containing the actual data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessagePayload {
    Text(String),
    Binary(Vec<u8>),
    Json(serde_json::Value),
    Function {
        name: String,
        args: serde_json::Value,
    },
    Event {
        event_type: String,
        data: serde_json::Value,
    },
    Error {
        code: u32,
        message: String,
        details: Option<serde_json::Value>,
    },
}

/// Request-response pattern for synchronous communication
pub struct PluginRequest {
    pub id: String,
    pub function: String,
    pub args: serde_json::Value,
    pub timeout: std::time::Duration,
}

/// Response to a plugin request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginResponse {
    pub request_id: String,
    pub success: bool,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub execution_time: std::time::Duration,
}

/// Event subscription for asynchronous communication
#[derive(Debug, Clone)]
pub struct EventSubscription {
    pub plugin_id: String,
    pub event_types: Vec<String>,
    pub callback: Arc<dyn Fn(PluginMessage) -> Result<()> + Send + Sync>,
}

impl PluginCommunication {
    /// Create a new plugin communication system
    pub fn new() -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
            message_router: MessageRouter::new(),
            serializer: MessageSerializer::new(),
        }
    }
    
    /// Register a new plugin for communication
    #[instrument(skip(self))]
    pub async fn register_plugin(&self, plugin_id: &str) -> Result<PluginChannel> {
        debug!("Registering plugin communication: {}", plugin_id);
        
        let (sender, receiver) = mpsc::unbounded_channel();
        
        let channel = PluginChannel {
            plugin_id: plugin_id.to_string(),
            sender,
            receiver: Arc::new(RwLock::new(receiver)),
            active: true,
        };
        
        let mut channels = self.channels.write().await;
        channels.insert(plugin_id.to_string(), channel.clone());
        
        debug!("Plugin communication registered: {}", plugin_id);
        Ok(channel)
    }
    
    /// Unregister a plugin from communication
    #[instrument(skip(self))]
    pub async fn unregister_plugin(&self, plugin_id: &str) -> Result<()> {
        debug!("Unregistering plugin communication: {}", plugin_id);
        
        let mut channels = self.channels.write().await;
        if let Some(mut channel) = channels.remove(plugin_id) {
            channel.active = false;
        }
        
        self.message_router.remove_routes(plugin_id).await;
        
        debug!("Plugin communication unregistered: {}", plugin_id);
        Ok(())
    }
    
    /// Send a message to a specific plugin
    #[instrument(skip(self, payload))]
    pub async fn send_message(
        &self,
        sender: &str,
        recipient: &str,
        message_type: MessageType,
        payload: MessagePayload,
    ) -> Result<()> {
        let message = PluginMessage {
            id: Uuid::new_v4().to_string(),
            sender: sender.to_string(),
            recipient: Some(recipient.to_string()),
            message_type,
            payload,
            timestamp: chrono::Utc::now(),
            reply_to: None,
        };
        
        self.deliver_message(message).await
    }
    
    /// Broadcast a message to all plugins
    #[instrument(skip(self, payload))]
    pub async fn broadcast_message(
        &self,
        sender: &str,
        message_type: MessageType,
        payload: MessagePayload,
    ) -> Result<()> {
        let message = PluginMessage {
            id: Uuid::new_v4().to_string(),
            sender: sender.to_string(),
            recipient: None,
            message_type,
            payload,
            timestamp: chrono::Utc::now(),
            reply_to: None,
        };
        
        self.broadcast_to_subscribers(message).await
    }
    
    /// Send a request and wait for response
    #[instrument(skip(self, args))]
    pub async fn send_request(
        &self,
        sender: &str,
        recipient: &str,
        function: &str,
        args: serde_json::Value,
        timeout: std::time::Duration,
    ) -> Result<PluginResponse> {
        let request_id = Uuid::new_v4().to_string();
        
        // Create response channel
        let (response_tx, response_rx) = oneshot::channel();
        
        // Store response channel for later
        self.store_pending_request(&request_id, response_tx).await?;
        
        // Send request message
        let payload = MessagePayload::Function {
            name: function.to_string(),
            args,
        };
        
        let message = PluginMessage {
            id: request_id.clone(),
            sender: sender.to_string(),
            recipient: Some(recipient.to_string()),
            message_type: MessageType::Request,
            payload,
            timestamp: chrono::Utc::now(),
            reply_to: Some(request_id.clone()),
        };
        
        self.deliver_message(message).await?;
        
        // Wait for response with timeout
        match tokio::time::timeout(timeout, response_rx).await {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(_)) => Err(AssistantError::Plugin("Response channel closed".to_string())),
            Err(_) => Err(AssistantError::Plugin("Request timeout".to_string())),
        }
    }
    
    /// Deliver a message to its recipient
    async fn deliver_message(&self, message: PluginMessage) -> Result<()> {
        let channels = self.channels.read().await;
        
        if let Some(recipient) = &message.recipient {
            if let Some(channel) = channels.get(recipient) {
                if channel.active {
                    channel.sender.send(message)
                        .map_err(|e| AssistantError::Plugin(format!("Failed to send message: {}", e)))?;
                } else {
                    warn!("Attempted to send message to inactive plugin: {}", recipient);
                }
            } else {
                warn!("Attempted to send message to unknown plugin: {}", recipient);
            }
        } else {
            // Broadcast message
            self.broadcast_to_all(message, &channels).await?;
        }
        
        Ok(())
    }
    
    /// Broadcast message to all active plugins
    async fn broadcast_to_all(
        &self,
        message: PluginMessage,
        channels: &HashMap<String, PluginChannel>,
    ) -> Result<()> {
        for (plugin_id, channel) in channels.iter() {
            if channel.active && plugin_id != &message.sender {
                if let Err(e) = channel.sender.send(message.clone()) {
                    warn!("Failed to broadcast to plugin {}: {}", plugin_id, e);
                }
            }
        }
        Ok(())
    }
    
    /// Broadcast to subscribed plugins only
    async fn broadcast_to_subscribers(&self, message: PluginMessage) -> Result<()> {
        let subscribers = self.message_router.get_broadcast_subscribers().await;
        let channels = self.channels.read().await;
        
        for subscriber_id in subscribers {
            if let Some(channel) = channels.get(&subscriber_id) {
                if channel.active {
                    if let Err(e) = channel.sender.send(message.clone()) {
                        warn!("Failed to send to subscriber {}: {}", subscriber_id, e);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Store pending request for response handling
    async fn store_pending_request(
        &self,
        request_id: &str,
        response_tx: oneshot::Sender<PluginResponse>,
    ) -> Result<()> {
        // In a real implementation, you would store this in a pending requests map
        // For now, we'll skip the actual storage
        debug!("Stored pending request: {}", request_id);
        Ok(())
    }
    
    /// Handle incoming response
    pub async fn handle_response(&self, response: PluginResponse) -> Result<()> {
        // In a real implementation, you would look up the pending request
        // and send the response through the stored channel
        debug!("Handling response for request: {}", response.request_id);
        Ok(())
    }
    
    /// Get message receiver for a plugin
    pub async fn get_receiver(&self, plugin_id: &str) -> Result<Arc<RwLock<mpsc::UnboundedReceiver<PluginMessage>>>> {
        let channels = self.channels.read().await;
        
        if let Some(channel) = channels.get(plugin_id) {
            Ok(channel.receiver.clone())
        } else {
            Err(AssistantError::NotFound(format!("Plugin channel not found: {}", plugin_id)))
        }
    }
    
    /// Add message route between plugins
    pub async fn add_route(&self, from: &str, to: &str) -> Result<()> {
        self.message_router.add_route(from, to).await
    }
    
    /// Remove message route
    pub async fn remove_route(&self, from: &str, to: &str) -> Result<()> {
        self.message_router.remove_route(from, to).await
    }
    
    /// Subscribe plugin to broadcast messages
    pub async fn subscribe_to_broadcasts(&self, plugin_id: &str) -> Result<()> {
        self.message_router.add_broadcast_subscriber(plugin_id).await
    }
    
    /// Unsubscribe plugin from broadcast messages
    pub async fn unsubscribe_from_broadcasts(&self, plugin_id: &str) -> Result<()> {
        self.message_router.remove_broadcast_subscriber(plugin_id).await
    }
}

impl PluginChannel {
    /// Send a message through this channel
    pub fn send(&self, message: PluginMessage) -> Result<()> {
        if self.active {
            self.sender.send(message)
                .map_err(|e| AssistantError::Plugin(format!("Failed to send message: {}", e)))?;
        }
        Ok(())
    }
    
    /// Check if channel is active
    pub fn is_active(&self) -> bool {
        self.active
    }
    
    /// Get plugin ID
    pub fn plugin_id(&self) -> &str {
        &self.plugin_id
    }
}

impl Clone for PluginChannel {
    fn clone(&self) -> Self {
        Self {
            plugin_id: self.plugin_id.clone(),
            sender: self.sender.clone(),
            receiver: self.receiver.clone(),
            active: self.active,
        }
    }
}

impl MessageRouter {
    /// Create a new message router
    pub fn new() -> Self {
        Self {
            routes: Arc::new(RwLock::new(HashMap::new())),
            broadcast_subscribers: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Add a route from one plugin to another
    pub async fn add_route(&self, from: &str, to: &str) -> Result<()> {
        let mut routes = self.routes.write().await;
        routes.entry(from.to_string())
            .or_insert_with(Vec::new)
            .push(to.to_string());
        Ok(())
    }
    
    /// Remove a specific route
    pub async fn remove_route(&self, from: &str, to: &str) -> Result<()> {
        let mut routes = self.routes.write().await;
        if let Some(destinations) = routes.get_mut(from) {
            destinations.retain(|dest| dest != to);
        }
        Ok(())
    }
    
    /// Remove all routes for a plugin
    pub async fn remove_routes(&self, plugin_id: &str) {
        let mut routes = self.routes.write().await;
        routes.remove(plugin_id);
        
        // Remove plugin from all destination lists
        for destinations in routes.values_mut() {
            destinations.retain(|dest| dest != plugin_id);
        }
    }
    
    /// Get destinations for a plugin
    pub async fn get_routes(&self, from: &str) -> Vec<String> {
        let routes = self.routes.read().await;
        routes.get(from).cloned().unwrap_or_default()
    }
    
    /// Add broadcast subscriber
    pub async fn add_broadcast_subscriber(&self, plugin_id: &str) -> Result<()> {
        let mut subscribers = self.broadcast_subscribers.write().await;
        if !subscribers.contains(&plugin_id.to_string()) {
            subscribers.push(plugin_id.to_string());
        }
        Ok(())
    }
    
    /// Remove broadcast subscriber
    pub async fn remove_broadcast_subscriber(&self, plugin_id: &str) -> Result<()> {
        let mut subscribers = self.broadcast_subscribers.write().await;
        subscribers.retain(|id| id != plugin_id);
        Ok(())
    }
    
    /// Get all broadcast subscribers
    pub async fn get_broadcast_subscribers(&self) -> Vec<String> {
        let subscribers = self.broadcast_subscribers.read().await;
        subscribers.clone()
    }
}

impl MessageSerializer {
    /// Create a new message serializer
    pub fn new() -> Self {
        Self
    }
    
    /// Serialize message to JSON
    pub fn to_json(&self, message: &PluginMessage) -> Result<String> {
        serde_json::to_string(message)
            .map_err(|e| AssistantError::Plugin(format!("Failed to serialize message: {}", e)))
    }
    
    /// Deserialize message from JSON
    pub fn from_json(&self, json: &str) -> Result<PluginMessage> {
        serde_json::from_str(json)
            .map_err(|e| AssistantError::Plugin(format!("Failed to deserialize message: {}", e)))
    }
    
    /// Serialize message to binary
    pub fn to_binary(&self, message: &PluginMessage) -> Result<Vec<u8>> {
        let json = self.to_json(message)?;
        Ok(json.into_bytes())
    }
    
    /// Deserialize message from binary
    pub fn from_binary(&self, bytes: &[u8]) -> Result<PluginMessage> {
        let json = String::from_utf8(bytes.to_vec())
            .map_err(|e| AssistantError::Plugin(format!("Invalid UTF-8 in message: {}", e)))?;
        self.from_json(&json)
    }
}

impl Default for PluginCommunication {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};
    
    #[tokio::test]
    async fn test_plugin_communication_creation() {
        let comm = PluginCommunication::new();
        
        let channel = comm.register_plugin("test_plugin").await.unwrap();
        assert_eq!(channel.plugin_id(), "test_plugin");
        assert!(channel.is_active());
    }
    
    #[tokio::test]
    async fn test_message_routing() {
        let router = MessageRouter::new();
        
        router.add_route("plugin1", "plugin2").await.unwrap();
        let routes = router.get_routes("plugin1").await;
        
        assert!(routes.contains(&"plugin2".to_string()));
    }
    
    #[tokio::test]
    async fn test_broadcast_subscription() {
        let router = MessageRouter::new();
        
        router.add_broadcast_subscriber("plugin1").await.unwrap();
        let subscribers = router.get_broadcast_subscribers().await;
        
        assert!(subscribers.contains(&"plugin1".to_string()));
    }
    
    #[tokio::test]
    async fn test_message_serialization() {
        let serializer = MessageSerializer::new();
        
        let message = PluginMessage {
            id: "test_id".to_string(),
            sender: "test_sender".to_string(),
            recipient: Some("test_recipient".to_string()),
            message_type: MessageType::Request,
            payload: MessagePayload::Text("test message".to_string()),
            timestamp: chrono::Utc::now(),
            reply_to: None,
        };
        
        let json = serializer.to_json(&message).unwrap();
        let deserialized = serializer.from_json(&json).unwrap();
        
        assert_eq!(message.id, deserialized.id);
        assert_eq!(message.sender, deserialized.sender);
    }
    
    #[tokio::test]
    async fn test_plugin_unregistration() {
        let comm = PluginCommunication::new();
        
        comm.register_plugin("test_plugin").await.unwrap();
        comm.unregister_plugin("test_plugin").await.unwrap();
        
        // After unregistration, getting receiver should fail
        let result = comm.get_receiver("test_plugin").await;
        assert!(result.is_err());
    }
}