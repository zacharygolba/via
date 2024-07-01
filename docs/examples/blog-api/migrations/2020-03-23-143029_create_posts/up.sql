-- Your SQL goes here

CREATE TABLE posts (
  id SERIAL PRIMARY KEY,
  body TEXT NOT NULL,
  title VARCHAR NOT NULL,
  user_id INTEGER NOT NULL REFERENCES users(id),
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  published_at TIMESTAMP
);

SELECT diesel_manage_updated_at('posts');

INSERT INTO posts (body, title, user_id, published_at) VALUES (
  'Hello, world!',
  'Hello, world!',
  (SELECT id FROM users WHERE username = 'admin'),
  CURRENT_TIMESTAMP
);
