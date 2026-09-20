# Troubleshooting

Audience: whoever diagnoses the blog in production. Update trigger: a new
known failure mode.

| Symptom | Likely cause | Action |
| --- | --- | --- |
| Backend container unhealthy right after a deploy | A migration failed at startup | Read the backend logs printed by the deploy workflow; fix the migration, redeploy. See [../architecture/data-model.md](../architecture/data-model.md). |
| Media images 404 while pages render | nginx not routing `/media/*` to `127.0.0.1:3001` | Check the nginx site config (`ops/nginx/`, VM copy). |
| v86 chunks 404 in `fs` mode | `/v86/`, `/games/s/`, `/project-demos/` not routed to the backend | Add the three location blocks; see [../reference/media-and-storage.md](../reference/media-and-storage.md). |
| Stale 404 for a new static file after deploy | Cloudflare edge cache | Purge that URL in Cloudflare (see the smoke-test note in `scripts/emergency/README.md`). |
| Contact form fails silently | Mail transport not configured, or origin not in `ALLOWED_ORIGIN(S)` | See [configuration.md](configuration.md); the route is CORS-restricted. |
| sync-pull fails with `no sync endpoints found` | Wrong URL shape, or the key was revoked | Use the public site URL or a direct backend URL; reissue the key from the dashboard. |
| Emails not delivered | No SMTP/Brevo variables set | Mail is optional; configure a transport in [configuration.md](configuration.md). |
