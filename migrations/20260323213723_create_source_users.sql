-- Add migration script here

CREATE TABLE source_users (
    source_id   TEXT NOT NULL,
    user_id     INTEGER NOT NULL,
    PRIMARY KEY (source_id, user_id)
);
