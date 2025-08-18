-- Rollback script for search and logging migration

-- Drop views
DROP VIEW IF EXISTS document_search_ranking;
DROP VIEW IF EXISTS user_activity_summary;
DROP VIEW IF EXISTS plugin_performance_summary;

-- Drop triggers
DROP TRIGGER IF EXISTS cleanup_old_metrics;
DROP TRIGGER IF EXISTS update_user_preferences_extended_timestamp;
DROP TRIGGER IF EXISTS update_scheduled_jobs_timestamp;
DROP TRIGGER IF EXISTS update_knowledge_relationships_timestamp;

-- Drop FTS triggers
DROP TRIGGER IF EXISTS documents_fts_update;
DROP TRIGGER IF EXISTS documents_fts_delete;
DROP TRIGGER IF EXISTS documents_fts_insert;

-- Drop indexes
DROP INDEX IF EXISTS idx_user_preferences_extended_category;
DROP INDEX IF EXISTS idx_user_preferences_extended_user_id;

DROP INDEX IF EXISTS idx_system_metrics_timestamp;
DROP INDEX IF EXISTS idx_system_metrics_name;

DROP INDEX IF EXISTS idx_notifications_created_at;
DROP INDEX IF EXISTS idx_notifications_priority;
DROP INDEX IF EXISTS idx_notifications_read;
DROP INDEX IF EXISTS idx_notifications_type;
DROP INDEX IF EXISTS idx_notifications_user_id;

DROP INDEX IF EXISTS idx_scheduled_jobs_enabled;
DROP INDEX IF EXISTS idx_scheduled_jobs_next_run;
DROP INDEX IF EXISTS idx_scheduled_jobs_type;
DROP INDEX IF EXISTS idx_scheduled_jobs_user_id;

DROP INDEX IF EXISTS idx_knowledge_relationships_strength;
DROP INDEX IF EXISTS idx_knowledge_relationships_type;
DROP INDEX IF EXISTS idx_knowledge_relationships_target;
DROP INDEX IF EXISTS idx_knowledge_relationships_source;
DROP INDEX IF EXISTS idx_knowledge_relationships_user_id;

DROP INDEX IF EXISTS idx_plugin_executions_execution_time;
DROP INDEX IF EXISTS idx_plugin_executions_started_at;
DROP INDEX IF EXISTS idx_plugin_executions_status;
DROP INDEX IF EXISTS idx_plugin_executions_plugin_id;
DROP INDEX IF EXISTS idx_plugin_executions_user_id;

-- Drop tables
DROP TABLE IF EXISTS user_preferences_extended;
DROP TABLE IF EXISTS system_metrics;
DROP TABLE IF EXISTS notifications;
DROP TABLE IF EXISTS scheduled_jobs;
DROP TABLE IF EXISTS knowledge_relationships;
DROP TABLE IF EXISTS plugin_executions;

-- Drop FTS table
DROP TABLE IF EXISTS documents_fts;