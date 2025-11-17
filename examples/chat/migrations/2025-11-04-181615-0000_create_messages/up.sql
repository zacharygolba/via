CREATE TABLE messages (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  author_id UUID NOT NULL REFERENCES users(id),
  thread_id UUID NOT NULL REFERENCES threads(id) ON DELETE CASCADE,

  body TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT current_timestamp,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT current_timestamp,
  reactions_count BIGINT NOT NULL DEFAULT 0
);

SELECT diesel_manage_updated_at('messages');

-- Used to update or delete a user's message.
CREATE INDEX messages_by_id_and_author_idx
ON messages (id, author_id);

-- Used to list the messages in a thread.
CREATE INDEX messages_recent_by_thread_idx
ON messages (thread_id, created_at DESC, id DESC);
