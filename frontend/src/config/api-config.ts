/**
 * API Configuration for Personal AI Assistant Frontend
 * 
 * This file contains all API endpoint configurations and connection settings
 * for communicating with the Rust backend server.
 */

// Environment variables with fallback defaults
const isDevelopment = import.meta.env.DEV || import.meta.env.NODE_ENV === 'development';
const isProduction = import.meta.env.PROD || import.meta.env.NODE_ENV === 'production';

// Base URLs for different environments
const API_BASE_URLS = {
  development: import.meta.env.VITE_API_BASE_URL || 'http://localhost:8081',
  production: import.meta.env.VITE_API_BASE_URL || 'https://api.yourdomain.com',
  test: import.meta.env.VITE_API_BASE_URL || 'http://localhost:8081',
} as const;

// WebSocket URLs for different environments
const WS_BASE_URLS = {
  development: import.meta.env.VITE_WS_BASE_URL || 'ws://localhost:8081',
  production: import.meta.env.VITE_WS_BASE_URL || 'wss://api.yourdomain.com',
  test: import.meta.env.VITE_WS_BASE_URL || 'ws://localhost:8081',
} as const;

// Get current environment
const getEnvironment = (): keyof typeof API_BASE_URLS => {
  if (import.meta.env.MODE) {
    return import.meta.env.MODE as keyof typeof API_BASE_URLS;
  }
  if (isProduction) return 'production';
  if (isDevelopment) return 'development';
  return 'development';
};

const currentEnvironment = getEnvironment();

// API Configuration
export const API_CONFIG = {
  // Base configuration
  baseURL: API_BASE_URLS[currentEnvironment],
  wsBaseURL: WS_BASE_URLS[currentEnvironment],
  timeout: Number(import.meta.env.VITE_API_TIMEOUT) || 30000, // 30 seconds
  
  // API versioning
  apiVersion: 'v1',
  
  // Request configuration
  maxRetries: Number(import.meta.env.VITE_API_MAX_RETRIES) || 3,
  retryDelay: Number(import.meta.env.VITE_API_RETRY_DELAY) || 1000, // 1 second
  
  // Authentication
  auth: {
    tokenStorageKey: 'ai_assistant_access_token',
    refreshTokenStorageKey: 'ai_assistant_refresh_token',
    tokenExpiryKey: 'ai_assistant_token_expiry',
    autoRefresh: true,
    refreshThreshold: 5 * 60 * 1000, // Refresh 5 minutes before expiry
  },
  
  // WebSocket configuration
  websocket: {
    reconnectAttempts: Number(import.meta.env.VITE_WS_RECONNECT_ATTEMPTS) || 5,
    reconnectInterval: Number(import.meta.env.VITE_WS_RECONNECT_INTERVAL) || 3000, // 3 seconds
    heartbeatInterval: Number(import.meta.env.VITE_WS_HEARTBEAT_INTERVAL) || 30000, // 30 seconds
    maxMessageSize: Number(import.meta.env.VITE_WS_MAX_MESSAGE_SIZE) || 1024 * 1024, // 1MB
  },
  
  // Voice configuration
  voice: {
    maxAudioFileSize: Number(import.meta.env.VITE_VOICE_MAX_FILE_SIZE) || 10 * 1024 * 1024, // 10MB
    supportedAudioFormats: ['mp3', 'wav', 'ogg', 'webm'],
    defaultVoice: import.meta.env.VITE_DEFAULT_VOICE || 'nova',
    defaultSpeed: Number(import.meta.env.VITE_DEFAULT_VOICE_SPEED) || 1.0,
    chunkSize: Number(import.meta.env.VITE_VOICE_CHUNK_SIZE) || 1024, // Audio chunk size for streaming
  },
  
  // File upload configuration
  upload: {
    maxFileSize: Number(import.meta.env.VITE_MAX_FILE_SIZE) || 50 * 1024 * 1024, // 50MB
    supportedDocumentTypes: [
      'application/pdf',
      'text/plain',
      'application/msword',
      'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
      'text/markdown',
      'application/json',
    ],
    chunkSize: Number(import.meta.env.VITE_UPLOAD_CHUNK_SIZE) || 1024 * 1024, // 1MB chunks
  },
  
  // Pagination defaults
  pagination: {
    defaultLimit: Number(import.meta.env.VITE_DEFAULT_PAGE_SIZE) || 50,
    maxLimit: Number(import.meta.env.VITE_MAX_PAGE_SIZE) || 100,
  },
  
  // Feature flags
  features: {
    voiceEnabled: import.meta.env.VITE_FEATURE_VOICE === 'true',
    pluginsEnabled: import.meta.env.VITE_FEATURE_PLUGINS === 'true',
    knowledgeBaseEnabled: import.meta.env.VITE_FEATURE_KNOWLEDGE_BASE === 'true',
    tasksEnabled: import.meta.env.VITE_FEATURE_TASKS === 'true',
    briefingEnabled: import.meta.env.VITE_FEATURE_BRIEFING === 'true',
    analyticsEnabled: import.meta.env.VITE_FEATURE_ANALYTICS === 'true',
  },
} as const;

