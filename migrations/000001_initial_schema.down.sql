-- Rollback script for initial schema
-- This will drop all tables and objects created in the up migration

-- Drop views first
DROP VIEW IF EXISTS conversation_stats;
DROP VIEW IF EXISTS user_task_summary;
DROP VIEW IF EXISTS active_users;

-- Drop triggers
DROP TRIGGER IF EXISTS decrease_conversation_message_count;
DROP TRIGGER IF EXISTS update_conversation_message_count;
DROP TRIGGER IF EXISTS update_api_integrations_timestamp;
DROP TRIGGER IF EXISTS update_plugin_configs_timestamp;
DROP TRIGGER IF EXISTS update_messages_timestamp;
DROP TRIGGER IF EXISTS update_conversations_timestamp;
DROP TRIGGER IF EXISTS update_tasks_timestamp;
DROP TRIGGER IF EXISTS update_documents_timestamp;
DROP TRIGGER IF EXISTS update_sessions_activity;
DROP TRIGGER IF EXISTS update_users_timestamp;

-- Drop indexes (SQLite will automatically drop them when tables are dropped, but being explicit)
DROP INDEX IF EXISTS idx_audit_log_resource;
DROP INDEX IF EXISTS idx_audit_log_timestamp;
DROP INDEX IF EXISTS idx_audit_log_action;
DROP INDEX IF EXISTS idx_audit_log_user_id;

DROP INDEX IF EXISTS idx_api_integrations_active;
DROP INDEX IF EXISTS idx_api_integrations_service;
DROP INDEX IF EXISTS idx_api_integrations_user_id;

DROP INDEX IF EXISTS idx_plugin_configs_enabled;
DROP INDEX IF EXISTS idx_plugin_configs_plugin_id;
DROP INDEX IF EXISTS idx_plugin_configs_user_id;

DROP INDEX IF EXISTS idx_messages_created_at;
DROP INDEX IF EXISTS idx_messages_role;
DROP INDEX IF EXISTS idx_messages_user_id;
DROP INDEX IF EXISTS idx_messages_conversation_id;

DROP INDEX IF EXISTS idx_conversations_archived;
DROP INDEX IF EXISTS idx_conversations_last_message;
DROP INDEX IF EXISTS idx_conversations_user_id;

DROP INDEX IF EXISTS idx_briefings_generated;
DROP INDEX IF EXISTS idx_briefings_scheduled;
DROP INDEX IF EXISTS idx_briefings_type;
DROP INDEX IF EXISTS idx_briefings_user_id;

DROP INDEX IF EXISTS idx_tasks_created_at;
DROP INDEX IF EXISTS idx_tasks_assigned_plugin;
DROP INDEX IF EXISTS idx_tasks_due_date;
DROP INDEX IF EXISTS idx_tasks_priority;
DROP INDEX IF EXISTS idx_tasks_status;
DROP INDEX IF EXISTS idx_tasks_user_id;

DROP INDEX IF EXISTS idx_documents_public;
DROP INDEX IF EXISTS idx_documents_created_at;
DROP INDEX IF EXISTS idx_documents_content_type;
DROP INDEX IF EXISTS idx_documents_user_id;

DROP INDEX IF EXISTS idx_sessions_active;
DROP INDEX IF EXISTS idx_sessions_expires;
DROP INDEX IF EXISTS idx_sessions_token;
DROP INDEX IF EXISTS idx_sessions_user_id;

DROP INDEX IF EXISTS idx_users_active;
DROP INDEX IF EXISTS idx_users_email;

-- Drop tables in reverse dependency order
DROP TABLE IF EXISTS audit_log;
DROP TABLE IF EXISTS api_integrations;
DROP TABLE IF EXISTS plugin_configs;
DROP TABLE IF EXISTS messages;
DROP TABLE IF EXISTS conversations;
DROP TABLE IF EXISTS briefings;
DROP TABLE IF EXISTS tasks;
DROP TABLE IF EXISTS documents;
DROP TABLE IF EXISTS sessions;
DROP TABLE IF EXISTS users;