/**
 * WebSocket Client for Personal AI Assistant
 * 
 * Provides real-time bidirectional communication with the backend server
 * with automatic reconnection, heartbeat, and message handling.
 */

import { API_CONFIG, buildWsUrl, getAuthHeaders } from '../config/api-config';

// Message types matching backend WebSocket API
export type MessageType = 'Chat' | 'VoiceData' | 'StatusUpdate' | 'Error' | 'Ping' | 'Pong';

export interface WebSocketMessage {
  message_type: MessageType;
  session_id?: string;
  user_id?: string;
  data: any;
  timestamp: string;
}

export interface WebSocketConfig {
  url?: string;
  reconnectAttempts?: number;
  reconnectInterval?: number;
  heartbeatInterval?: number;
  maxMessageSize?: number;
  debug?: boolean;
}

export type ConnectionState = 'disconnected' | 'connecting' | 'connected' | 'reconnecting' | 'error';

export interface WebSocketEventListeners {
  onOpen?: () => void;
  onClose?: (code: number, reason: string) => void;
  onError?: (error: Event) => void;
  onMessage?: (message: WebSocketMessage) => void;
  onStateChange?: (state: ConnectionState) => void;
  onReconnectAttempt?: (attempt: number, maxAttempts: number) => void;
}

export class WebSocketClient {
  private ws: WebSocket | null = null;
  private config: Required<WebSocketConfig>;
  private listeners: WebSocketEventListeners = {};
  private state: ConnectionState = 'disconnected';
  private reconnectAttempt = 0;
  private heartbeatTimer: number | null = null;
  private reconnectTimer: number | null = null;
  private sessionId: string | null = null;
  private userId: string | null = null;
  private messageQueue: WebSocketMessage[] = [];
  private isManuallyDisconnected = false;

  constructor(config: WebSocketConfig = {}) {
    this.config = {
      url: config.url || buildWsUrl(),
      reconnectAttempts: config.reconnectAttempts ?? API_CONFIG.websocket.reconnectAttempts,
      reconnectInterval: config.reconnectInterval ?? API_CONFIG.websocket.reconnectInterval,
      heartbeatInterval: config.heartbeatInterval ?? API_CONFIG.websocket.heartbeatInterval,
      maxMessageSize: config.maxMessageSize ?? API_CONFIG.websocket.maxMessageSize,
      debug: config.debug ?? import.meta.env.DEV,
    };

    this.log('WebSocket client initialized with config:', this.config);
  }

  /**
   * Connect to the WebSocket server
   */
  public connect(sessionId?: string, userId?: string): Promise<void> {
    return new Promise((resolve, reject) => {
      if (this.ws?.readyState === WebSocket.OPEN) {
        this.log('Already connected');
        resolve();
        return;
      }

      this.sessionId = sessionId || this.sessionId;
      this.userId = userId || this.userId;
      this.isManuallyDisconnected = false;

      this.setState('connecting');

      try {
        // Build WebSocket URL with authentication token
        const authHeaders = getAuthHeaders();
        const token = authHeaders.Authorization?.replace('Bearer ', '');
        const wsUrl = token ? `${this.config.url}?token=${token}` : this.config.url;

        this.ws = new WebSocket(wsUrl);
        
        this.ws.onopen = () => {
          this.log('WebSocket connected');
          this.setState('connected');
          this.reconnectAttempt = 0;
          this.startHeartbeat();
          this.flushMessageQueue();
          this.listeners.onOpen?.();
          resolve();
        };

        this.ws.onclose = (event) => {
          this.log('WebSocket disconnected:', event.code, event.reason);
          this.setState('disconnected');
          this.stopHeartbeat();
          this.listeners.onClose?.(event.code, event.reason);

          if (!this.isManuallyDisconnected && this.shouldReconnect()) {
            this.scheduleReconnect();
          }
        };

        this.ws.onerror = (error) => {
          this.log('WebSocket error:', error);
          this.setState('error');
          this.listeners.onError?.(error);
          reject(error);
        };

        this.ws.onmessage = (event) => {
          try {
            const message: WebSocketMessage = JSON.parse(event.data);
            this.handleMessage(message);
          } catch (error) {
            this.log('Failed to parse WebSocket message:', error);
          }
        };

      } catch (error) {
        this.log('Failed to create WebSocket connection:', error);
        this.setState('error');
        reject(error);
      }
    });
  }

  /**
   * Disconnect from the WebSocket server
   */
  public disconnect(): void {
    this.isManuallyDisconnected = true;
    this.stopHeartbeat();
    this.clearReconnectTimer();
    
    if (this.ws) {
      this.ws.close(1000, 'Manual disconnect');
      this.ws = null;
    }
    
    this.setState('disconnected');
    this.log('WebSocket manually disconnected');
  }

