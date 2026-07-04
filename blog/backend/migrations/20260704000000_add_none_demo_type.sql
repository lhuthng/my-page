PRAGMA foreign_keys=off;

CREATE TABLE projects_new (
    id INTEGER PRIMARY KEY,
    post_id INTEGER NOT NULL UNIQUE,

    demo_type TEXT NOT NULL DEFAULT 'html5'
        CHECK (demo_type IN ('none', 'html5', 'embed', 'webgl', 'download', 'video')),
    demo_entry_path TEXT NOT NULL DEFAULT 'index.html',
    demo_width TEXT,
    demo_height TEXT,
    demo_config TEXT,
    demo_url TEXT,

    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (post_id) REFERENCES posts(id) ON DELETE CASCADE
);

INSERT INTO projects_new (
    id,
    post_id,
    demo_type,
    demo_entry_path,
    demo_width,
    demo_height,
    demo_config,
    demo_url,
    created_at,
    updated_at
)
SELECT
    id,
    post_id,
    demo_type,
    demo_entry_path,
    demo_width,
    demo_height,
    demo_config,
    demo_url,
    created_at,
    updated_at
FROM projects;

DROP TABLE projects;
ALTER TABLE projects_new RENAME TO projects;

CREATE INDEX IF NOT EXISTS idx_projects_post_id ON projects(post_id);

PRAGMA foreign_keys=on;
