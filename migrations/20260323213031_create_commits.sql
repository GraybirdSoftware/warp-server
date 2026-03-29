-- Add migration script here

CREATE TABLE commits(
    created_at  TEXT NOT NULL,
    description TEXT,
    id          INTEGER PRIMARY KEY,
    name        TEXT,
    source_id   TEXT NOT NULL,
    user_id     INTEGER
);
