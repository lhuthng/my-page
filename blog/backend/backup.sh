#!/bin/bash

BACKUP_DIR="/tmp/backups"

TIMESTAMP=$(date +%Y-%m-%d_%H-%M)
R2_REMOTE="r2-backup"
BUCKET_NAME="blog-backup"

mkdir -p $BACKUP_DIR

tar -czf $BACKUP_DIR/data_$TIMESTAMP.tar.gz "./data"
tar -czf $BACKUP_DIR/media_$TIMESTAMP.tar.gz "./media"

rclone copy $BACKUP_DIR $R2_REMOTE:$BUCKET_NAME/daily-backups

rm -rf $BACKUP_DIR
