DROP TRIGGER reactions_counter_cache_trigger ON reactions;

DROP FUNCTION reactions_counter_cache;

DROP INDEX reactions_message_id_idx;
DROP INDEX reactions_recent_by_message_idx;
DROP INDEX reactions_distinct_by_message_idx;

DROP TABLE reactions;
