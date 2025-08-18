/**
 * React Hook for WebSocket Integration
 * 
 * Provides a React hook interface for managing WebSocket connections
 * with automatic cleanup and state management.
 */

import { useEffect, useRef, useState, useCallback } from 'react';
import { 
  WebSocketClient, 
  WebSocketMessage, 
  ConnectionState, 
  WebSocketEventListeners,
  websocketClient 
} from '../lib/websocket-client';

export interface UseWebSocketOptions {
  /** Custom WebSocket client instance (optional) */
  client?: WebSocketClient;
  /** Auto-connect on mount */
  autoConnect?: boolean;
  /** Session ID for the connection */
  sessionId?: string;
  /** User ID for the connection */
  userId?: string;
  /** Event listeners */
  onMessage?: (message: WebSocketMessage) => void;
  onOpen?: () => void;
  onClose?: (code: number, reason: string) => void;
  onError?: (error: Event) => void;
  onStateChange?: (state: ConnectionState) => void;
  onReconnectAttempt?: (attempt: number, maxAttempts: number) => void;
}

export interface UseWebSocketReturn {
  /** Current connection state */
  connectionState: ConnectionState;
  /** Whether currently connected */
  isConnected: boolean;
  /** Connect to WebSocket server */
  connect: (sessionId?: string, userId?: string) => Promise<void>;
  /** Disconnect from WebSocket server */
  disconnect: () => void;
  /** Send a generic message */
  sendMessage: (message: Omit<WebSocketMessage, 'timestamp'>) => boolean;
  /** Send a chat message */
  sendChatMessage: (text: string, sessionId?: string) => boolean;
  /** Send voice data */
  sendVoiceData: (audioData: ArrayBuffer, sessionId?: string) => boolean;
  /** Send ping */
  ping: () => boolean;
  /** Update session information */
  updateSession: (sessionId: string, userId?: string) => void;
  /** Last received message */
  lastMessage: WebSocketMessage | null;
  /** Connection error if any */
  error: Event | null;
}

/**
 * Custom hook for WebSocket management
 */
