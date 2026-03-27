-- Add migration script here

CREATE TABLE source (
    created_at  TEXT NOT NULL,
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL
);