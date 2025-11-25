DROP TRIGGER replies_counter_cache_trigger ON conversations;

DROP FUNCTION replies_counter_cache;

DROP INDEX conversations_by_id_and_user_idx;
DROP INDEX conversations_recent_by_channel_and_thread_idx;
DROP INDEX conversations_recent_by_channel_idx;

DROP TABLE conversations;
