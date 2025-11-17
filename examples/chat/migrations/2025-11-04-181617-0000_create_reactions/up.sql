CREATE TABLE reactions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

  emoji VARCHAR(16) NOT NULL,

  message_id UUID NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
  user_id UUID NOT NULL REFERENCES users(id),

  created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

SELECT diesel_manage_updated_at('reactions');

CREATE INDEX reactions_message_id_idx ON reactions (message_id);

CREATE INDEX reactions_recent_by_message_idx
ON reactions (message_id, created_at DESC, id DESC);

CREATE INDEX reactions_distinct_by_message_idx
ON reactions (message_id, emoji, id);

-- Update the reactions_count for a message.
CREATE FUNCTION reactions_counter_cache()
RETURNS trigger AS $$
BEGIN
  -- INSERT: increment reactions_count on messages
  IF TG_OP = 'INSERT' THEN
    UPDATE messages
    SET reactions_count = reactions_count + 1
    WHERE id = NEW.message_id;
    RETURN NEW;

  -- DELETE: decrement parent
  ELSIF TG_OP = 'DELETE' THEN
    UPDATE messages
    SET reactions_count = reactions_count - 1
    WHERE id = OLD.message_id;
    RETURN OLD;
  END IF;

  RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER reactions_counter_cache_trigger
AFTER INSERT OR DELETE ON reactions
FOR EACH ROW
EXECUTE FUNCTION reactions_counter_cache();
