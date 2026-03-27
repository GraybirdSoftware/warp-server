-- Add migration script here

CREATE TABLE users (
    id          INTEGER PRIMARY KEY,
    username    TEXT,
    email       TEXT NOT NULL,
    role        TEXT NOT NULL,
    created_at  TEXT NOT NULL
);
