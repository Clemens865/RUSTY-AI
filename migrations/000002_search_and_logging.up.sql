-- Second migration: Add full-text search and enhanced logging capabilities

-- Full-text search virtual table for documents
CREATE VIRTUAL TABLE documents_fts USING fts5(
    title,
    content,
    tags,
    content='documents',
    content_rowid='rowid'
);

-- Populate initial FTS data
INSERT INTO documents_fts(rowid, title, content, tags)
SELECT rowid, title, COALESCE(content, ''), tags FROM documents;

-- Trigger to keep FTS table in sync with documents
CREATE TRIGGER documents_fts_insert AFTER INSERT ON documents BEGIN
    INSERT INTO documents_fts(rowid, title, content, tags) 
    VALUES (NEW.rowid, NEW.title, COALESCE(NEW.content, ''), NEW.tags);
END;

CREATE TRIGGER documents_fts_delete AFTER DELETE ON documents BEGIN
    DELETE FROM documents_fts WHERE rowid = OLD.rowid;
END;

CREATE TRIGGER documents_fts_update AFTER UPDATE ON documents BEGIN
    DELETE FROM documents_fts WHERE rowid = OLD.rowid;
    INSERT INTO documents_fts(rowid, title, content, tags) 
    VALUES (NEW.rowid, NEW.title, COALESCE(NEW.content, ''), NEW.tags);
END;

