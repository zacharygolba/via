CREATE TABLE reactions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

  emoji VARCHAR(16) NOT NULL,

  message_id UUID NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
  user_id UUID NOT NULL REFERENCES users(id),

  created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

SELECT diesel_manage_updated_at('reactions');

CREATE INDEX reactions_message_id_idx
ON reactions (message_id);

CREATE INDEX reactions_recent_by_message_idx
ON reactions (message_id, created_at DESC, id);

CREATE INDEX reactions_distinct_by_message_idx
ON reactions (emoji, message_id);

CREATE INDEX reactions_distinct_by_message_sorted_idx
ON reactions (emoji, message_id, created_at, id)
INCLUDE (user_id);

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

CREATE FUNCTION top_reactions_for(
  message_ids uuid[],
  distinct_emoji_count int,
  max_usernames_per_emoji int
) RETURNS TABLE (
    message_id uuid,
    emoji text,
    usernames text[],
    total_count bigint
) AS $$
  WITH reaction_counts AS (
    SELECT
      message_id,
      emoji,
      COUNT(*) AS total_count
    FROM reactions
    WHERE message_id = ANY(message_ids)
    GROUP BY message_id, emoji
  ),
  ranked_reactions AS (
    SELECT
      message_id,
      emoji,
      total_count,
      ROW_NUMBER() OVER (
        PARTITION BY message_id
        ORDER BY total_count DESC, emoji
      ) AS rn
    FROM reaction_counts
  )
  SELECT
    rr.message_id,
    rr.emoji,
    (
      SELECT ARRAY_AGG(u.username ORDER BY u.username, u.id)
      FROM reactions r
      JOIN users u ON u.id = r.user_id
      WHERE r.message_id = rr.message_id
        AND r.emoji = rr.emoji
      LIMIT max_usernames_per_emoji
    ) AS usernames,
    rr.total_count
  FROM ranked_reactions rr
  WHERE rr.rn <= distinct_emoji_count
  ORDER BY rr.total_count DESC, rr.message_id;
$$ LANGUAGE SQL STABLE;

CREATE TRIGGER reactions_counter_cache_trigger
AFTER INSERT OR DELETE ON reactions
FOR EACH ROW
EXECUTE FUNCTION reactions_counter_cache();


