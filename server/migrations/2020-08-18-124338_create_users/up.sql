CREATE TABLE users (
  id BIGSERIAL PRIMARY KEY,
  auth_token TEXT NOT NULL UNIQUE,
  nick TEXT
);

CREATE TABLE games (
  id BIGSERIAL PRIMARY KEY,
  name TEXT NOT NULL,
  replay BYTEA
);