// API Endpoints
export const API_ENDPOINTS = {
  // Health endpoints
  health: {
    basic: '/health',
    ready: '/health/ready',
    live: '/health/live',
    metrics: '/health/metrics',
  },
  
  // Authentication endpoints
  auth: {
    login: '/auth/login',
    logout: '/auth/logout',
    refresh: '/auth/refresh',
    validate: '/auth/validate',
  },
  
  // Conversation endpoints
  conversation: {
    send: `/api/${API_CONFIG.apiVersion}/conversation/send`,
    history: `/api/${API_CONFIG.apiVersion}/conversation/history`,
    session: (sessionId: string) => `/api/${API_CONFIG.apiVersion}/conversation/session/${sessionId}`,
  },
  
  // Voice endpoints
  voice: {
    transcribe: `/api/${API_CONFIG.apiVersion}/voice/transcribe`,
    synthesize: `/api/${API_CONFIG.apiVersion}/voice/synthesize`,
    voices: `/api/${API_CONFIG.apiVersion}/voice/voices`,
  },
  
  // Plugin endpoints
  plugins: {
    list: `/api/${API_CONFIG.apiVersion}/plugins`,
    install: `/api/${API_CONFIG.apiVersion}/plugins/install`,
    enable: (pluginId: string) => `/api/${API_CONFIG.apiVersion}/plugins/${pluginId}/enable`,
    disable: (pluginId: string) => `/api/${API_CONFIG.apiVersion}/plugins/${pluginId}/disable`,
    uninstall: (pluginId: string) => `/api/${API_CONFIG.apiVersion}/plugins/${pluginId}`,
  },
  
  // Knowledge base endpoints
  knowledge: {
    documents: `/api/${API_CONFIG.apiVersion}/knowledge/documents`,
    search: `/api/${API_CONFIG.apiVersion}/knowledge/search`,
    document: (documentId: string) => `/api/${API_CONFIG.apiVersion}/knowledge/documents/${documentId}`,
  },
  
  // Task management endpoints
  tasks: {
    list: `/api/${API_CONFIG.apiVersion}/tasks`,
    create: `/api/${API_CONFIG.apiVersion}/tasks`,
    update: (taskId: string) => `/api/${API_CONFIG.apiVersion}/tasks/${taskId}`,
    delete: (taskId: string) => `/api/${API_CONFIG.apiVersion}/tasks/${taskId}`,
  },
  
  // Briefing endpoints
  briefing: {
    daily: `/api/${API_CONFIG.apiVersion}/briefing/daily`,
    generate: `/api/${API_CONFIG.apiVersion}/briefing/generate`,
  },
  
  // WebSocket endpoint
  websocket: '/ws',
} as const;

// Request headers configuration
export const DEFAULT_HEADERS = {
  'Content-Type': 'application/json',
  'Accept': 'application/json',
  'User-Agent': `AI-Assistant-Frontend/${import.meta.env.VITE_APP_VERSION || '1.0.0'}`,
} as const;