  /**
   * Send a message through the WebSocket
   */
  public send(message: Omit<WebSocketMessage, 'timestamp'>): boolean {
    const fullMessage: WebSocketMessage = {
      ...message,
      session_id: message.session_id || this.sessionId || undefined,
      user_id: message.user_id || this.userId || undefined,
      timestamp: new Date().toISOString(),
    };

    // Validate message size
    const messageStr = JSON.stringify(fullMessage);
    if (messageStr.length > this.config.maxMessageSize) {
      this.log('Message too large:', messageStr.length, 'bytes');
      return false;
    }

    if (this.ws?.readyState === WebSocket.OPEN) {
      try {
        this.ws.send(messageStr);
        this.log('Sent message:', fullMessage.message_type);
        return true;
      } catch (error) {
        this.log('Failed to send message:', error);
        return false;
      }
    } else {
      // Queue message for later if not connected
      this.messageQueue.push(fullMessage);
      this.log('Message queued (not connected):', fullMessage.message_type);
      return false;
    }
  }

  /**
   * Send a chat message
   */
  public sendChatMessage(text: string, sessionId?: string): boolean {
    return this.send({
      message_type: 'Chat',
      session_id: sessionId,
      data: text,
    });
  }

  /**
   * Send voice data (binary)
   */
  public sendVoiceData(audioData: ArrayBuffer, sessionId?: string): boolean {
    // Convert to base64 for JSON transport
    const base64Data = btoa(String.fromCharCode(...new Uint8Array(audioData)));
    return this.send({
      message_type: 'VoiceData',
      session_id: sessionId,
      data: {
        audio: base64Data,
        format: 'webm', // or whatever format is being used
      },
    });
  }

  /**
   * Send a ping message
   */
  public ping(): boolean {
    return this.send({
      message_type: 'Ping',
      data: {},
    });
  }

  /**
   * Get current connection state
   */
  public getState(): ConnectionState {
    return this.state;
  }

  /**
   * Check if currently connected
   */
  public isConnected(): boolean {
    return this.state === 'connected' && this.ws?.readyState === WebSocket.OPEN;
  }

  /**
   * Set event listeners
   */
  public setListeners(listeners: WebSocketEventListeners): void {
    this.listeners = { ...this.listeners, ...listeners };
  }

  /**
   * Add a single event listener
   */
  public addEventListener<K extends keyof WebSocketEventListeners>(
    event: K,
    listener: WebSocketEventListeners[K]
  ): void {
    this.listeners[event] = listener;
  }

  /**
   * Remove an event listener
   */
  public removeEventListener<K extends keyof WebSocketEventListeners>(event: K): void {
    delete this.listeners[event];
  }

  /**
   * Update session and user IDs
   */
  public updateSession(sessionId: string, userId?: string): void {
    this.sessionId = sessionId;
    if (userId) {
      this.userId = userId;
    }
    this.log('Session updated:', sessionId, userId);
  }

  // Private methods

  private setState(newState: ConnectionState): void {
    if (this.state !== newState) {
      this.state = newState;
      this.listeners.onStateChange?.(newState);
      this.log('State changed to:', newState);
    }
  }

  private handleMessage(message: WebSocketMessage): void {
    this.log('Received message:', message.message_type);

    // Handle specific message types
    switch (message.message_type) {
      case 'Ping':
        // Respond to ping with pong
        this.send({
          message_type: 'Pong',
          data: {},
        });
        break;
      
      case 'Pong':
        // Heartbeat response received
        this.log('Heartbeat acknowledged');
        break;
      
      default:
        // Forward to application
        this.listeners.onMessage?.(message);
        break;
    }
  }

  private startHeartbeat(): void {
    this.stopHeartbeat();
    this.heartbeatTimer = window.setInterval(() => {
      if (this.isConnected()) {
        this.ping();
      } else {
        this.stopHeartbeat();
      }
    }, this.config.heartbeatInterval);
  }

  private stopHeartbeat(): void {
    if (this.heartbeatTimer) {
      clearInterval(this.heartbeatTimer);
      this.heartbeatTimer = null;
    }
  }

  private shouldReconnect(): boolean {
    return this.reconnectAttempt < this.config.reconnectAttempts;
  }

  private scheduleReconnect(): void {
    if (!this.shouldReconnect()) {
      this.log('Max reconnect attempts reached');
      return;
    }

    this.reconnectAttempt++;
    this.setState('reconnecting');
    
    this.listeners.onReconnectAttempt?.(this.reconnectAttempt, this.config.reconnectAttempts);
    
    this.reconnectTimer = window.setTimeout(() => {
      this.log(`Reconnect attempt ${this.reconnectAttempt}/${this.config.reconnectAttempts}`);
      this.connect(this.sessionId || undefined, this.userId || undefined).catch((error) => {
        this.log('Reconnect failed:', error);
      });
    }, this.config.reconnectInterval);
  }

  private clearReconnectTimer(): void {
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
  }

  private flushMessageQueue(): void {
    while (this.messageQueue.length > 0 && this.isConnected()) {
      const message = this.messageQueue.shift();
      if (message) {
        const messageStr = JSON.stringify(message);
        try {
          this.ws?.send(messageStr);
          this.log('Sent queued message:', message.message_type);
        } catch (error) {
          this.log('Failed to send queued message:', error);
          // Re-queue the message
          this.messageQueue.unshift(message);
          break;
        }
      }
    }
  }

  private log(...args: any[]): void {
    if (this.config.debug) {
      console.log('[WebSocket]', ...args);
    }
  }
}

// Create a singleton instance for the application
export const websocketClient = new WebSocketClient();

// React hook for using WebSocket in components
export function useWebSocket() {
  return websocketClient;
}

export default WebSocketClient;