use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
};
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use rusty_ai_core::AssistantCore;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketMessage {
    pub message_type: MessageType,
    pub session_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub data: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    Chat,
    VoiceData,
    StatusUpdate,
    Error,
    Ping,
    Pong,
}

#[derive(Debug)]
pub struct WebSocketConnection {
    pub user_id: Uuid,
    pub session_id: Uuid,
    pub connected_at: chrono::DateTime<chrono::Utc>,
    pub tx: broadcast::Sender<WebSocketMessage>,
}

pub struct WebSocketManager {
    connections: Arc<RwLock<HashMap<Uuid, WebSocketConnection>>>,
    core: Arc<AssistantCore>,
}

impl WebSocketManager {
    pub fn new(core: Arc<AssistantCore>) -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            core,
        }
    }

    pub async fn handle_socket(
        &self,
        socket: WebSocket,
        user_id: Uuid,
        session_id: Uuid,
    ) {
        info!("WebSocket connection established for user {}", user_id);

        let (tx, _rx) = broadcast::channel(100);
        let connection_id = Uuid::new_v4();

        // Store connection
        {
            let mut connections = self.connections.write().await;
            connections.insert(connection_id, WebSocketConnection {
                user_id,
                session_id,
                connected_at: chrono::Utc::now(),
                tx: tx.clone(),
            });
        }

        let (mut sender, mut receiver) = socket.split();
        let connections_ref = self.connections.clone();
        let core_ref = self.core.clone();

        // Spawn task to handle incoming messages
        let recv_task = tokio::spawn(async move {
            while let Some(msg) = receiver.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        debug!("Received WebSocket message: {}", text);
                        
                        match serde_json::from_str::<WebSocketMessage>(&text) {
                            Ok(ws_msg) => {
                                if let Err(e) = handle_websocket_message(
                                    ws_msg,
                                    &core_ref,
                                    &tx,
                                    user_id,
                                    session_id,
                                ).await {
                                    error!("Error handling WebSocket message: {}", e);
                                }
                            }
                            Err(e) => {
                                warn!("Failed to parse WebSocket message: {}", e);
                            }
                        }
                    }
                    Ok(Message::Binary(data)) => {
                        debug!("Received binary WebSocket data: {} bytes", data.len());
                        // Handle binary data (e.g., voice audio)
                        if let Err(e) = handle_binary_data(data, &core_ref, user_id, session_id).await {
                            error!("Error handling binary data: {}", e);
                        }
                    }
                    Ok(Message::Close(_)) => {
                        info!("WebSocket connection closed for user {}", user_id);
                        break;
                    }
                    Err(e) => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
        });

        // Spawn task to handle outgoing messages
        let mut rx = tx.subscribe();
        let send_task = tokio::spawn(async move {
            while let Ok(msg) = rx.recv().await {
                let json_msg = match serde_json::to_string(&msg) {
                    Ok(json) => json,
                    Err(e) => {
                        error!("Failed to serialize WebSocket message: {}", e);
                        continue;
                    }
                };

                if sender.send(Message::Text(json_msg)).await.is_err() {
                    break;
                }
            }
        });

        // Wait for either task to complete
        tokio::select! {
            _ = recv_task => {},
            _ = send_task => {},
        }

        // Clean up connection
        {
            let mut connections = connections_ref.write().await;
            connections.remove(&connection_id);
        }

        info!("WebSocket connection closed for user {}", user_id);
    }

    pub async fn broadcast_to_user(&self, user_id: Uuid, message: WebSocketMessage) {
        let connections = self.connections.read().await;
        for connection in connections.values() {
            if connection.user_id == user_id {
                if let Err(e) = connection.tx.send(message.clone()) {
                    warn!("Failed to send message to user {}: {}", user_id, e);
                }
            }
        }
    }

    pub async fn get_active_connections(&self) -> usize {
        self.connections.read().await.len()
    }
}

async fn handle_websocket_message(
    message: WebSocketMessage,
    core: &Arc<AssistantCore>,
    tx: &broadcast::Sender<WebSocketMessage>,
    user_id: Uuid,
    session_id: Uuid,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match message.message_type {
        MessageType::Chat => {
            // Handle chat message
            if let Some(text) = message.data.as_str() {
                let context_manager = core.context_manager.read().await;
                let user_context = context_manager.get_user_context(session_id).await?;
                
                let classification = core.intent_classifier.classify(text, Some(user_context));
                let response = core.orchestrator.process_intent(classification.intent.clone(), user_context).await?;
                
                // Send response back
                let response_msg = WebSocketMessage {
                    message_type: MessageType::Chat,
                    session_id: Some(session_id),
                    user_id: Some(user_id),
                    data: serde_json::json!({
                        "response": response,
                        "intent": classification.intent,
                        "confidence": classification.confidence
                    }),
                    timestamp: chrono::Utc::now(),
                };
                
                tx.send(response_msg)?;
            }
        }
        MessageType::Ping => {
            // Respond with pong
            let pong_msg = WebSocketMessage {
                message_type: MessageType::Pong,
                session_id: Some(session_id),
                user_id: Some(user_id),
                data: serde_json::json!({}),
                timestamp: chrono::Utc::now(),
            };
            tx.send(pong_msg)?;
        }
        _ => {
            debug!("Unhandled WebSocket message type: {:?}", message.message_type);
        }
    }

    Ok(())
}

async fn handle_binary_data(
    _data: Vec<u8>,
    _core: &Arc<AssistantCore>,
    _user_id: Uuid,
    _session_id: Uuid,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // TODO: Implement voice data processing
    debug!("Binary data received - voice processing not yet implemented");
    Ok(())
}

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(manager): State<Arc<WebSocketManager>>,
) -> Response {
    // In a real implementation, you would extract user info from headers/query params
    let user_id = Uuid::new_v4(); // Placeholder
    let session_id = Uuid::new_v4(); // Placeholder

    ws.on_upgrade(move |socket| manager.handle_socket(socket, user_id, session_id))
}