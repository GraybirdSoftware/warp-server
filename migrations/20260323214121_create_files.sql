-- Add migration script here

CREATE TABLE files (
    file        BLOB NOT NULL,
    name        TEXT NOT NULL,
    source      TEXT NOT NULL,
    commit_id   INTEGER NOT NULL
);