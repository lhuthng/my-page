ALTER TABLE posts
ADD COLUMN content_kind TEXT NOT NULL DEFAULT 'post';

CREATE INDEX IF NOT EXISTS idx_posts_content_kind_status
ON posts(content_kind, status);

CREATE TABLE IF NOT EXISTS projects (
    id INTEGER PRIMARY KEY,
    post_id INTEGER NOT NULL UNIQUE,

    demo_type TEXT NOT NULL DEFAULT 'html5'
        CHECK (demo_type IN ('html5', 'embed', 'webgl', 'download', 'video')),
    demo_entry_path TEXT NOT NULL DEFAULT 'index.html',
    demo_width TEXT,
    demo_height TEXT,
    demo_config TEXT,

    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (post_id) REFERENCES posts(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS project_links (
    id INTEGER PRIMARY KEY,
    project_id INTEGER NOT NULL,
    label TEXT NOT NULL,
    url TEXT NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,

    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_projects_post_id ON projects(post_id);
CREATE INDEX IF NOT EXISTS idx_project_links_project_order
ON project_links(project_id, sort_order);
