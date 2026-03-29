-- Add migration script here

CREATE TABLE functions (
    commit_id   INTEGER NOT NULL,
    created_at  TEXT NOT NULL,
    guid        TEXT NOT NULL,
    id          INTEGER PRIMARY KEY,
    source_id   TEXT NOT NULL,
    symbol_id   INTEGER NOT NULL,
    target_id   INTEGER NOT NULL,
    type_id     INTEGER
);