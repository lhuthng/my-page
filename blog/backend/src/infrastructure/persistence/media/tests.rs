// The media read queries are built with `sqlx::query_as` over string SQL, so a
// column added to the SELECT and not to the tuple (or vice versa) is a *runtime*
// failure that `cargo check` cannot see. These run the real queries against a
// migrated database instead.
use axum::body::Bytes;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};

use crate::application::commands::media::{
    ChangeAvatarCommand, ChangePostCoverCommand, GetLinkCommand, GetMediaDetailsCommand,
    SearchMediaCommand,
};
use crate::domain::entities::media::{MediaType, MediumDetails};
use crate::infrastructure::web::server::MediaConfig;

use super::MediaServiceImpl;

/// A temp database with this crate's real migrations applied, so the schema
/// under test is the one that ships rather than one restated here.
async fn test_pool(label: &str) -> sqlx::SqlitePool {
    let db = std::env::temp_dir().join(format!("{label}-{}.db", uuid::Uuid::new_v4()));
    let _ = std::fs::remove_file(&db);
    let opts = format!("sqlite://{}", db.display())
        .parse::<SqliteConnectOptions>()
        .unwrap()
        .create_if_missing(true)
        .foreign_keys(false)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await
        .unwrap();
    sqlx::migrate::Migrator::new(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("migrations"),
    )
    .await
    .unwrap()
    .run(&pool)
    .await
    .unwrap();
    pool
}

