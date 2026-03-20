-- Add migration script here

CREATE TABLE users(
    id          INTEGER PRIMARY KEY,
    username    TEXT NOT NULL,
    email       TEXT NOT NULL,
    role        TEXT NOT NULL,
    created_at  TEXT NOT NULL
);