// Error messages
export const API_ERROR_MESSAGES = {
  NETWORK_ERROR: 'Network error. Please check your internet connection.',
  TIMEOUT_ERROR: 'Request timed out. Please try again.',
  AUTH_ERROR: 'Authentication failed. Please log in again.',
  SERVER_ERROR: 'Server error. Please try again later.',
  VALIDATION_ERROR: 'Invalid request. Please check your input.',
  RATE_LIMIT_ERROR: 'Too many requests. Please wait before trying again.',
  UNKNOWN_ERROR: 'An unexpected error occurred. Please try again.',
} as const;

// HTTP Status codes
export const HTTP_STATUS = {
  OK: 200,
  CREATED: 201,
  BAD_REQUEST: 400,
  UNAUTHORIZED: 401,
  FORBIDDEN: 403,
  NOT_FOUND: 404,
  CONFLICT: 409,
  RATE_LIMITED: 429,
  INTERNAL_ERROR: 500,
  SERVICE_UNAVAILABLE: 503,
} as const;

// Utility functions
export const buildApiUrl = (endpoint: string): string => {
  const baseUrl = API_CONFIG.baseURL.replace(/\/$/, ''); // Remove trailing slash
  const cleanEndpoint = endpoint.startsWith('/') ? endpoint : `/${endpoint}`;
  return `${baseUrl}${cleanEndpoint}`;
};

export const buildWsUrl = (endpoint: string = API_ENDPOINTS.websocket): string => {
  const baseUrl = API_CONFIG.wsBaseURL.replace(/\/$/, ''); // Remove trailing slash
  const cleanEndpoint = endpoint.startsWith('/') ? endpoint : `/${endpoint}`;
  return `${baseUrl}${cleanEndpoint}`;
};

export const getAuthHeaders = (): Record<string, string> => {
  const token = localStorage.getItem(API_CONFIG.auth.tokenStorageKey);
  return token ? { Authorization: `Bearer ${token}` } : {};
};

export const isTokenExpired = (): boolean => {
  const expiryTime = localStorage.getItem(API_CONFIG.auth.tokenExpiryKey);
  if (!expiryTime) return true;
  return Date.now() > parseInt(expiryTime, 10);
};

export const shouldRefreshToken = (): boolean => {
  const expiryTime = localStorage.getItem(API_CONFIG.auth.tokenExpiryKey);
  if (!expiryTime) return true;
  const timeUntilExpiry = parseInt(expiryTime, 10) - Date.now();
  return timeUntilExpiry < API_CONFIG.auth.refreshThreshold;
};

// Environment validation
export const validateEnvironmentConfig = (): string[] => {
  const errors: string[] = [];
  
  if (!API_CONFIG.baseURL) {
    errors.push('API_BASE_URL is not configured');
  }
  
  if (!API_CONFIG.wsBaseURL) {
    errors.push('WS_BASE_URL is not configured');
  }
  
  if (isProduction) {
    if (API_CONFIG.baseURL.includes('localhost')) {
      errors.push('Production build is using localhost API URL');
    }
    
    if (API_CONFIG.wsBaseURL.includes('localhost')) {
      errors.push('Production build is using localhost WebSocket URL');
    }
    
    if (!API_CONFIG.baseURL.startsWith('https://')) {
      errors.push('Production API URL should use HTTPS');
    }
    
    if (!API_CONFIG.wsBaseURL.startsWith('wss://')) {
      errors.push('Production WebSocket URL should use WSS');
    }
  }
  
  return errors;
};

// Development helper
if (isDevelopment) {
  const configErrors = validateEnvironmentConfig();
  if (configErrors.length > 0) {
    console.warn('API Configuration Issues:', configErrors);
  }
  
  console.log('API Configuration:', {
    environment: currentEnvironment,
    baseURL: API_CONFIG.baseURL,
    wsBaseURL: API_CONFIG.wsBaseURL,
    features: API_CONFIG.features,
  });
}

export default API_CONFIG;