/// Insert one media row with an explicit upload time, so the assertion is about
/// the value travelling through the query rather than about "now".
async fn seed_media(pool: &sqlx::SqlitePool, short_name: &str, created_at: &str) {
    sqlx::query(
        r#"
        INSERT INTO media (hash, short_name, file_name, file_type, url, size, description, uploader_id, created_at)
        VALUES (?, ?, ?, 'image/webp', './media/webp/1/x.webp', 10, '', 1, ?)
        "#,
    )
    .bind(format!("hash-{short_name}"))
    .bind(short_name)
    .bind(format!("{short_name}.webp"))
    .bind(created_at)
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn search_reports_each_result_upload_time() {
    let pool = test_pool("media-search-created-at").await;
    seed_media(&pool, "banner.one", "2026-09-29 21:04:33").await;
    seed_media(&pool, "banner.two", "2026-09-30 08:15:00").await;

    let service = MediaServiceImpl::new(pool.clone());
    let results = service
        .search(SearchMediaCommand {
            term: Some("banner".to_string()),
            size: 10,
            skip: 0,
        })
        .await
        .unwrap();

    assert_eq!(results.len(), 2, "both seeded rows match the term");

    // The SQL orders by `created_at DESC`, so the newest upload comes first —
    // which is what makes an empty search read as "my most recent uploads".
    assert_eq!(results[0].short_name.as_deref(), Some("banner.two"));
    assert_eq!(
        results[0].created_at.as_deref(),
        Some("2026-09-30 08:15:00")
    );
    assert_eq!(
        results[1].created_at.as_deref(),
        Some("2026-09-29 21:04:33")
    );
}

#[tokio::test]
async fn an_empty_term_lists_the_most_recent_uploads() {
    let pool = test_pool("media-search-empty-term").await;
    seed_media(&pool, "older", "2026-09-01 00:00:00").await;
    seed_media(&pool, "newer", "2026-09-30 00:00:00").await;

    let service = MediaServiceImpl::new(pool.clone());
    let results = service
        .search(SearchMediaCommand {
            // An empty string is what the library sends when the box is empty.
            term: Some(String::new()),
            size: 10,
            skip: 0,
        })
        .await
        .unwrap();

    assert_eq!(
        results.len(),
        2,
        "an empty term must match everything, not nothing"
    );
    assert_eq!(
        results[0].short_name.as_deref(),
        Some("newer"),
        "the library opens on the most recent upload"
    );
}

#[tokio::test]
async fn details_and_link_both_carry_the_upload_time() {
    let pool = test_pool("media-details-created-at").await;
    seed_media(&pool, "album.art", "2026-09-29 21:04:33").await;

    let service = MediaServiceImpl::new(pool.clone());

    let details = service
        .get_details(GetMediaDetailsCommand {
            short_name: "album.art".to_string(),
        })
        .await
        .unwrap();
    assert_eq!(details.created_at.as_deref(), Some("2026-09-29 21:04:33"));

    // `get_link` feeds the byte-serving path, where the field goes unused — but
    // it is selected on the same row shape, so it is asserted here too: a
    // mismatch there would be a runtime error on every media request.
    let link = service
        .get_link(GetLinkCommand {
            short_name: "album.art".to_string(),
        })
        .await
        .unwrap();
    assert_eq!(link.created_at.as_deref(), Some("2026-09-29 21:04:33"));
}

#[tokio::test]
async fn a_row_with_no_upload_time_reads_as_none_rather_than_failing() {
    let pool = test_pool("media-null-created-at").await;
    // The column is `TEXT DEFAULT CURRENT_TIMESTAMP`, not `NOT NULL`, so a row
    // can genuinely have no value and the read path must tolerate it.
    sqlx::query(
        r#"
        INSERT INTO media (hash, short_name, file_name, file_type, url, size, description, uploader_id, created_at)
        VALUES ('h', 'undated', 'undated.png', 'image/png', './media/png/1/u.png', 1, '', 1, NULL)
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    let service = MediaServiceImpl::new(pool.clone());
    let details = service
        .get_details(GetMediaDetailsCommand {
            short_name: "undated".to_string(),
        })
        .await
        .unwrap();

    assert_eq!(details.created_at, None);
}

#[tokio::test]
async fn releasing_an_unreferenced_row_deletes_it() {
    let pool = test_pool("media-release-unreferenced").await;
    seed_media(&pool, "unused", "2026-10-01 00:00:00").await;
    let id: i64 = sqlx::query_scalar("SELECT id FROM media WHERE short_name = 'unused'")
        .fetch_one(&pool)
        .await
        .unwrap();

    let mut tx = pool.begin().await.unwrap();
    let outcome = super::release_media(&mut tx, id).await.unwrap();
    tx.commit().await.unwrap();

    assert!(matches!(outcome, super::ReleaseOutcome::Deleted));
    let remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM media WHERE id = ?")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(remaining, 0);
}

#[tokio::test]
async fn releasing_a_referenced_row_archives_it_instead_of_deleting_it() {
    let pool = test_pool("media-release-referenced").await;
    seed_media(&pool, "cover", "2026-10-01 00:00:00").await;
    let id: i64 = sqlx::query_scalar("SELECT id FROM media WHERE short_name = 'cover'")
        .fetch_one(&pool)
        .await
        .unwrap();

    // An inline usage is what a cover replacement used to cascade away.
    sqlx::query(
        r#"
        INSERT INTO posts
        (id, user_id, title, slug, content, excerpt, status, content_kind)
        VALUES (9, 1, 'Post', 'post-with-inline-cover', 'body', 'excerpt', 'published', 'post')
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO post_media_usages (post_id, medium_id, code) VALUES (9, ?, 0)")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();

    let mut tx = pool.begin().await.unwrap();
    let outcome = super::release_media(&mut tx, id).await.unwrap();
    tx.commit().await.unwrap();

    assert!(matches!(outcome, super::ReleaseOutcome::Archived));

    let (short_name, url): (String, String) =
        sqlx::query_as("SELECT short_name, url FROM media WHERE id = ?")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(short_name, format!("cover.replaced.{id}"));
    assert_eq!(
        url, "./media/webp/1/x.webp",
        "the archived row keeps the url the post renders through"
    );

    let usage: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM post_media_usages WHERE medium_id = ?")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        usage, 1,
        "the inline reference must survive the replacement"
    );
}

fn cover_config(dir: std::path::PathBuf) -> MediaConfig {
    MediaConfig {
        dir,
        allowed_file_types: vec![MediaType::ImageWebp],
        allowed_avatar_types: vec![],
        allowed_cover_types: vec![MediaType::ImagePng, MediaType::ImageWebp],
        allowed_audio_types: vec![],
        audio_bucket: None,
    }
}

fn png_bytes() -> Vec<u8> {
    let img = image::RgbaImage::from_pixel(1, 1, image::Rgba([200, 20, 20, 255]));
    let mut bytes = Vec::new();
    img.write_to(
        &mut std::io::Cursor::new(&mut bytes),
        image::ImageFormat::Png,
    )
    .unwrap();
    bytes
}

/// Seeds post 7 with `.post.7` as its cover, and returns the path the old cover
/// file is expected to live at.
async fn seed_post_with_old_cover(
    pool: &sqlx::SqlitePool,
    media_dir: &std::path::Path,
    media_id: i64,
    inline_usage: bool,
) -> std::path::PathBuf {
    sqlx::query(
        r#"
        INSERT INTO media (id, hash, short_name, file_name, file_type, url, size)
        VALUES (?, 'hash-old', '.post.7', 'old.webp', 'image/webp', './media/webp/1/x.webp', 1)
        "#,
    )
    .bind(media_id)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        r#"
        INSERT INTO posts
        (id, user_id, title, slug, content, excerpt, status, content_kind, cover_media_id)
        VALUES (7, 1, 'Cover post', 'cover-post', 'body', 'excerpt', 'published', 'post', ?)
        "#,
    )
    .bind(media_id)
    .execute(pool)
    .await
    .unwrap();
    if inline_usage {
        sqlx::query("INSERT INTO post_media_usages (post_id, medium_id, code) VALUES (7, ?, 0)")
            .bind(media_id)
            .execute(pool)
            .await
            .unwrap();
    }

    let old_path = media_dir.join("post").join("1").join("hash-old.webp");
    tokio::fs::create_dir_all(old_path.parent().unwrap())
        .await
        .unwrap();
    tokio::fs::write(&old_path, b"old cover bytes")
        .await
        .unwrap();
    old_path
}

fn replace_cover_cmd() -> ChangePostCoverCommand {
    ChangePostCoverCommand {
        user_id: 1,
        post_id: 7,
        medium_details: MediumDetails {
            filename: "new-cover.png".to_string(),
            content_type: "image/png".to_string(),
            bytes: Bytes::from(png_bytes()),
        },
        og_image_seconds: None,
    }
}

#[tokio::test]
async fn replacing_a_cover_used_inline_archives_the_old_row_and_keeps_its_file() {
    let pool = test_pool("media-cover-inline").await;
    let media_dir = std::env::temp_dir().join(format!("media-cover-{}", uuid::Uuid::new_v4()));
    let old_path = seed_post_with_old_cover(&pool, &media_dir, 201, true).await;

    let service = MediaServiceImpl::new(pool.clone());
    service
        .change_post_cover(replace_cover_cmd(), &cover_config(media_dir.clone()))
        .await
        .expect("replacing a cover with an inline reference must succeed");

    let (short_name, url): (String, String) =
        sqlx::query_as("SELECT short_name, url FROM media WHERE id = 201")
            .fetch_one(&pool)
            .await
            .expect("the inline-referenced old cover must not be deleted");
    assert_eq!(short_name, ".post.7.replaced.201");
    assert_eq!(
        url, "./media/webp/1/x.webp",
        "the archived row keeps the url the inline usage renders through"
    );
    assert!(
        tokio::fs::try_exists(&old_path).await.unwrap(),
        "an archived row keeps its file"
    );

    let new_cover_id: i64 = sqlx::query_scalar("SELECT cover_media_id FROM posts WHERE id = 7")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_ne!(new_cover_id, 201);
    let new_short_name: String = sqlx::query_scalar("SELECT short_name FROM media WHERE id = ?")
        .bind(new_cover_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        new_short_name, ".post.7",
        "the new cover takes the slot name"
    );

    let usage: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM post_media_usages WHERE medium_id = 201")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(usage, 1);
}

#[tokio::test]
async fn replacing_an_unreferenced_cover_still_deletes_the_old_row_and_file() {
    let pool = test_pool("media-cover-unreferenced").await;
    let media_dir = std::env::temp_dir().join(format!("media-cover-{}", uuid::Uuid::new_v4()));
    let old_path = seed_post_with_old_cover(&pool, &media_dir, 202, false).await;

    let service = MediaServiceImpl::new(pool.clone());
    service
        .change_post_cover(replace_cover_cmd(), &cover_config(media_dir.clone()))
        .await
        .expect("replacing an unreferenced cover must succeed");

    let old_row: Option<(i64,)> = sqlx::query_as("SELECT id FROM media WHERE id = 202")
        .fetch_optional(&pool)
        .await
        .unwrap();
    assert!(old_row.is_none(), "nothing pointed at it, so it is deleted");
    assert!(
        !tokio::fs::try_exists(&old_path).await.unwrap(),
        "a deleted row's file is removed too"
    );

    let new_cover_id: i64 = sqlx::query_scalar("SELECT cover_media_id FROM posts WHERE id = 7")
        .fetch_one(&pool)
        .await
        .unwrap();
    let new_short_name: String = sqlx::query_scalar("SELECT short_name FROM media WHERE id = ?")
        .bind(new_cover_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(new_short_name, ".post.7");
}

fn avatar_config(dir: std::path::PathBuf) -> MediaConfig {
    MediaConfig {
        dir,
        allowed_file_types: vec![MediaType::ImageWebp],
        allowed_avatar_types: vec![MediaType::ImagePng, MediaType::ImageWebp],
        allowed_cover_types: vec![MediaType::ImagePng, MediaType::ImageWebp],
        allowed_audio_types: vec![],
        audio_bucket: None,
    }
}

/// Seeds user 1 with `.avt.1` as their avatar, and returns the path the old
/// avatar file is expected to live at.
async fn seed_user_with_old_avatar(
    pool: &sqlx::SqlitePool,
    media_dir: &std::path::Path,
    media_id: i64,
    inline_usage: bool,
) -> std::path::PathBuf {
    sqlx::query("INSERT INTO users (id, username, email, password_hash) VALUES (1, 'u', 'u@example.com', 'h')")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO user_meta (user_id, display_name, avatar_image_id) VALUES (1, 'U', ?)",
    )
    .bind(media_id)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        r#"
        INSERT INTO media (id, hash, short_name, file_name, file_type, url, size)
        VALUES (?, '.avt.1.hashold', '.avt.1', 'old.webp', 'image/webp', './media/webp/1/x.webp', 1)
        "#,
    )
    .bind(media_id)
    .execute(pool)
    .await
    .unwrap();
    if inline_usage {
        sqlx::query(
            r#"
            INSERT INTO posts
            (id, user_id, title, slug, content, excerpt, status, content_kind)
            VALUES (7, 1, 'Body', 'avatar-inline', 'body', 'excerpt', 'published', 'post')
            "#,
        )
        .execute(pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO post_media_usages (post_id, medium_id, code) VALUES (7, ?, 0)")
            .bind(media_id)
            .execute(pool)
            .await
            .unwrap();
    }

    let old_path = media_dir.join("avt").join("1").join("hashold.webp");
    tokio::fs::create_dir_all(old_path.parent().unwrap())
        .await
        .unwrap();
    tokio::fs::write(&old_path, b"old avatar bytes")
        .await
        .unwrap();
    old_path
}

fn change_avatar_cmd() -> ChangeAvatarCommand {
    ChangeAvatarCommand {
        user_id: 1,
        medium_details: MediumDetails {
            filename: "new-avatar.png".to_string(),
            content_type: "image/png".to_string(),
            bytes: Bytes::from(png_bytes()),
        },
    }
}

#[tokio::test]
async fn replacing_an_avatar_used_inline_archives_the_old_row_and_keeps_its_file() {
    let pool = test_pool("media-avatar-inline").await;
    let media_dir = std::env::temp_dir().join(format!("media-avatar-{}", uuid::Uuid::new_v4()));
    let old_path = seed_user_with_old_avatar(&pool, &media_dir, 301, true).await;

    let service = MediaServiceImpl::new(pool.clone());
    service
        .change_avatar(change_avatar_cmd(), &avatar_config(media_dir.clone()))
        .await
        .expect("replacing an avatar with an inline reference must succeed");

    let short_name: String = sqlx::query_scalar("SELECT short_name FROM media WHERE id = 301")
        .fetch_one(&pool)
        .await
        .expect("the inline-referenced old avatar must not be deleted");
    assert_eq!(short_name, ".avt.1.replaced.301");
    assert!(
        tokio::fs::try_exists(&old_path).await.unwrap(),
        "an archived row keeps its file"
    );

    let new_avatar_id: i64 =
        sqlx::query_scalar("SELECT avatar_image_id FROM user_meta WHERE user_id = 1")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_ne!(new_avatar_id, 301);
    let new_short_name: String = sqlx::query_scalar("SELECT short_name FROM media WHERE id = ?")
        .bind(new_avatar_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(new_short_name, ".avt.1");

    let usage: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM post_media_usages WHERE medium_id = 301")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(usage, 1);
}

#[tokio::test]
async fn replacing_an_unreferenced_avatar_still_deletes_the_old_row_and_file() {
    let pool = test_pool("media-avatar-unreferenced").await;
    let media_dir = std::env::temp_dir().join(format!("media-avatar-{}", uuid::Uuid::new_v4()));
    let old_path = seed_user_with_old_avatar(&pool, &media_dir, 302, false).await;

    let service = MediaServiceImpl::new(pool.clone());
    service
        .change_avatar(change_avatar_cmd(), &avatar_config(media_dir.clone()))
        .await
        .expect("replacing an unreferenced avatar must succeed");

    let old_row: Option<(i64,)> = sqlx::query_as("SELECT id FROM media WHERE id = 302")
        .fetch_optional(&pool)
        .await
        .unwrap();
    assert!(old_row.is_none(), "nothing pointed at it, so it is deleted");
    assert!(
        !tokio::fs::try_exists(&old_path).await.unwrap(),
        "a deleted row's file is removed too"
    );

    let new_avatar_id: i64 =
        sqlx::query_scalar("SELECT avatar_image_id FROM user_meta WHERE user_id = 1")
            .fetch_one(&pool)
            .await
            .unwrap();
    let new_short_name: String = sqlx::query_scalar("SELECT short_name FROM media WHERE id = ?")
        .bind(new_avatar_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(new_short_name, ".avt.1");
}
