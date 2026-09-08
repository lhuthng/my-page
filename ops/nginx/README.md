# VM nginx sites, managed from the repo.
#
# What lives here vs on the VM:
#   blog.conf  - blog.huuthangle.site 301s to the apex (kept for Google)
#   root.conf  - the apex site itself
#   api.conf   - api.huuthangle.site, straight to the backend
#
# Deployment (.github/workflows/deploy.yml) scp's this directory to
# /etc/nginx/sites-available and reloads nginx when it changed. Certificates
# are NOT managed here - certbot on the VM owns /etc/letsencrypt. A new
# subdomain needs a one-time on-VM cert:
#   sudo certbot certonly --nginx -d api.huuthangle.site