export function useWebSocket(options: UseWebSocketOptions = {}): UseWebSocketReturn {
  const {
    client = websocketClient,
    autoConnect = false,
    sessionId: initialSessionId,
    userId: initialUserId,
    onMessage,
    onOpen,
    onClose,
    onError,
    onStateChange,
    onReconnectAttempt,
  } = options;

  // State management
  const [connectionState, setConnectionState] = useState<ConnectionState>(client.getState());
  const [lastMessage, setLastMessage] = useState<WebSocketMessage | null>(null);
  const [error, setError] = useState<Event | null>(null);

  // Refs to avoid stale closures
  const onMessageRef = useRef(onMessage);
  const onOpenRef = useRef(onOpen);
  const onCloseRef = useRef(onClose);
  const onErrorRef = useRef(onError);
  const onStateChangeRef = useRef(onStateChange);
  const onReconnectAttemptRef = useRef(onReconnectAttempt);

  // Update refs when callbacks change
  useEffect(() => {
    onMessageRef.current = onMessage;
  }, [onMessage]);

  useEffect(() => {
    onOpenRef.current = onOpen;
  }, [onOpen]);

  useEffect(() => {
    onCloseRef.current = onClose;
  }, [onClose]);

  useEffect(() => {
    onErrorRef.current = onError;
  }, [onError]);

  useEffect(() => {
    onStateChangeRef.current = onStateChange;
  }, [onStateChange]);

  useEffect(() => {
    onReconnectAttemptRef.current = onReconnectAttempt;
  }, [onReconnectAttempt]);

  // Set up event listeners
  useEffect(() => {
    const listeners: WebSocketEventListeners = {
      onMessage: (message) => {
        setLastMessage(message);
        onMessageRef.current?.(message);
      },
      onOpen: () => {
        setError(null);
        onOpenRef.current?.();
      },
      onClose: (code, reason) => {
        onCloseRef.current?.(code, reason);
      },
      onError: (error) => {
        setError(error);
        onErrorRef.current?.(error);
      },
      onStateChange: (state) => {
        setConnectionState(state);
        onStateChangeRef.current?.(state);
      },
      onReconnectAttempt: (attempt, maxAttempts) => {
        onReconnectAttemptRef.current?.(attempt, maxAttempts);
      },
    };

    client.setListeners(listeners);

    // Update state to current client state
    setConnectionState(client.getState());

    return () => {
      // Clean up listeners on unmount
      client.setListeners({});
    };
  }, [client]);

  // Auto-connect on mount if enabled
  useEffect(() => {
    if (autoConnect && !client.isConnected()) {
      client.connect(initialSessionId, initialUserId).catch((err) => {
        console.error('Auto-connect failed:', err);
      });
    }
  }, [autoConnect, client, initialSessionId, initialUserId]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      if (autoConnect) {
        client.disconnect();
      }
    };
  }, [autoConnect, client]);

  // Memoized functions
  const connect = useCallback(
    (sessionId?: string, userId?: string) => {
      return client.connect(sessionId, userId);
    },
    [client]
  );

  const disconnect = useCallback(() => {
    client.disconnect();
  }, [client]);

  const sendMessage = useCallback(
    (message: Omit<WebSocketMessage, 'timestamp'>) => {
      return client.send(message);
    },
    [client]
  );

  const sendChatMessage = useCallback(
    (text: string, sessionId?: string) => {
      return client.sendChatMessage(text, sessionId);
    },
    [client]
  );

  const sendVoiceData = useCallback(
    (audioData: ArrayBuffer, sessionId?: string) => {
      return client.sendVoiceData(audioData, sessionId);
    },
    [client]
  );

  const ping = useCallback(() => {
    return client.ping();
  }, [client]);

  const updateSession = useCallback(
    (sessionId: string, userId?: string) => {
      client.updateSession(sessionId, userId);
    },
    [client]
  );

  const isConnected = connectionState === 'connected';

  return {
    connectionState,
    isConnected,
    connect,
    disconnect,
    sendMessage,
    sendChatMessage,
    sendVoiceData,
    ping,
    updateSession,
    lastMessage,
    error,
  };
}

/**
 * Hook for simple chat messaging over WebSocket
 */
export function useChatWebSocket(sessionId?: string) {
  const { sendChatMessage, lastMessage, isConnected, ...rest } = useWebSocket({
    autoConnect: true,
    sessionId,
  });

  // Filter for chat messages only
  const lastChatMessage = lastMessage?.message_type === 'Chat' ? lastMessage : null;

  const sendChat = useCallback(
    (text: string) => sendChatMessage(text, sessionId),
    [sendChatMessage, sessionId]
  );

  return {
    sendChat,
    lastChatMessage,
    isConnected,
    ...rest,
  };
}

/**
 * Hook for voice data streaming over WebSocket
 */
export function useVoiceWebSocket(sessionId?: string) {
  const { sendVoiceData, lastMessage, isConnected, ...rest } = useWebSocket({
    autoConnect: true,
    sessionId,
  });

  // Filter for voice messages only
  const lastVoiceMessage = lastMessage?.message_type === 'VoiceData' ? lastMessage : null;

  const sendVoice = useCallback(
    (audioData: ArrayBuffer) => sendVoiceData(audioData, sessionId),
    [sendVoiceData, sessionId]
  );

  return {
    sendVoice,
    lastVoiceMessage,
    isConnected,
    ...rest,
  };
}

/**
 * Hook for monitoring connection status with UI feedback
 */
export function useWebSocketStatus() {
  const { connectionState, isConnected, error } = useWebSocket();

  const statusText = {
    disconnected: 'Disconnected',
    connecting: 'Connecting...',
    connected: 'Connected',
    reconnecting: 'Reconnecting...',
    error: 'Connection Error',
  }[connectionState];

  const statusColor = {
    disconnected: 'gray',
    connecting: 'yellow',
    connected: 'green',
    reconnecting: 'orange',
    error: 'red',
  }[connectionState];

  return {
    connectionState,
    isConnected,
    statusText,
    statusColor,
    error,
  };
}

export default useWebSocket;