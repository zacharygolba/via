CREATE TABLE subscriptions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id UUID NOT NULL REFERENCES users(id),
  thread_id UUID NOT NULL REFERENCES threads(id),

  claims INTEGER NOT NULL DEFAULT 1,
  created_at TIMESTAMPTZ NOT NULL DEFAULT current_timestamp,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT current_timestamp
);

-- Automated timestamp updates managed by diesel.
SELECT diesel_manage_updated_at('subscriptions');

-- Asserts that a user cannot have more than one subscription to a thread.
CREATE UNIQUE INDEX subscriptions_by_join_idx
ON subscriptions (user_id, thread_id);

-- Used to list a user's threads.
CREATE INDEX subscriptions_recent_by_user_idx
ON subscriptions (user_id, thread_id, created_at DESC, id DESC);

-- Used to list the users in a thread.
CREATE INDEX subscriptions_recent_by_thread_idx
ON subscriptions(thread_id, created_at DESC, id DESC);

-- Used to filter subscriptions that have the base level of permissions
-- required to participate in a chat thread.
--
-- VIEW | WRITE | REACT
--
CREATE INDEX subscriptions_claims_can_participate_idx
ON subscriptions((claims & 7));

-- Build a BitAnd Eq expression from diesel's query DSL.
-- Used to assert auth claims that a user has for a thread.
CREATE FUNCTION has_flags(lhs integer, rhs integer)
RETURNS boolean AS $$
  SELECT (lhs & rhs) = rhs;
$$ LANGUAGE SQL IMMUTABLE;
