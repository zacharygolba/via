CREATE TABLE reactions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

  emoji VARCHAR(16) NOT NULL,

  conversation_id UUID NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
  user_id UUID NOT NULL REFERENCES users(id),

  created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

SELECT diesel_manage_updated_at('reactions');

CREATE INDEX reactions_recent_by_conversation_idx
ON reactions (conversation_id, created_at DESC, id);

CREATE INDEX reactions_distinct_by_conversation_idx
ON reactions (emoji, conversation_id);

CREATE INDEX reactions_distinct_by_conversation_sorted_idx
ON reactions (emoji, conversation_id, created_at, id)
INCLUDE (user_id);

-- Update the total_reactions for a conversation.
CREATE FUNCTION reactions_counter_cache()
RETURNS trigger AS $$
BEGIN
  -- INSERT: increment total_reactions on conversations
  IF TG_OP = 'INSERT' THEN
    UPDATE conversations
    SET total_reactions = total_reactions + 1
    WHERE id = NEW.conversation_id;
    RETURN NEW;

  -- DELETE: decrement total_reactions on conversations
  ELSIF TG_OP = 'DELETE' THEN
    UPDATE conversations
    SET total_reactions = total_reactions - 1
    WHERE id = OLD.conversation_id;
    RETURN OLD;
  END IF;

  RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE FUNCTION top_reactions_for(
  conversation_ids uuid[],
  distinct_emoji_count int,
  max_usernames_per_emoji int
) RETURNS TABLE (
    conversation_id uuid,
    emoji text,
    usernames text[],
    total_count bigint
) AS $$
  WITH reaction_counts AS (
    SELECT
      conversation_id,
      emoji,
      COUNT(*) AS total_count
    FROM reactions
    WHERE conversation_id = ANY(conversation_ids)
    GROUP BY conversation_id, emoji
  ),
  ranked_reactions AS (
    SELECT
      conversation_id,
      emoji,
      total_count,
      ROW_NUMBER() OVER (
        PARTITION BY conversation_id
        ORDER BY total_count DESC, emoji
      ) AS rn
    FROM reaction_counts
  )
  SELECT
    rr.conversation_id,
    rr.emoji,
    (
      SELECT ARRAY_AGG(u.username ORDER BY u.username, u.id)
      FROM reactions r
      JOIN users u ON u.id = r.user_id
      WHERE r.conversation_id = rr.conversation_id
        AND r.emoji = rr.emoji
      LIMIT max_usernames_per_emoji
    ) AS usernames,
    rr.total_count
  FROM ranked_reactions rr
  WHERE rr.rn <= distinct_emoji_count
  ORDER BY rr.total_count DESC, rr.conversation_id;
$$ LANGUAGE SQL STABLE;

CREATE TRIGGER reactions_counter_cache_trigger
AFTER INSERT OR DELETE ON reactions
FOR EACH ROW
EXECUTE FUNCTION reactions_counter_cache();


