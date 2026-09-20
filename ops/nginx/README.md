# VM nginx Sites

Audience: whoever changes nginx config. Update trigger: a site or routing
change.

## What lives here vs on the VM

| File | Site | Purpose |
| --- | --- | --- |
| `blog.conf` | blog.huuthangle.site | 301s to the apex (kept for Google) |
| `root.conf` | huuthangle.site | The apex site itself |
| `api.conf` | api.huuthangle.site | Straight to the backend |

## Deployment

`.github/workflows/deploy.yml` scp's this directory to
`/etc/nginx/sites-available` and reloads nginx when it changed.

## Certificates

NOT managed here — certbot on the VM owns `/etc/letsencrypt`. A new subdomain
needs a one-time on-VM certificate:

```bash
sudo certbot certonly --nginx -d api.huuthangle.site
```

Routing rules (media vs frontend prefixes, `fs`-mode artifact locations) are
documented in [../../docs/architecture/deployment.md](../../docs/architecture/deployment.md).
