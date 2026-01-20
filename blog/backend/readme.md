To create a migration
```bash
sqlx migrate add <name>
```

To run migrations
```bash
sqlx migrate run --database-url sqlite://data/blog.db
```

To install rclone
```bash
sudo apt update && sudo apt install rclone -y
```

To create rclone config
```bash
mkdir -p ~/.config/rclone/
nano ~/.config/rclone/rclone.conf
```

Cronjob
```bash
crontab -e
0 0 * * * cd /var/dev/my-page/blog/backend && ./backup.sh
```
