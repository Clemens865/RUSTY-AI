# Personal AI Assistant API Documentation

This document provides comprehensive documentation for the Personal AI Assistant REST API and WebSocket interface.

## Table of Contents

- [Overview](#overview)
- [Authentication](#authentication)
- [Base URLs](#base-urls)
- [Response Format](#response-format)
- [Error Codes](#error-codes)
- [Health Endpoints](#health-endpoints)
- [Authentication Endpoints](#authentication-endpoints)
- [Conversation Endpoints](#conversation-endpoints)
- [Voice Endpoints](#voice-endpoints)
- [Plugin Endpoints](#plugin-endpoints)
- [Knowledge Base Endpoints](#knowledge-base-endpoints)
- [Task Management Endpoints](#task-management-endpoints)
- [Briefing Endpoints](#briefing-endpoints)
- [WebSocket API](#websocket-api)
- [Rate Limiting](#rate-limiting)
- [Examples](#examples)

## Overview

The Personal AI Assistant API provides a comprehensive interface for managing conversations, voice interactions, plugins, knowledge bases, tasks, and daily briefings. The API follows REST principles and uses JSON for data exchange.

### API Version

Current API version: `v1`

### Content Type

All API requests and responses use `application/json` content type unless otherwise specified.

## Authentication

The API uses JWT (JSON Web Token) based authentication with access and refresh tokens.

### Authentication Flow

1. **Login** - Exchange credentials for access and refresh tokens
2. **Protected Requests** - Include access token in Authorization header
3. **Token Refresh** - Use refresh token to get new access token when expired
4. **Logout** - Invalidate tokens

### Authorization Header Format

```
Authorization: Bearer <access_token>
```

## Base URLs

- **Development**: `http://localhost:8080`
- **Production**: `https://api.yourdomain.com`

## Response Format

All API responses follow a consistent format:

### Success Response

```json
{
  "success": true,
  "data": {
    // Response data
  },
  "timestamp": "2024-01-15T10:30:00Z"
}
```

### Error Response

```json
{
  "success": false,
  "error": {
    "code": "ERROR_CODE",
    "message": "Human readable error message",
    "details": {
      // Additional error details (optional)
    }
  },
  "timestamp": "2024-01-15T10:30:00Z"
}
```

## Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `VALIDATION_ERROR` | 400 | Request validation failed |
| `UNAUTHORIZED` | 401 | Authentication required |
| `FORBIDDEN` | 403 | Insufficient permissions |
| `NOT_FOUND` | 404 | Resource not found |
| `CONFLICT` | 409 | Resource already exists |
| `RATE_LIMITED` | 429 | Rate limit exceeded |
| `INTERNAL_ERROR` | 500 | Internal server error |
| `SERVICE_UNAVAILABLE` | 503 | Service temporarily unavailable |

## Health Endpoints

### GET /health

Basic health check endpoint.

**Response:**
```json
{
  "success": true,
  "data": {
    "status": "healthy",
    "version": "1.0.0",
    "uptime": 3600,
    "services": {
      "database": "healthy",
      "storage": "healthy",
      "plugins": "healthy",
      "voice": "inactive"
    }
  }
}
```

### GET /health/ready

Kubernetes readiness probe.

**Response:**
```json
{
  "status": "ready",
  "timestamp": "2024-01-15T10:30:00Z",
  "checks": {
    "database": "ready",
    "storage": "ready"
  }
}
```

### GET /health/live

Kubernetes liveness probe.

**Response:**
```json
{
  "status": "alive",
  "timestamp": "2024-01-15T10:30:00Z",
  "uptime_seconds": 3600
}
```

### GET /health/metrics

Basic metrics endpoint.

**Response:**
```json
{
  "timestamp": "2024-01-15T10:30:00Z",
  "system": {
    "memory_usage_mb": 512,
    "cpu_usage_percent": 25.5,
    "uptime_seconds": 3600
  },
  "application": {
    "active_sessions": 15,
    "total_requests": 1250,
    "error_rate": 0.02,
    "avg_response_time_ms": 150.5
  },
  "services": {
    "database_connections": 10,
    "plugin_count": 5,
    "voice_pipeline_status": "active"
  }
}
```

## Authentication Endpoints

### POST /auth/login

Authenticate user and receive access tokens.

**Request:**
```json
{
  "email": "user@example.com",
  "password": "securepassword"
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "expires_in": 3600,
    "token_type": "Bearer",
    "user": {
      "id": "123e4567-e89b-12d3-a456-426614174000",
      "email": "user@example.com",
      "name": "John Doe",
      "permissions": ["read", "write"]
    }
  }
}
```

### POST /auth/refresh

Refresh access token using refresh token.

**Request:**
```json
{
  "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "expires_in": 3600,
    "token_type": "Bearer"
  }
}
```

### POST /auth/logout

Invalidate user tokens.

**Request:**
```json
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "message": "Logged out successfully"
  }
}
```

### POST /auth/validate

Validate access token.

**Request:**
```json
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "valid": true,
    "user_id": "123e4567-e89b-12d3-a456-426614174000",
    "email": "user@example.com",
    "expires_at": 1642249800,
    "permissions": ["read", "write"]
  }
}
```

## Conversation Endpoints

All conversation endpoints require authentication.

### POST /api/v1/conversation/send

Send a message to the AI assistant.

**Request:**
```json
{
  "message": "What's the weather like today?",
  "session_id": "123e4567-e89b-12d3-a456-426614174000",
  "context": {
    "location": "San Francisco"
  }
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "response": "The current weather in San Francisco is 68°F with partly cloudy skies.",
    "session_id": "123e4567-e89b-12d3-a456-426614174000",
    "intent": "weather_query",
    "confidence": 0.95,
    "response_time_ms": 250
  }
}
```

### GET /api/v1/conversation/history

Get conversation history.

**Query Parameters:**
- `session_id` (optional): Filter by session ID
- `limit` (optional): Number of messages to return (default: 50)
- `offset` (optional): Pagination offset (default: 0)

**Response:**
```json
{
  "success": true,
  "data": {
    "messages": [
      {
        "id": "123e4567-e89b-12d3-a456-426614174000",
        "type": "user",
        "content": "What's the weather like today?",
        "timestamp": "2024-01-15T10:30:00Z",
        "session_id": "123e4567-e89b-12d3-a456-426614174000"
      },
      {
        "id": "123e4567-e89b-12d3-a456-426614174001",
        "type": "assistant",
        "content": "The current weather in San Francisco is 68°F with partly cloudy skies.",
        "timestamp": "2024-01-15T10:30:01Z",
        "session_id": "123e4567-e89b-12d3-a456-426614174000",
        "intent": "weather_query",
        "confidence": 0.95
      }
    ],
    "total": 2,
    "has_more": false
  }
}
```

### DELETE /api/v1/conversation/session/{session_id}

Delete a conversation session.

**Response:**
```json
{
  "success": true,
  "data": {
    "message": "Session deleted successfully"
  }
}
```

## Voice Endpoints

### POST /api/v1/voice/transcribe

Transcribe audio to text.

**Request:**
- Content-Type: `multipart/form-data`
- Form field: `audio` (audio file)
- Form field: `language` (optional, default: "en")

**Response:**
```json
{
  "success": true,
  "data": {
    "text": "Hello, what's the weather like today?",
    "confidence": 0.92,
    "language": "en",
    "duration_ms": 2500
  }
}
```

### POST /api/v1/voice/synthesize

Convert text to speech.

**Request:**
```json
{
  "text": "The current weather in San Francisco is 68°F with partly cloudy skies.",
  "voice": "nova",
  "speed": 1.0,
  "format": "mp3"
}
```

**Response:**
- Content-Type: `audio/mpeg`
- Binary audio data

### GET /api/v1/voice/voices

List available voices.

**Response:**
```json
{
  "success": true,
  "data": {
    "voices": [
      {
        "id": "nova",
        "name": "Nova",
        "language": "en",
        "gender": "female",
        "description": "Clear and natural female voice"
      },
      {
        "id": "echo",
        "name": "Echo", 
        "language": "en",
        "gender": "male",
        "description": "Warm and engaging male voice"
      }
    ]
  }
}
```

## Plugin Endpoints

### GET /api/v1/plugins

List installed plugins.

**Response:**
```json
{
  "success": true,
  "data": {
    "plugins": [
      {
        "id": "weather-plugin",
        "name": "Weather Plugin",
        "version": "1.0.0",
        "status": "active",
        "description": "Provides weather information",
        "capabilities": ["weather_query", "weather_forecast"]
      }
    ]
  }
}
```

### POST /api/v1/plugins/install

Install a new plugin.

**Request:**
```json
{
  "name": "calendar-plugin",
  "version": "1.0.0",
  "source": "https://plugins.example.com/calendar-plugin-1.0.0.wasm"
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "plugin_id": "calendar-plugin",
    "status": "installed",
    "message": "Plugin installed successfully"
  }
}
```

### POST /api/v1/plugins/{plugin_id}/enable

Enable a plugin.

**Response:**
```json
{
  "success": true,
  "data": {
    "plugin_id": "calendar-plugin",
    "status": "active",
    "message": "Plugin enabled successfully"
  }
}
```

### POST /api/v1/plugins/{plugin_id}/disable

Disable a plugin.

**Response:**
```json
{
  "success": true,
  "data": {
    "plugin_id": "calendar-plugin",
    "status": "inactive",
    "message": "Plugin disabled successfully"
  }
}
```

### DELETE /api/v1/plugins/{plugin_id}

Uninstall a plugin.

**Response:**
```json
{
  "success": true,
  "data": {
    "message": "Plugin uninstalled successfully"
  }
}
```

## Knowledge Base Endpoints

### POST /api/v1/knowledge/documents

Upload a document to the knowledge base.

**Request:**
- Content-Type: `multipart/form-data`
- Form field: `file` (document file)
- Form field: `metadata` (optional JSON metadata)

**Response:**
```json
{
  "success": true,
  "data": {
    "document_id": "123e4567-e89b-12d3-a456-426614174000",
    "filename": "document.pdf",
    "size_bytes": 1024768,
    "content_type": "application/pdf",
    "status": "processing"
  }
}
```

### GET /api/v1/knowledge/documents

List documents in knowledge base.

**Query Parameters:**
- `limit` (optional): Number of documents to return (default: 50)
- `offset` (optional): Pagination offset (default: 0)
- `search` (optional): Search query

**Response:**
```json
{
  "success": true,
  "data": {
    "documents": [
      {
        "id": "123e4567-e89b-12d3-a456-426614174000",
        "filename": "document.pdf",
        "size_bytes": 1024768,
        "content_type": "application/pdf",
        "uploaded_at": "2024-01-15T10:30:00Z",
        "status": "processed",
        "metadata": {
          "title": "Technical Documentation",
          "author": "John Doe"
        }
      }
    ],
    "total": 1,
    "has_more": false
  }
}
```

### POST /api/v1/knowledge/search

Search the knowledge base.

**Request:**
```json
{
  "query": "How to configure the API?",
  "limit": 10,
  "filters": {
    "content_type": "application/pdf"
  }
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "results": [
      {
        "document_id": "123e4567-e89b-12d3-a456-426614174000",
        "title": "API Configuration Guide",
        "snippet": "To configure the API, you need to set the following environment variables...",
        "relevance_score": 0.92,
        "page_number": 5
      }
    ],
    "total": 1,
    "query_time_ms": 45
  }
}
```

### DELETE /api/v1/knowledge/documents/{document_id}

Delete a document from the knowledge base.

**Response:**
```json
{
  "success": true,
  "data": {
    "message": "Document deleted successfully"
  }
}
```

## Task Management Endpoints

### POST /api/v1/tasks

Create a new task.

**Request:**
```json
{
  "title": "Review quarterly reports",
  "description": "Review and analyze Q4 financial reports",
  "due_date": "2024-01-20T17:00:00Z",
  "priority": "high",
  "tags": ["finance", "review"]
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "id": "123e4567-e89b-12d3-a456-426614174000",
    "title": "Review quarterly reports",
    "description": "Review and analyze Q4 financial reports",
    "status": "pending",
    "priority": "high",
    "due_date": "2024-01-20T17:00:00Z",
    "created_at": "2024-01-15T10:30:00Z",
    "tags": ["finance", "review"]
  }
}
```

### GET /api/v1/tasks

List tasks.

**Query Parameters:**
- `status` (optional): Filter by status (pending, in_progress, completed)
- `priority` (optional): Filter by priority (low, medium, high)
- `limit` (optional): Number of tasks to return (default: 50)
- `offset` (optional): Pagination offset (default: 0)

**Response:**
```json
{
  "success": true,
  "data": {
    "tasks": [
      {
        "id": "123e4567-e89b-12d3-a456-426614174000",
        "title": "Review quarterly reports",
        "description": "Review and analyze Q4 financial reports",
        "status": "pending",
        "priority": "high",
        "due_date": "2024-01-20T17:00:00Z",
        "created_at": "2024-01-15T10:30:00Z",
        "tags": ["finance", "review"]
      }
    ],
    "total": 1,
    "has_more": false
  }
}
```

### PUT /api/v1/tasks/{task_id}

Update a task.

**Request:**
```json
{
  "status": "completed",
  "notes": "Reports reviewed and analysis complete"
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "id": "123e4567-e89b-12d3-a456-426614174000",
    "title": "Review quarterly reports",
    "description": "Review and analyze Q4 financial reports",
    "status": "completed",
    "priority": "high",
    "due_date": "2024-01-20T17:00:00Z",
    "created_at": "2024-01-15T10:30:00Z",
    "completed_at": "2024-01-16T14:30:00Z",
    "notes": "Reports reviewed and analysis complete",
    "tags": ["finance", "review"]
  }
}
```

### DELETE /api/v1/tasks/{task_id}

Delete a task.

**Response:**
```json
{
  "success": true,
  "data": {
    "message": "Task deleted successfully"
  }
}
```

## Briefing Endpoints

### GET /api/v1/briefing/daily

Get daily briefing.

**Query Parameters:**
- `date` (optional): Date for briefing (YYYY-MM-DD, defaults to today)

**Response:**
```json
{
  "success": true,
  "data": {
    "date": "2024-01-15",
    "summary": "Good morning! Here's your briefing for today.",
    "weather": {
      "location": "San Francisco",
      "temperature": 68,
      "condition": "Partly cloudy",
      "forecast": "Sunny with highs of 72°F"
    },
    "calendar": {
      "events_today": 3,
      "next_event": {
        "title": "Team standup",
        "time": "09:00",
        "location": "Conference Room A"
      }
    },
    "tasks": {
      "due_today": 2,
      "overdue": 0,
      "urgent": 1
    },
    "news": [
      {
        "title": "Tech industry update",
        "summary": "Latest developments in AI and machine learning",
        "url": "https://example.com/news/1"
      }
    ]
  }
}
```

### POST /api/v1/briefing/generate

Generate a custom briefing.

**Request:**
```json
{
  "components": ["weather", "calendar", "tasks", "news"],
  "preferences": {
    "location": "San Francisco",
    "news_categories": ["technology", "business"],
    "detail_level": "summary"
  }
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "briefing": "Here's your custom briefing...",
    "components": {
      "weather": { /* weather data */ },
      "calendar": { /* calendar data */ },
      "tasks": { /* tasks data */ },
      "news": [ /* news items */ ]
    },
    "generated_at": "2024-01-15T10:30:00Z"
  }
}
```

## WebSocket API

The WebSocket endpoint provides real-time bidirectional communication.

### Connection

**Endpoint:** `ws://localhost:8080/ws` (development) or `wss://api.yourdomain.com/ws` (production)

**Authentication:** Include access token in query parameter: `?token=<access_token>`

### Message Format

All WebSocket messages follow this format:

```json
{
  "message_type": "Chat|VoiceData|StatusUpdate|Error|Ping|Pong",
  "session_id": "123e4567-e89b-12d3-a456-426614174000",
  "user_id": "123e4567-e89b-12d3-a456-426614174000",
  "data": { /* message-specific data */ },
  "timestamp": "2024-01-15T10:30:00Z"
}
```

### Message Types

#### Chat Message

**Client to Server:**
```json
{
  "message_type": "Chat",
  "session_id": "123e4567-e89b-12d3-a456-426614174000",
  "user_id": "123e4567-e89b-12d3-a456-426614174000",
  "data": "What's the weather like today?",
  "timestamp": "2024-01-15T10:30:00Z"
}
```

**Server to Client:**
```json
{
  "message_type": "Chat",
  "session_id": "123e4567-e89b-12d3-a456-426614174000",
  "user_id": "123e4567-e89b-12d3-a456-426614174000",
  "data": {
    "response": "The current weather in San Francisco is 68°F with partly cloudy skies.",
    "intent": "weather_query",
    "confidence": 0.95
  },
  "timestamp": "2024-01-15T10:30:01Z"
}
```

#### Voice Data

**Client to Server (Binary):**
Raw audio data sent as binary message.

**Server to Client:**
```json
{
  "message_type": "VoiceData",
  "session_id": "123e4567-e89b-12d3-a456-426614174000",
  "user_id": "123e4567-e89b-12d3-a456-426614174000",
  "data": {
    "transcription": "What's the weather like today?",
    "confidence": 0.92
  },
  "timestamp": "2024-01-15T10:30:00Z"
}
```

#### Status Update

**Server to Client:**
```json
{
  "message_type": "StatusUpdate",
  "session_id": "123e4567-e89b-12d3-a456-426614174000",
  "user_id": "123e4567-e89b-12d3-a456-426614174000",
  "data": {
    "status": "processing",
    "message": "Analyzing your request..."
  },
  "timestamp": "2024-01-15T10:30:00Z"
}
```

#### Ping/Pong

**Client to Server:**
```json
{
  "message_type": "Ping",
  "session_id": "123e4567-e89b-12d3-a456-426614174000",
  "user_id": "123e4567-e89b-12d3-a456-426614174000",
  "data": {},
  "timestamp": "2024-01-15T10:30:00Z"
}
```

**Server to Client:**
```json
{
  "message_type": "Pong",
  "session_id": "123e4567-e89b-12d3-a456-426614174000",
  "user_id": "123e4567-e89b-12d3-a456-426614174000",
  "data": {},
  "timestamp": "2024-01-15T10:30:01Z"
}
```

### Connection Events

- **Connection Established**: Server acknowledges successful connection
- **Connection Closed**: Either party closes the connection
- **Error**: Server sends error message if something goes wrong

## Rate Limiting

The API implements rate limiting to ensure fair usage:

- **Authentication endpoints**: 5 requests per minute per IP
- **General API endpoints**: 100 requests per minute per user
- **Voice endpoints**: 20 requests per minute per user
- **WebSocket messages**: 60 messages per minute per connection

Rate limit headers are included in responses:
- `X-RateLimit-Limit`: Request limit per window
- `X-RateLimit-Remaining`: Requests remaining in current window
- `X-RateLimit-Reset`: Time when the rate limit resets (Unix timestamp)

## Examples

### Complete Authentication Flow

```javascript
// 1. Login
const loginResponse = await fetch('/auth/login', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    email: 'user@example.com',
    password: 'password'
  })
});

const { data } = await loginResponse.json();
const { access_token, refresh_token } = data;

// 2. Make authenticated request
const response = await fetch('/api/v1/conversation/send', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
    'Authorization': `Bearer ${access_token}`
  },
  body: JSON.stringify({
    message: 'Hello, AI!',
    session_id: '123e4567-e89b-12d3-a456-426614174000'
  })
});

// 3. Handle token refresh (when access token expires)
const refreshResponse = await fetch('/auth/refresh', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    refresh_token: refresh_token
  })
});

const refreshData = await refreshResponse.json();
const newAccessToken = refreshData.data.access_token;
```

### WebSocket Connection

```javascript
// Connect to WebSocket
const ws = new WebSocket(`ws://localhost:8080/ws?token=${access_token}`);

// Handle connection events
ws.onopen = () => {
  console.log('WebSocket connected');
  
  // Send a chat message
  ws.send(JSON.stringify({
    message_type: 'Chat',
    session_id: '123e4567-e89b-12d3-a456-426614174000',
    user_id: '123e4567-e89b-12d3-a456-426614174000',
    data: 'Hello via WebSocket!',
    timestamp: new Date().toISOString()
  }));
};

ws.onmessage = (event) => {
  const message = JSON.parse(event.data);
  console.log('Received:', message);
  
  if (message.message_type === 'Chat') {
    console.log('AI Response:', message.data.response);
  }
};

ws.onerror = (error) => {
  console.error('WebSocket error:', error);
};

ws.onclose = () => {
  console.log('WebSocket disconnected');
};
```

### File Upload Example

```javascript
// Upload document to knowledge base
const formData = new FormData();
formData.append('file', file);
formData.append('metadata', JSON.stringify({
  title: 'My Document',
  category: 'reference'
}));

const uploadResponse = await fetch('/api/v1/knowledge/documents', {
  method: 'POST',
  headers: {
    'Authorization': `Bearer ${access_token}`
  },
  body: formData
});

const uploadResult = await uploadResponse.json();
console.log('Document uploaded:', uploadResult.data.document_id);
```

This API documentation provides a comprehensive guide for integrating with the Personal AI Assistant. For additional support or questions, please refer to the project's documentation or contact the development team.