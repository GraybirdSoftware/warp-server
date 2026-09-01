-- Extend the schema to back the full WARP server API.
--
-- NOTE: `functions` and `files` are rebuilt here. No released build of this
-- server was ever able to write to them (every route returned 501), so there
-- is nothing to carry over.

-- users -------------------------------------------------------------------
UPDATE users
SET username = substr(email, 1, instr(email, '@') - 1)
WHERE username IS NULL OR username = '';

CREATE UNIQUE INDEX IF NOT EXISTS idx_users_email ON users(email);
CREATE UNIQUE INDEX IF NOT EXISTS idx_users_username ON users(username);

-- api_keys: `key` stores the SHA-256 hex digest of the key, never the key.
CREATE UNIQUE INDEX IF NOT EXISTS idx_api_keys_key ON api_keys(key);
CREATE INDEX IF NOT EXISTS idx_api_keys_user ON api_keys(user_id);

-- browser sessions (OAuth logins) and pending OAuth states -----------------
CREATE TABLE sessions (
    id          TEXT PRIMARY KEY,       -- sha256 hex of the session token
    user_id     INTEGER NOT NULL,
    created_at  TEXT NOT NULL,
    expires_at  TEXT NOT NULL
);
CREATE INDEX idx_sessions_user ON sessions(user_id);

CREATE TABLE oauth_states (
    state       TEXT PRIMARY KEY,
    next        TEXT,
    created_at  TEXT NOT NULL
);

-- sources -----------------------------------------------------------------
CREATE TABLE source_tags (
    source_id   TEXT NOT NULL,
    tag         TEXT NOT NULL,
    PRIMARY KEY (source_id, tag)
);
CREATE INDEX idx_source_users_user ON source_users(user_id);

-- targets -----------------------------------------------------------------
-- platform/arch use '' for "unspecified" so the UNIQUE constraint works.
CREATE TABLE targets (
    id          INTEGER PRIMARY KEY,
    platform    TEXT NOT NULL DEFAULT '',
    arch        TEXT NOT NULL DEFAULT '',
    created_at  TEXT NOT NULL,
    UNIQUE (platform, arch)
);

-- symbols -----------------------------------------------------------------
CREATE TABLE symbols (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL,
    class       TEXT NOT NULL,             -- Bare | Data | Function
    modifier    TEXT NOT NULL,             -- None | Extern | Exported
    created_at  TEXT NOT NULL,
    UNIQUE (name, class, modifier)
);

-- types -------------------------------------------------------------------
CREATE TABLE types (
    id          TEXT PRIMARY KEY,          -- WARP type GUID
    name        TEXT,
    source_id   TEXT NOT NULL,
    commit_id   INTEGER NOT NULL,
    data        BLOB NOT NULL,             -- WARP `Type` flatbuffer
    created_at  TEXT NOT NULL
);
CREATE INDEX idx_types_name ON types(name);
CREATE INDEX idx_types_source ON types(source_id);
CREATE INDEX idx_types_commit ON types(commit_id);

-- functions ---------------------------------------------------------------
DROP TABLE functions;
CREATE TABLE functions (
    id          INTEGER PRIMARY KEY,
    guid        TEXT NOT NULL,
    commit_id   INTEGER NOT NULL,
    source_id   TEXT NOT NULL,
    target_id   INTEGER NOT NULL,
    symbol_id   INTEGER NOT NULL,
    type_id     TEXT,                      -- WARP type GUID, if the function has a type
    data        BLOB NOT NULL,             -- WARP `Function` flatbuffer
    created_at  TEXT NOT NULL
);
CREATE INDEX idx_functions_guid ON functions(guid);
CREATE INDEX idx_functions_source ON functions(source_id);
CREATE INDEX idx_functions_commit ON functions(commit_id);
CREATE INDEX idx_functions_target ON functions(target_id);
CREATE INDEX idx_functions_symbol ON functions(symbol_id);

CREATE TABLE function_constraints (
    id          INTEGER PRIMARY KEY,
    function_id INTEGER NOT NULL,
    guid        TEXT NOT NULL,
    byte_offset INTEGER
);
CREATE INDEX idx_function_constraints_function ON function_constraints(function_id);
CREATE INDEX idx_function_constraints_guid ON function_constraints(guid);

CREATE TABLE function_comments (
    id          INTEGER PRIMARY KEY,
    function_id INTEGER NOT NULL,
    text        TEXT NOT NULL,
    byte_offset INTEGER NOT NULL
);
CREATE INDEX idx_function_comments_function ON function_comments(function_id);

-- files / commits ---------------------------------------------------------
DROP TABLE files;
CREATE TABLE files (
    id          INTEGER PRIMARY KEY,
    commit_id   INTEGER NOT NULL,
    source_id   TEXT NOT NULL,
    name        TEXT NOT NULL,
    file        BLOB NOT NULL,             -- the uploaded .warp file, verbatim
    created_at  TEXT NOT NULL
);
CREATE INDEX idx_files_commit ON files(commit_id);
CREATE INDEX idx_commits_source ON commits(source_id);
CREATE INDEX idx_commits_user ON commits(user_id);
