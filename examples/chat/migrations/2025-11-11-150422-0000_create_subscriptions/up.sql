CREATE TABLE subscriptions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id UUID NOT NULL REFERENCES users(id),
  thread_id UUID NOT NULL REFERENCES threads(id),

  claims INTEGER NOT NULL DEFAULT 1,
  created_at TIMESTAMP NOT NULL DEFAULT current_timestamp,
  updated_at TIMESTAMP NOT NULL DEFAULT current_timestamp
);

SELECT diesel_manage_updated_at('subscriptions');

CREATE UNIQUE INDEX subscriptions_by_join_idx ON subscriptions (user_id, thread_id);

CREATE FUNCTION has_flags(lhs integer, rhs integer)
RETURNS boolean AS $$
  SELECT (lhs & rhs) = rhs;
$$ LANGUAGE SQL IMMUTABLE;
