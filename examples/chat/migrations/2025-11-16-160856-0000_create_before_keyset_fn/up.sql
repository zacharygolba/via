CREATE FUNCTION before_keyset(lhs0 TIMESTAMP, lhs1 UUID, rhs0 TIMESTAMP, rhs1 UUID)
RETURNS boolean AS $$
  SELECT (lhs0, lhs1) > (rhs0, rhs1)
$$ LANGUAGE SQL STABLE;
