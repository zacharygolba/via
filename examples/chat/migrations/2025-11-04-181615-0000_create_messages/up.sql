CREATE TABLE messages (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

  body TEXT NOT NULL,

  author_id UUID NOT NULL REFERENCES users(id),
  thread_id UUID NOT NULL REFERENCES threads(id) ON DELETE CASCADE,

  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

SELECT diesel_manage_updated_at('messages');

CREATE INDEX messages_by_id_and_author_idx
ON messages (id, author_id);

CREATE INDEX messages_paginated_by_thread_idx
ON messages (thread_id, created_at DESC, id DESC);
