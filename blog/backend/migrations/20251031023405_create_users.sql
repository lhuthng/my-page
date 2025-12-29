-- Users table
CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    email TEXT NOT NULL UNIQUE,
    role TEXT NOT NULL DEFAULT 'user' CHECK(role IN ('user', 'admin', 'moderator')),
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);

-- User meta table
CREATE TABLE IF NOT EXISTS user_meta (
    user_id INTEGER PRIMARY KEY,
    display_name TEXT NOT NULL,
    bio TEXT NOT NULL DEFAULT '',
    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
);
