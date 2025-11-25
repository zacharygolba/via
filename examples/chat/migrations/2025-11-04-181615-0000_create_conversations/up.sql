CREATE TABLE conversations (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

  channel_id UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
  thread_id UUID REFERENCES conversations(id) ON DELETE CASCADE,
  user_id UUID NOT NULL REFERENCES users(id),

  body TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT current_timestamp,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT current_timestamp,
  total_reactions BIGINT NOT NULL DEFAULT 0,
  total_replies BIGINT NOT NULL DEFAULT 0
);

SELECT diesel_manage_updated_at('conversations');

-- Used to list the conversations in a channel.
CREATE INDEX conversations_recent_by_channel_idx
ON conversations (channel_id, created_at DESC, id DESC);

-- Used to list the conversations in a thread.
CREATE INDEX conversations_recent_by_channel_and_thread_idx
ON conversations (channel_id, thread_id, created_at DESC, id DESC);

-- Used to update or delete a user's conversation.
CREATE INDEX conversations_by_id_and_user_idx
ON conversations (id, user_id);

-- Update the total_replies for a conversation.
CREATE FUNCTION replies_counter_cache()
RETURNS trigger AS $$
BEGIN
  -- INSERT: increment total_replies on conversations
  IF TG_OP = 'INSERT' THEN
    UPDATE conversations
    SET total_replies = total_replies + 1
    WHERE id = NEW.thread_id;
    RETURN NEW;

  -- DELETE: decrement total_replies on conversations
  ELSIF TG_OP = 'DELETE' THEN
    UPDATE conversations
    SET total_replies = total_replies - 1
    WHERE id = OLD.thread_id;
    RETURN OLD;
  END IF;

  RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER replies_counter_cache_trigger
AFTER INSERT OR DELETE ON conversations
FOR EACH ROW
EXECUTE FUNCTION replies_counter_cache();