-- Plugin execution logs table for detailed monitoring
CREATE TABLE plugin_executions (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    plugin_id TEXT NOT NULL,
    function_name TEXT NOT NULL,
    input_data TEXT, -- JSON input parameters
    output_data TEXT, -- JSON output results
    execution_time_ms INTEGER NOT NULL,
    memory_used_bytes INTEGER DEFAULT 0,
    cpu_time_ms INTEGER DEFAULT 0,
    status TEXT NOT NULL CHECK (status IN ('success', 'error', 'timeout', 'cancelled')),
    error_message TEXT,
    error_stack TEXT,
    context_data TEXT DEFAULT '{}', -- JSON execution context
    session_id TEXT,
    request_id TEXT,
    started_at DATETIME NOT NULL,
    completed_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Knowledge graph relationships table
CREATE TABLE knowledge_relationships (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    source_document_id TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    target_document_id TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    relationship_type TEXT NOT NULL DEFAULT 'related',
    relationship_strength REAL DEFAULT 1.0 CHECK (relationship_strength >= 0.0 AND relationship_strength <= 1.0),
    metadata TEXT DEFAULT '{}', -- JSON relationship metadata
    created_by TEXT DEFAULT 'system', -- 'system' or 'user'
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(source_document_id, target_document_id, relationship_type)
);

-- Scheduled jobs table for background tasks
CREATE TABLE scheduled_jobs (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    user_id TEXT REFERENCES users(id) ON DELETE CASCADE,
    job_type TEXT NOT NULL,
    job_name TEXT NOT NULL,
    job_data TEXT DEFAULT '{}', -- JSON job configuration
    cron_expression TEXT,
    next_run_at DATETIME,
    last_run_at DATETIME,
    last_result TEXT DEFAULT '{}', -- JSON last execution result
    is_enabled BOOLEAN DEFAULT TRUE,
    max_retries INTEGER DEFAULT 3,
    retry_count INTEGER DEFAULT 0,
    timeout_seconds INTEGER DEFAULT 300,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Notifications table for user alerts and updates
CREATE TABLE notifications (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    notification_type TEXT NOT NULL DEFAULT 'info' CHECK (notification_type IN ('info', 'warning', 'error', 'success')),
    title TEXT NOT NULL,
    message TEXT NOT NULL,
    action_url TEXT,
    action_text TEXT,
    metadata TEXT DEFAULT '{}', -- JSON notification metadata
    is_read BOOLEAN DEFAULT FALSE,
    is_dismissed BOOLEAN DEFAULT FALSE,
    priority TEXT DEFAULT 'normal' CHECK (priority IN ('low', 'normal', 'high', 'urgent')),
    expires_at DATETIME,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    read_at DATETIME,
    dismissed_at DATETIME
);

-- System metrics table for monitoring and analytics
CREATE TABLE system_metrics (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    metric_name TEXT NOT NULL,
    metric_value REAL NOT NULL,
    metric_unit TEXT DEFAULT '',
    dimensions TEXT DEFAULT '{}', -- JSON key-value pairs for dimensions
    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
    retention_days INTEGER DEFAULT 30
);

-- User preferences extension table
CREATE TABLE user_preferences_extended (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    category TEXT NOT NULL,
    preference_key TEXT NOT NULL,
    preference_value TEXT NOT NULL,
    value_type TEXT DEFAULT 'string' CHECK (value_type IN ('string', 'number', 'boolean', 'json')),
    description TEXT,
    is_sensitive BOOLEAN DEFAULT FALSE,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, category, preference_key)
);

-- Indexes for new tables
CREATE INDEX idx_plugin_executions_user_id ON plugin_executions(user_id);
CREATE INDEX idx_plugin_executions_plugin_id ON plugin_executions(plugin_id);
CREATE INDEX idx_plugin_executions_status ON plugin_executions(status);
CREATE INDEX idx_plugin_executions_started_at ON plugin_executions(started_at);
CREATE INDEX idx_plugin_executions_execution_time ON plugin_executions(execution_time_ms);

CREATE INDEX idx_knowledge_relationships_user_id ON knowledge_relationships(user_id);
CREATE INDEX idx_knowledge_relationships_source ON knowledge_relationships(source_document_id);
CREATE INDEX idx_knowledge_relationships_target ON knowledge_relationships(target_document_id);
CREATE INDEX idx_knowledge_relationships_type ON knowledge_relationships(relationship_type);
CREATE INDEX idx_knowledge_relationships_strength ON knowledge_relationships(relationship_strength);

CREATE INDEX idx_scheduled_jobs_user_id ON scheduled_jobs(user_id);
CREATE INDEX idx_scheduled_jobs_type ON scheduled_jobs(job_type);
CREATE INDEX idx_scheduled_jobs_next_run ON scheduled_jobs(next_run_at);
CREATE INDEX idx_scheduled_jobs_enabled ON scheduled_jobs(is_enabled) WHERE is_enabled = TRUE;

CREATE INDEX idx_notifications_user_id ON notifications(user_id);
CREATE INDEX idx_notifications_type ON notifications(notification_type);
CREATE INDEX idx_notifications_read ON notifications(is_read);
CREATE INDEX idx_notifications_priority ON notifications(priority);
CREATE INDEX idx_notifications_created_at ON notifications(created_at);

CREATE INDEX idx_system_metrics_name ON system_metrics(metric_name);
CREATE INDEX idx_system_metrics_timestamp ON system_metrics(timestamp);

CREATE INDEX idx_user_preferences_extended_user_id ON user_preferences_extended(user_id);
CREATE INDEX idx_user_preferences_extended_category ON user_preferences_extended(category);

-- Additional triggers for timestamp management
CREATE TRIGGER update_knowledge_relationships_timestamp 
    AFTER UPDATE ON knowledge_relationships 
    FOR EACH ROW 
    WHEN NEW.updated_at = OLD.updated_at
BEGIN
    UPDATE knowledge_relationships SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

CREATE TRIGGER update_scheduled_jobs_timestamp 
    AFTER UPDATE ON scheduled_jobs 
    FOR EACH ROW 
    WHEN NEW.updated_at = OLD.updated_at
BEGIN
    UPDATE scheduled_jobs SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

CREATE TRIGGER update_user_preferences_extended_timestamp 
    AFTER UPDATE ON user_preferences_extended 
    FOR EACH ROW 
    WHEN NEW.updated_at = OLD.updated_at
BEGIN
    UPDATE user_preferences_extended SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

-- Trigger to clean up old metrics based on retention policy
CREATE TRIGGER cleanup_old_metrics 
    AFTER INSERT ON system_metrics
    FOR EACH ROW
BEGIN
    DELETE FROM system_metrics 
    WHERE metric_name = NEW.metric_name 
    AND timestamp < datetime('now', '-' || NEW.retention_days || ' days');
END;

-- Enhanced views
CREATE VIEW plugin_performance_summary AS
SELECT 
    plugin_id,
    function_name,
    COUNT(*) as total_executions,
    COUNT(CASE WHEN status = 'success' THEN 1 END) as successful_executions,
    COUNT(CASE WHEN status = 'error' THEN 1 END) as failed_executions,
    COUNT(CASE WHEN status = 'timeout' THEN 1 END) as timeout_executions,
    AVG(execution_time_ms) as avg_execution_time_ms,
    MAX(execution_time_ms) as max_execution_time_ms,
    MIN(execution_time_ms) as min_execution_time_ms,
    AVG(CAST(memory_used_bytes AS REAL)) as avg_memory_used_bytes,
    MAX(memory_used_bytes) as max_memory_used_bytes
FROM plugin_executions
GROUP BY plugin_id, function_name;

CREATE VIEW user_activity_summary AS
SELECT 
    u.id as user_id,
    u.full_name,
    u.last_login,
    COUNT(DISTINCT pe.id) as plugin_executions_today,
    COUNT(DISTINCT t.id) as active_tasks,
    COUNT(DISTINCT c.id) as active_conversations,
    COUNT(DISTINCT n.id) as unread_notifications
FROM users u
LEFT JOIN plugin_executions pe ON u.id = pe.user_id 
    AND pe.started_at >= date('now', 'start of day')
LEFT JOIN tasks t ON u.id = t.user_id 
    AND t.status IN ('pending', 'in_progress')
LEFT JOIN conversations c ON u.id = c.user_id 
    AND c.is_archived = FALSE
LEFT JOIN notifications n ON u.id = n.user_id 
    AND n.is_read = FALSE
WHERE u.is_active = TRUE
GROUP BY u.id, u.full_name, u.last_login;

CREATE VIEW document_search_ranking AS
SELECT 
    d.id,
    d.user_id,
    d.title,
    d.content_type,
    d.created_at,
    d.last_accessed,
    COUNT(kr1.id) as outgoing_relationships,
    COUNT(kr2.id) as incoming_relationships,
    (COUNT(kr1.id) + COUNT(kr2.id)) as total_relationships
FROM documents d
LEFT JOIN knowledge_relationships kr1 ON d.id = kr1.source_document_id
LEFT JOIN knowledge_relationships kr2 ON d.id = kr2.target_document_id
GROUP BY d.id, d.user_id, d.title, d.content_type, d.created_at, d.last_accessed;