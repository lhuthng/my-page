# Blog Backend

Rust 2024 edition · Axum 0.8 · SQLite via sqlx 0.8 · Port 3000

## Architecture

```
src/
├── domain/          # Core types and traits — no external dependencies
├── application/     # Use cases, service logic
└── infrastructure/  # DB, HTTP handlers, auth, email, media
```

GraphQL (via `async-graphql`) is admin-only, gated behind auth middleware. REST handles everything else.

---

## Environment Variables

Copy `example.env` to `.env` and fill in values.

| Variable | Description | Default |
|---|---|---|
| `DATABASE_URL` | SQLite path or PostgreSQL URL | `sqlite:data/blog.db` |
| `JWT_SECRET` | Secret for signing JWTs | — |
| `ACCESS_JWT_EXP_HOURS` | Access token lifetime (hours) | `24` |
| `REFRESH_JWT_EXP_HOURS` | Refresh token lifetime (hours) | `360` |
| `MEDIA_PATH` | Directory for uploaded files | `./media` |
| `ALLOWED_ORIGIN` | Origin allowed to call `/mail/contact-form` | `https://portfolio.huuthangle.site` |
| `SMTP_HOST` | SMTP server hostname | — |
| `SMTP_PORT` | SMTP server port | — |
| `SMTP_USERNAME` | SMTP login username | — |
| `SMTP_PASSWORD` | SMTP login password | — |
| `SMTP_FROM` | Sender address | — |
| `SMTP_TO` | Recipient address for contact form | — |

Email (via `lettre`) is optional — the server starts fine without SMTP configured.

---

## API Routes

**Auth levels:** public · optional auth · protected (any logged-in user) · mod protected · admin protected

### Auth — `/auth`

| Method | Path | Description |
|---|---|---|
| `POST` | `/auth/login` | Issue access + refresh tokens |
| `POST` | `/auth/register` | Create a new account |
| `POST` | `/auth/refresh` | Exchange refresh token for new access token |

### Users — `/users`

| Method | Path | Auth | Description |
|---|---|---|---|
| `GET` | `/users/:username` | Public | Get user profile |
| `GET` | `/users/` | Public | Search users |
| `GET` | `/users/:username/posts` | Optional | Get posts by user |
| `GET` | `/users/me` | Protected | Get own profile |
| `PATCH` | `/users/me/details` | Protected | Update own details |
| `PATCH` | `/users/me/avatar` | Protected | Upload avatar (20 MB limit) |
| `GET` | `/users/me/check-mod` | Protected | Check if current user is a moderator |

### Posts — `/posts`

| Method | Path | Auth | Description |
|---|---|---|---|
| `GET` | `/posts/featured` | Public | Get featured posts |
| `GET` | `/posts/latest` | Public | Get latest posts |
| `GET` | `/posts/` | Public | Search posts |
| `GET` | `/posts/categories` | Public | List all categories |
| `GET` | `/posts/check` | Public | Check post availability |
| `GET` | `/posts/s/:slug` | Optional | Get post by slug |
| `GET` | `/posts/id/:id/comments` | Public | Get comments for a post |
| `POST` | `/posts/id/:id/view` | Public | Record a view |
| `POST` | `/posts/id/:id/like` | Public | Like a post |
| `PUT` | `/posts/id/:id/comments/new` | Optional | Add a comment (2 MB limit) |
| `POST` | `/posts/new` | Mod | Create a new post (draft) |
| `POST` | `/posts/id/:id` | Mod | Publish a post |
| `GET` | `/posts/id/:id` | Mod | Get post by ID (includes unpublished) |
| `PATCH` | `/posts/id/:id` | Mod | Edit post content/metadata |
| `PATCH` | `/posts/id/:id/cover` | Mod | Upload cover image (100 MB limit) |

### Series — `/series`

| Method | Path | Auth | Description |
|---|---|---|---|
| `GET` | `/series/public/all` | Public | List all published series |
| `GET` | `/series/all` | Mod | List all series including drafts |
| `POST` | `/series/new` | Mod | Create a new series |
| `PATCH` | `/series/id/:id` | Mod | Add a post to a series |
| `DELETE` | `/series/id/:id` | Mod | Remove a post from a series |
| `GET` | `/series/id/:id/posts` | Mod | List posts in a series |

### Media — `/media`

Static files are also served directly from `MEDIA_PATH` via `tower-http ServeDir` as a fallback.

| Method | Path | Auth | Description |
|---|---|---|---|
| `GET` | `/media/s/:name` | Public | Get a shareable link for a file |
| `GET` | `/media/i/:name` | Public | Serve a file by name |
| `GET` | `/media/all` | Public | Search media |
| `POST` | `/media/upload` | Mod | Upload a file (100 MB limit) |
| `GET` | `/media/d/:name` | Mod | Get file details |
| `PATCH` | `/media/d/:name` | Mod | Update file details |
| `GET` | `/media/d/:name/aliases` | Mod | List aliases for a file |
| `POST` | `/media/d/:name/aliases` | Mod | Add an alias |
| `PATCH` | `/media/d/:name/aliases/:alias` | Mod | Update an alias |
| `DELETE` | `/media/d/:name/aliases/:alias` | Mod | Delete an alias |

### Dashboard — `/dashboard` (mod protected)

| Method | Path | Description |
|---|---|---|
| `GET` | `/dashboard/overview` | Summary stats |
| `GET` | `/dashboard/posts` | Post management data |
| `GET` | `/dashboard/users` | User management data |

### Mail — `/mail`

| Method | Path | Auth | Description |
|---|---|---|---|
| `POST` | `/mail/contact-form` | CORS-restricted | Send contact form email — only accepts requests from `ALLOWED_ORIGIN` |

### GraphQL — `/graphql` (admin protected)

| Method | Path | Description |
|---|---|---|
| `GET` | `/graphql` | GraphQL Playground |
| `POST` | `/graphql` | Execute a query |

---

## Database Migrations

Migrations run automatically on startup. To manage them manually:

```bash
# Create a new migration
sqlx migrate add <migration_name>

# Run migrations manually
sqlx migrate run --database-url sqlite://data/blog.db
```

---

## Backup

`backup.sh` compresses `data/` (database) and `media/` (uploaded files) into dated tarballs, then pushes them to a configured rclone remote (`r2-backup:blog-backup/daily-backups`).

**Setup:**

```bash
sudo apt install rclone -y
mkdir -p ~/.config/rclone/
nano ~/.config/rclone/rclone.conf
```

Configure your remote in `rclone.conf` — any backend rclone supports works (R2, S3, etc.). The remote name in the script is `r2-backup` and the bucket is `blog-backup`.

**Optional: run daily at midnight via cron:**

```bash
crontab -e
# Add:
0 0 * * * cd ~/MyPage/blog/backend && ./backup.sh
```
