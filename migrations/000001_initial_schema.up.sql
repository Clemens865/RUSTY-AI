-- Initial schema for Personal AI Assistant
-- This creates the core tables needed for the application

-- Users table for authentication and user management
CREATE TABLE users (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    email TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    full_name TEXT NOT NULL,
    avatar_url TEXT,
    preferences TEXT DEFAULT '{}', -- JSON preferences
    subscription_tier TEXT DEFAULT 'free' CHECK (subscription_tier IN ('free', 'premium', 'enterprise')),
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    last_login DATETIME,
    is_active BOOLEAN DEFAULT TRUE,
    email_verified BOOLEAN DEFAULT FALSE,
    timezone TEXT DEFAULT 'UTC'
);

-- Sessions table for user session management
CREATE TABLE sessions (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    session_token TEXT UNIQUE NOT NULL,
    ip_address TEXT,
    user_agent TEXT,
    expires_at DATETIME NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    last_activity DATETIME DEFAULT CURRENT_TIMESTAMP,
    is_active BOOLEAN DEFAULT TRUE
);

-- Documents table for knowledge base and file management
CREATE TABLE documents (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    content TEXT,
    content_type TEXT DEFAULT 'text/plain',
    file_path TEXT,
    file_size INTEGER DEFAULT 0,
    checksum TEXT,
    tags TEXT DEFAULT '[]', -- JSON array of tags
    metadata TEXT DEFAULT '{}', -- JSON metadata
    embedding_vector BLOB, -- Vector embeddings for semantic search
    is_public BOOLEAN DEFAULT FALSE,
    is_encrypted BOOLEAN DEFAULT FALSE,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    last_accessed DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Tasks table for task management and automation
CREATE TABLE tasks (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    description TEXT,
    status TEXT DEFAULT 'pending' CHECK (status IN ('pending', 'in_progress', 'completed', 'cancelled', 'failed')),
    priority TEXT DEFAULT 'medium' CHECK (priority IN ('low', 'medium', 'high', 'urgent')),
    category TEXT DEFAULT 'general',
    due_date DATETIME,
    estimated_duration INTEGER, -- in minutes
    actual_duration INTEGER, -- in minutes
    assigned_plugin TEXT, -- plugin responsible for the task
    task_data TEXT DEFAULT '{}', -- JSON task configuration
    result_data TEXT DEFAULT '{}', -- JSON task results
    tags TEXT DEFAULT '[]', -- JSON array of tags
    dependencies TEXT DEFAULT '[]', -- JSON array of task IDs this depends on
    recurrence_rule TEXT, -- RRULE for recurring tasks
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    started_at DATETIME,
    completed_at DATETIME
);

-- Briefings table for daily/weekly summaries and reports
CREATE TABLE briefings (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    briefing_type TEXT DEFAULT 'daily' CHECK (briefing_type IN ('daily', 'weekly', 'monthly', 'custom')),
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    summary TEXT,
    format TEXT DEFAULT 'markdown' CHECK (format IN ('markdown', 'html', 'plain')),
    data_sources TEXT DEFAULT '[]', -- JSON array of data sources used
    metrics TEXT DEFAULT '{}', -- JSON metrics and KPIs
    attachments TEXT DEFAULT '[]', -- JSON array of attachment references
    is_read BOOLEAN DEFAULT FALSE,
    scheduled_for DATETIME,
    generated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    expires_at DATETIME,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Conversations table for chat history and context
CREATE TABLE conversations (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title TEXT,
    context_summary TEXT,
    message_count INTEGER DEFAULT 0,
    total_tokens INTEGER DEFAULT 0,
    last_message_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    is_archived BOOLEAN DEFAULT FALSE,
    tags TEXT DEFAULT '[]', -- JSON array of tags
    metadata TEXT DEFAULT '{}', -- JSON conversation metadata
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Messages table for individual conversation messages
CREATE TABLE messages (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system', 'tool')),
    content TEXT NOT NULL,
    content_type TEXT DEFAULT 'text' CHECK (content_type IN ('text', 'image', 'audio', 'file')),
    tokens INTEGER DEFAULT 0,
    tool_calls TEXT DEFAULT '[]', -- JSON array of tool calls
    tool_results TEXT DEFAULT '[]', -- JSON array of tool results
    metadata TEXT DEFAULT '{}', -- JSON message metadata
    is_edited BOOLEAN DEFAULT FALSE,
    edit_history TEXT DEFAULT '[]', -- JSON array of previous versions
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Plugin configurations table
CREATE TABLE plugin_configs (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    plugin_id TEXT NOT NULL,
    plugin_name TEXT NOT NULL,
    version TEXT NOT NULL,
    is_enabled BOOLEAN DEFAULT TRUE,
    configuration TEXT DEFAULT '{}', -- JSON plugin configuration
    permissions TEXT DEFAULT '[]', -- JSON array of granted permissions
    resource_limits TEXT DEFAULT '{}', -- JSON resource limits
    execution_stats TEXT DEFAULT '{}', -- JSON execution statistics
    last_error TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    last_used DATETIME,
    UNIQUE(user_id, plugin_id)
);

-- API keys and integrations table
CREATE TABLE api_integrations (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    service_name TEXT NOT NULL,
    api_key_hash TEXT NOT NULL, -- encrypted API key
    configuration TEXT DEFAULT '{}', -- JSON service configuration
    is_active BOOLEAN DEFAULT TRUE,
    rate_limit_remaining INTEGER DEFAULT 0,
    rate_limit_reset DATETIME,
    last_used DATETIME,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, service_name)
);

-- Audit log for security and compliance
CREATE TABLE audit_log (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    user_id TEXT REFERENCES users(id) ON DELETE SET NULL,
    action TEXT NOT NULL,
    resource_type TEXT NOT NULL,
    resource_id TEXT,
    details TEXT DEFAULT '{}', -- JSON details
    ip_address TEXT,
    user_agent TEXT,
    success BOOLEAN DEFAULT TRUE,
    error_message TEXT,
    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Indexes for performance
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_active ON users(is_active) WHERE is_active = TRUE;

CREATE INDEX idx_sessions_user_id ON sessions(user_id);
CREATE INDEX idx_sessions_token ON sessions(session_token);
CREATE INDEX idx_sessions_expires ON sessions(expires_at);
CREATE INDEX idx_sessions_active ON sessions(is_active) WHERE is_active = TRUE;

CREATE INDEX idx_documents_user_id ON documents(user_id);
CREATE INDEX idx_documents_content_type ON documents(content_type);
CREATE INDEX idx_documents_created_at ON documents(created_at);
CREATE INDEX idx_documents_public ON documents(is_public) WHERE is_public = TRUE;

CREATE INDEX idx_tasks_user_id ON tasks(user_id);
CREATE INDEX idx_tasks_status ON tasks(status);
CREATE INDEX idx_tasks_priority ON tasks(priority);
CREATE INDEX idx_tasks_due_date ON tasks(due_date);
CREATE INDEX idx_tasks_assigned_plugin ON tasks(assigned_plugin);
CREATE INDEX idx_tasks_created_at ON tasks(created_at);

CREATE INDEX idx_briefings_user_id ON briefings(user_id);
CREATE INDEX idx_briefings_type ON briefings(briefing_type);
CREATE INDEX idx_briefings_scheduled ON briefings(scheduled_for);
CREATE INDEX idx_briefings_generated ON briefings(generated_at);

CREATE INDEX idx_conversations_user_id ON conversations(user_id);
CREATE INDEX idx_conversations_last_message ON conversations(last_message_at);
CREATE INDEX idx_conversations_archived ON conversations(is_archived);

CREATE INDEX idx_messages_conversation_id ON messages(conversation_id);
CREATE INDEX idx_messages_user_id ON messages(user_id);
CREATE INDEX idx_messages_role ON messages(role);
CREATE INDEX idx_messages_created_at ON messages(created_at);

CREATE INDEX idx_plugin_configs_user_id ON plugin_configs(user_id);
CREATE INDEX idx_plugin_configs_plugin_id ON plugin_configs(plugin_id);
CREATE INDEX idx_plugin_configs_enabled ON plugin_configs(is_enabled) WHERE is_enabled = TRUE;

CREATE INDEX idx_api_integrations_user_id ON api_integrations(user_id);
CREATE INDEX idx_api_integrations_service ON api_integrations(service_name);
CREATE INDEX idx_api_integrations_active ON api_integrations(is_active) WHERE is_active = TRUE;

CREATE INDEX idx_audit_log_user_id ON audit_log(user_id);
CREATE INDEX idx_audit_log_action ON audit_log(action);
CREATE INDEX idx_audit_log_timestamp ON audit_log(timestamp);
CREATE INDEX idx_audit_log_resource ON audit_log(resource_type, resource_id);

-- Triggers for updating timestamps
CREATE TRIGGER update_users_timestamp 
    AFTER UPDATE ON users 
    FOR EACH ROW 
    WHEN NEW.updated_at = OLD.updated_at
BEGIN
    UPDATE users SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

CREATE TRIGGER update_sessions_activity 
    AFTER UPDATE ON sessions 
    FOR EACH ROW 
BEGIN
    UPDATE sessions SET last_activity = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

CREATE TRIGGER update_documents_timestamp 
    AFTER UPDATE ON documents 
    FOR EACH ROW 
    WHEN NEW.updated_at = OLD.updated_at
BEGIN
    UPDATE documents SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

CREATE TRIGGER update_tasks_timestamp 
    AFTER UPDATE ON tasks 
    FOR EACH ROW 
    WHEN NEW.updated_at = OLD.updated_at
BEGIN
    UPDATE tasks SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

CREATE TRIGGER update_conversations_timestamp 
    AFTER UPDATE ON conversations 
    FOR EACH ROW 
    WHEN NEW.updated_at = OLD.updated_at
BEGIN
    UPDATE conversations SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

CREATE TRIGGER update_messages_timestamp 
    AFTER UPDATE ON messages 
    FOR EACH ROW 
    WHEN NEW.updated_at = OLD.updated_at
BEGIN
    UPDATE messages SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

CREATE TRIGGER update_plugin_configs_timestamp 
    AFTER UPDATE ON plugin_configs 
    FOR EACH ROW 
    WHEN NEW.updated_at = OLD.updated_at
BEGIN
    UPDATE plugin_configs SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

CREATE TRIGGER update_api_integrations_timestamp 
    AFTER UPDATE ON api_integrations 
    FOR EACH ROW 
    WHEN NEW.updated_at = OLD.updated_at
BEGIN
    UPDATE api_integrations SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

-- Trigger to update conversation message count
CREATE TRIGGER update_conversation_message_count
    AFTER INSERT ON messages
    FOR EACH ROW
BEGIN
    UPDATE conversations 
    SET message_count = message_count + 1,
        last_message_at = CURRENT_TIMESTAMP
    WHERE id = NEW.conversation_id;
END;

-- Trigger to decrease conversation message count on delete
CREATE TRIGGER decrease_conversation_message_count
    AFTER DELETE ON messages
    FOR EACH ROW
BEGIN
    UPDATE conversations 
    SET message_count = message_count - 1
    WHERE id = OLD.conversation_id;
END;

-- Views for common queries
CREATE VIEW active_users AS
SELECT u.*, 
       COUNT(DISTINCT s.id) as active_sessions,
       MAX(s.last_activity) as last_session_activity
FROM users u
LEFT JOIN sessions s ON u.id = s.user_id AND s.is_active = TRUE
WHERE u.is_active = TRUE
GROUP BY u.id, u.email, u.full_name, u.created_at, u.updated_at, u.last_login, u.subscription_tier;

CREATE VIEW user_task_summary AS
SELECT u.id as user_id,
       u.full_name,
       COUNT(CASE WHEN t.status = 'pending' THEN 1 END) as pending_tasks,
       COUNT(CASE WHEN t.status = 'in_progress' THEN 1 END) as active_tasks,
       COUNT(CASE WHEN t.status = 'completed' THEN 1 END) as completed_tasks,
       COUNT(CASE WHEN t.status = 'failed' THEN 1 END) as failed_tasks
FROM users u
LEFT JOIN tasks t ON u.id = t.user_id
WHERE u.is_active = TRUE
GROUP BY u.id, u.full_name;

CREATE VIEW conversation_stats AS
SELECT c.id,
       c.user_id,
       c.title,
       c.message_count,
       c.total_tokens,
       c.last_message_at,
       COALESCE(SUM(m.tokens), 0) as calculated_tokens,
       COUNT(m.id) as calculated_messages
FROM conversations c
LEFT JOIN messages m ON c.id = m.conversation_id
GROUP BY c.id, c.user_id, c.title, c.message_count, c.total_tokens, c.last_message_at;