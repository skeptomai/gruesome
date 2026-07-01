-- Gruesome self-hosted platform schema (SQLite).
-- Ports the AWS DynamoDB single-table model (USER#/GAME#/SAVE# items) to
-- relational tables. Timestamps are unix seconds (INTEGER) throughout — this
-- avoids the string-vs-numeric `created_at` inconsistency the DynamoDB version
-- had between the admin writer (RFC3339) and the game reader (numeric).

CREATE TABLE IF NOT EXISTS users (
    user_id       TEXT PRIMARY KEY,          -- uuid v4
    email         TEXT NOT NULL UNIQUE COLLATE NOCASE,
    username      TEXT NOT NULL UNIQUE COLLATE NOCASE,
    display_name  TEXT NOT NULL,
    password_hash TEXT NOT NULL,             -- argon2id PHC string (Cognito held this before)
    role          TEXT,                      -- NULL for normal users; 'admin' grants admin
    created_at    INTEGER NOT NULL           -- unix seconds
);

CREATE TABLE IF NOT EXISTS games (
    game_id       TEXT PRIMARY KEY,          -- e.g. "zork1"
    title         TEXT NOT NULL,
    author        TEXT NOT NULL,
    description   TEXT NOT NULL DEFAULT '',
    category      TEXT,
    year          INTEGER,
    version       INTEGER NOT NULL,          -- Z-Machine version (3/4/5/8)
    release       INTEGER NOT NULL DEFAULT 0,
    serial        TEXT NOT NULL DEFAULT '',
    checksum      TEXT NOT NULL DEFAULT '',
    file_size     INTEGER NOT NULL DEFAULT 0,
    s3_key        TEXT NOT NULL,             -- object key in the games bucket
    display_order INTEGER,
    archived      INTEGER NOT NULL DEFAULT 0, -- soft-delete (boolean 0/1)
    created_at    INTEGER NOT NULL,
    updated_at    INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_games_active
    ON games (archived, display_order, created_at);

CREATE TABLE IF NOT EXISTS saves (
    user_id      TEXT NOT NULL,
    game_id      TEXT NOT NULL,
    save_name    TEXT NOT NULL,
    s3_key       TEXT NOT NULL,             -- object key in the saves bucket
    file_size    INTEGER NOT NULL DEFAULT 0,
    created_at   INTEGER NOT NULL,
    last_updated INTEGER NOT NULL,
    PRIMARY KEY (user_id, game_id, save_name),
    FOREIGN KEY (user_id) REFERENCES users (user_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_saves_user_game ON saves (user_id, game_id);

-- Password reset codes (replaces Cognito's ForgotPassword flow). The code is
-- short and single-use; for a home instance with no SMTP it is written to the
-- server log for the admin/user to relay. Email delivery is a later enhancement.
CREATE TABLE IF NOT EXISTS password_resets (
    user_id    TEXT NOT NULL,
    code_hash  TEXT NOT NULL,             -- argon2 hash of the reset code
    expires_at INTEGER NOT NULL,          -- unix seconds
    PRIMARY KEY (user_id),
    FOREIGN KEY (user_id) REFERENCES users (user_id) ON DELETE CASCADE
);
