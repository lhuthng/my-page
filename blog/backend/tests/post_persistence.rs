//! Integration tests for the post save path against a real migrated SQLite DB.
//!
//! These cover the two regressions fixed alongside the editor hardening work:
//! the tag/media placeholder-bind mismatch, and the missing ownership predicate
//! on `update_post`.

use std::collections::HashMap;

use backend::application::commands::post::{
    GetDetailedPostsCommand, GetPostCommand, NewPostCommand, UpdatePostCommand,
};
use backend::application::services::post::PostService;
use backend::domain::errors::post::PostError;
use backend::infrastructure::persistence::content_links;
use backend::infrastructure::persistence::post::PostServiceImpl;
use backend::infrastructure::persistence::redirects;
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

fn mig_src() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("migrations")
}

/// A migrated, in-memory database with two users seeded (ids 1 and 2).
async fn setup() -> SqlitePool {
    let opts = "sqlite::memory:"
        .parse::<SqliteConnectOptions>()
        .unwrap()
        .create_if_missing(true)
        .foreign_keys(false);
    // max_connections(1) keeps every query on the same in-memory database.
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await
        .unwrap();

    sqlx::migrate::Migrator::new(mig_src())
        .await
        .unwrap()
        .run(&pool)
        .await
        .unwrap();

    for (id, name) in [(1, "author"), (2, "other")] {
        sqlx::query("INSERT INTO users (id,username,email,password_hash) VALUES (?,?,?,'h')")
            .bind(id)
            .bind(name)
            .bind(format!("{name}@example.com"))
            .execute(&pool)
            .await
            .unwrap();
    }

    pool
}

fn new_post_cmd(user_id: i64, slug: &str, tags: Vec<&str>) -> NewPostCommand {
    NewPostCommand {
        user_id,
        title: "A title".to_string(),
        slug: slug.to_string(),
        excerpt: "An excerpt".to_string(),
        content: "Body text".to_string(),
        tags: tags.into_iter().map(str::to_string).collect(),
        cover_media: None,
        media_usage: HashMap::new(),
        content_kind: "post".to_string(),
    }
}

fn update_cmd(user_id: i64, post_id: i64) -> UpdatePostCommand {
    UpdatePostCommand {
        user_id,
        required_author_id: Some(user_id),
        expected_updated_at: None,
        post_id,
        title: None,
        slug: None,
        excerpt: None,
        content: None,
        tags: None,
        media_usage: None,
    }
}

async fn tags_of(pool: &SqlitePool, post_id: i64) -> Vec<String> {
    let mut rows: Vec<String> = sqlx::query_scalar(
        "SELECT t.slug FROM tags t JOIN post_tags pt ON pt.tag_id = t.id WHERE pt.post_id = ?",
    )
    .bind(post_id)
    .fetch_all(pool)
    .await
    .unwrap();
    rows.sort();
    rows
}

#[tokio::test]
async fn new_post_creates_tags_that_do_not_exist_yet() {
    let pool = setup().await;
    let svc = PostServiceImpl { pool: pool.clone() };

    // Every tag here is novel. Before the fix this produced a placeholder/bind
    // mismatch because placeholders came from the input while binds came from
    // the (empty) set of already-existing tags.
    let post_id = svc
        .new_post(new_post_cmd(1, "brand-new-tags", vec!["rust", "sqlite"]))
        .await
        .expect("new_post should create missing tags");

    assert_eq!(tags_of(&pool, post_id).await, vec!["rust", "sqlite"]);
}

#[tokio::test]
async fn new_post_accepts_a_mix_of_existing_and_novel_tags() {
    let pool = setup().await;
    let svc = PostServiceImpl { pool: pool.clone() };

    sqlx::query("INSERT INTO tags (slug, name) VALUES ('rust','rust')")
        .execute(&pool)
        .await
        .unwrap();

    let post_id = svc
        .new_post(new_post_cmd(1, "mixed-tags", vec!["rust", "axum"]))
        .await
        .expect("new_post should handle a partially-existing tag set");

    assert_eq!(tags_of(&pool, post_id).await, vec!["axum", "rust"]);
}

#[tokio::test]
async fn duplicate_tags_are_deduplicated_rather_than_erroring() {
    let pool = setup().await;
    let svc = PostServiceImpl { pool: pool.clone() };

    let post_id = svc
        .new_post(new_post_cmd(1, "dupe-tags", vec!["rust", "rust", "Rust"]))
        .await
        .expect("duplicate tags should collapse, not desynchronise the binds");

    assert_eq!(tags_of(&pool, post_id).await, vec!["rust"]);
}

#[tokio::test]
async fn update_post_replaces_tags_including_novel_and_duplicate_ones() {
    let pool = setup().await;
    let svc = PostServiceImpl { pool: pool.clone() };

    let post_id = svc
        .new_post(new_post_cmd(1, "retag-me", vec!["rust"]))
        .await
        .unwrap();

    let mut cmd = update_cmd(1, post_id);
    cmd.tags = Some(vec!["axum".into(), "axum".into(), "sqlite".into()]);
    svc.update_post(cmd)
        .await
        .expect("update_post should retag");

    assert_eq!(tags_of(&pool, post_id).await, vec!["axum", "sqlite"]);
}

#[tokio::test]
async fn invalid_tag_slug_is_a_validation_error_not_a_db_error() {
    let pool = setup().await;
    let svc = PostServiceImpl { pool: pool.clone() };

    let err = svc
        .new_post(new_post_cmd(1, "bad-tag", vec!["not a slug!"]))
        .await
        .expect_err("an invalid tag slug should be rejected");

    assert!(
        matches!(err, PostError::Validation(_)),
        "expected a validation error, got {err:?}"
    );
}

#[tokio::test]
async fn update_post_rejects_a_non_owner() {
    let pool = setup().await;
    let svc = PostServiceImpl { pool: pool.clone() };

    let post_id = svc
        .new_post(new_post_cmd(1, "owned-by-one", vec![]))
        .await
        .unwrap();

    // User 2 is a moderator as far as the router is concerned, but does not own
    // this post.
    let mut cmd = update_cmd(2, post_id);
    cmd.title = Some("Hijacked".to_string());
    let err = svc
        .update_post(cmd)
        .await
        .expect_err("a non-owner must not be able to overwrite the post");
    assert!(
        matches!(err, PostError::PostNotFound | PostError::Forbidden),
        "expected a not-found/forbidden error, got {err:?}"
    );

    let title: String = sqlx::query_scalar("SELECT title FROM posts WHERE id = ?")
        .bind(post_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(title, "A title", "the post body must be unchanged");
}

#[tokio::test]
async fn update_post_rejects_a_non_owner_even_for_a_tags_only_patch() {
    let pool = setup().await;
    let svc = PostServiceImpl { pool: pool.clone() };

    let post_id = svc
        .new_post(new_post_cmd(1, "tags-only-hijack", vec!["rust"]))
        .await
        .unwrap();

    // A patch with no scalar fields never reaches the UPDATE statement, so an
    // `AND user_id = ?` predicate on that statement alone would not catch it.
    let mut cmd = update_cmd(2, post_id);
    cmd.tags = Some(vec!["hijacked".into()]);
    let err = svc
        .update_post(cmd)
        .await
        .expect_err("a tags-only patch from a non-owner must be rejected");
    assert!(
        matches!(err, PostError::PostNotFound | PostError::Forbidden),
        "expected a not-found/forbidden error, got {err:?}"
    );

    assert_eq!(tags_of(&pool, post_id).await, vec!["rust"]);
}

#[tokio::test]
async fn update_post_allows_an_admin_on_someone_elses_post() {
    let pool = setup().await;
    let svc = PostServiceImpl { pool: pool.clone() };

    let post_id = svc
        .new_post(new_post_cmd(1, "admin-editable", vec![]))
        .await
        .unwrap();

    let mut cmd = update_cmd(2, post_id);
    cmd.required_author_id = None; // what the handler passes for an admin
    cmd.title = Some("Edited by admin".to_string());
    svc.update_post(cmd).await.expect("an admin may edit");

    let title: String = sqlx::query_scalar("SELECT title FROM posts WHERE id = ?")
        .bind(post_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(title, "Edited by admin");
}

#[tokio::test]
async fn update_post_bumps_updated_at_and_returns_it() {
    let pool = setup().await;
    let svc = PostServiceImpl { pool: pool.clone() };

    let post_id = svc
        .new_post(new_post_cmd(1, "stamped", vec![]))
        .await
        .unwrap();

    let mut cmd = update_cmd(1, post_id);
    cmd.title = Some("First edit".to_string());
    let returned = svc.update_post(cmd).await.unwrap();

    let stored: String = sqlx::query_scalar("SELECT updated_at FROM posts WHERE id = ?")
        .bind(post_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(!returned.is_empty());
    assert!(
        returned.starts_with(&stored[..10]),
        "returned {returned} should correspond to stored {stored}"
    );
}

#[tokio::test]
async fn update_post_bumps_updated_at_for_a_tags_only_patch() {
    let pool = setup().await;
    let svc = PostServiceImpl { pool: pool.clone() };

    let post_id = svc
        .new_post(new_post_cmd(1, "tags-only-stamp", vec!["rust"]))
        .await
        .unwrap();

    // updated_at is the lock token, so even a patch that touches no scalar
    // column has to move it or a concurrent writer could not detect the change.
    sqlx::query("UPDATE posts SET updated_at = '2000-01-01 00:00:00' WHERE id = ?")
        .bind(post_id)
        .execute(&pool)
        .await
        .unwrap();

    let mut cmd = update_cmd(1, post_id);
    cmd.tags = Some(vec!["axum".into()]);
    svc.update_post(cmd).await.unwrap();

    let stored: String = sqlx::query_scalar("SELECT updated_at FROM posts WHERE id = ?")
        .bind(post_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_ne!(stored, "2000-01-01 00:00:00");
}

#[tokio::test]
async fn stale_expected_updated_at_is_a_conflict() {
    let pool = setup().await;
    let svc = PostServiceImpl { pool: pool.clone() };

    let post_id = svc
        .new_post(new_post_cmd(1, "two-tabs", vec![]))
        .await
        .unwrap();

    // Tab A loads, then tab B saves.
    let stale: String = sqlx::query_scalar("SELECT updated_at FROM posts WHERE id = ?")
        .bind(post_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE posts SET updated_at = '2099-01-01 00:00:00' WHERE id = ?")
        .bind(post_id)
        .execute(&pool)
        .await
        .unwrap();

    // Tab A now saves against the copy it loaded.
    let mut cmd = update_cmd(1, post_id);
    cmd.expected_updated_at = Some(stale);
    cmd.title = Some("Tab A wins?".to_string());
    let err = svc
        .update_post(cmd)
        .await
        .expect_err("a stale save must be rejected");
    assert!(matches!(err, PostError::Conflict(_)), "got {err:?}");

    let title: String = sqlx::query_scalar("SELECT title FROM posts WHERE id = ?")
        .bind(post_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(title, "A title", "the stale write must not have landed");
}

#[tokio::test]
async fn matching_expected_updated_at_is_accepted_and_chains() {
    let pool = setup().await;
    let svc = PostServiceImpl { pool: pool.clone() };

    let post_id = svc
        .new_post(new_post_cmd(1, "chained-saves", vec![]))
        .await
        .unwrap();

    let current: String = sqlx::query_scalar("SELECT updated_at FROM posts WHERE id = ?")
        .bind(post_id)
        .fetch_one(&pool)
        .await
        .unwrap();

    let mut cmd = update_cmd(1, post_id);
    cmd.expected_updated_at = Some(current);
    cmd.title = Some("Edit one".to_string());
    let after_first = svc
        .update_post(cmd)
        .await
        .expect("a fresh save is accepted");

    // The value handed back must be usable as the token for the next save.
    let mut cmd = update_cmd(1, post_id);
    cmd.expected_updated_at = Some(after_first);
    cmd.title = Some("Edit two".to_string());
    svc.update_post(cmd)
        .await
        .expect("the returned updated_at should chain into the next save");

    let title: String = sqlx::query_scalar("SELECT title FROM posts WHERE id = ?")
        .bind(post_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(title, "Edit two");
}

#[tokio::test]
async fn omitting_expected_updated_at_keeps_last_write_wins() {
    let pool = setup().await;
    let svc = PostServiceImpl { pool: pool.clone() };

    let post_id = svc
        .new_post(new_post_cmd(1, "no-token", vec![]))
        .await
        .unwrap();

    sqlx::query("UPDATE posts SET updated_at = '2099-01-01 00:00:00' WHERE id = ?")
        .bind(post_id)
        .execute(&pool)
        .await
        .unwrap();

    // A client that sends no token behaves exactly as before this change.
    let mut cmd = update_cmd(1, post_id);
    cmd.title = Some("Unlocked".to_string());
    svc.update_post(cmd)
        .await
        .expect("no token means no conflict check");

    let title: String = sqlx::query_scalar("SELECT title FROM posts WHERE id = ?")
        .bind(post_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(title, "Unlocked");
}

#[tokio::test]
async fn update_post_reports_a_missing_post_as_not_found() {
    let pool = setup().await;
    let svc = PostServiceImpl { pool: pool.clone() };

    let err = svc
        .update_post(update_cmd(1, 4242))
        .await
        .expect_err("a missing post should not silently succeed");
    assert!(matches!(err, PostError::PostNotFound), "got {err:?}");
}

#[tokio::test]
async fn update_post_allows_the_owner() {
    let pool = setup().await;
    let svc = PostServiceImpl { pool: pool.clone() };

    let post_id = svc
        .new_post(new_post_cmd(1, "owned-and-edited", vec![]))
        .await
        .unwrap();

    let mut cmd = update_cmd(1, post_id);
    cmd.title = Some("Edited".to_string());
    svc.update_post(cmd).await.expect("the owner may edit");

    let title: String = sqlx::query_scalar("SELECT title FROM posts WHERE id = ?")
        .bind(post_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(title, "Edited");
}

#[tokio::test]
async fn a_missing_media_code_leaves_a_hole_instead_of_failing_the_read() {
    let pool = setup().await;
    sqlx::query("INSERT INTO user_meta (user_id, display_name) VALUES (1, 'Author')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        r#"
        INSERT INTO posts
        (id, user_id, title, slug, content, excerpt, status, content_kind, published_at)
        VALUES (7, 1, 'Sparse media', 'sparse-media', 'body', 'excerpt', 'published', 'post', '2026-01-01 00:00:00')
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    // Two surviving media rows, with the middle code (1) gone. Before this fix
    // the arrays were sized by row count, so code 2 was rejected as oversized
    // and the whole post failed to load.
    for (id, short_name, url) in [
        (101_i64, "first", "media/i/first"),
        (102_i64, "third", "media/i/third"),
    ] {
        sqlx::query(
            r#"
            INSERT INTO media (id, hash, short_name, file_name, file_type, url, size)
            VALUES (?, ?, ?, ?, 'image/webp', ?, 1)
            "#,
        )
        .bind(id)
        .bind(format!("hash-{short_name}"))
        .bind(short_name)
        .bind(format!("{short_name}.webp"))
        .bind(url)
        .execute(&pool)
        .await
        .unwrap();
    }
    sqlx::query(
        r#"
        INSERT INTO post_media_usages (post_id, medium_id, code)
        VALUES (7, 101, 0), (7, 102, 2)
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    let svc = PostServiceImpl { pool: pool.clone() };

    let post = svc
        .get_post(GetPostCommand {
            slug: "sparse-media".to_string(),
        })
        .await
        .expect("a hole must not fail the public post read");
    assert_eq!(
        post.medium_urls,
        vec![
            "media/i/first".to_string(),
            String::new(),
            "media/i/third".to_string()
        ],
        "code 2 must stay at index 2, with an empty slot at the missing code"
    );

    let details = svc
        .get_post_details(GetDetailedPostsCommand {
            required_author_id: None,
            viewing_user_id: 1,
            post_id: 7,
        })
        .await
        .expect("a hole must not fail the dashboard read");
    assert_eq!(
        details.medium_urls,
        vec![
            "media/i/first".to_string(),
            String::new(),
            "media/i/third".to_string()
        ]
    );
    assert_eq!(
        details.medium_short_names,
        vec!["first".to_string(), String::new(), "third".to_string()],
        "the editor decode must see the hole, not a shifted short name"
    );
}

async fn seed_media(pool: &SqlitePool, id: i64, short_name: &str) {
    sqlx::query(
        "INSERT INTO media (id, hash, short_name, file_name, file_type, url, size) \
         VALUES (?, ?, ?, ?, 'image/png', ?, 10)",
    )
    .bind(id)
    .bind(format!("hash-{short_name}"))
    .bind(short_name)
    .bind(format!("{short_name}.png"))
    .bind(format!("media/i/{short_name}"))
    .execute(pool)
    .await
    .unwrap();
}

async fn link_rows(pool: &SqlitePool, post_id: i64) -> Vec<(String, String, Option<i64>)> {
    sqlx::query_as::<_, (String, String, Option<i64>)>(
        "SELECT ref_key, target_kind, target_id FROM content_links \
         WHERE source_post_id = ? ORDER BY ref_key",
    )
    .bind(post_id)
    .fetch_all(pool)
    .await
    .unwrap()
}

#[tokio::test]
async fn new_post_registers_media_and_internal_content_links() {
    let pool = setup().await;
    let svc = PostServiceImpl { pool: pool.clone() };

    let post_id = svc
        .new_post(new_post_cmd(1, "target-post", vec![]))
        .await
        .unwrap();
    let project_id = svc
        .new_post(NewPostCommand {
            content_kind: "project".to_string(),
            ..new_post_cmd(1, "target-project", vec![])
        })
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO series (id, title, slug, description, user_id) \
         VALUES (5, 'S', 'target-series', 'd', 1)",
    )
    .execute(&pool)
    .await
    .unwrap();
    seed_media(&pool, 7, "pic-7").await;

    let mut cmd = new_post_cmd(1, "source-post", vec![]);
    cmd.content = concat!(
        "[a](/posts/target-post) ",
        "[b](https://blog.huuthangle.site/projects/target-project) ",
        "[c](/series/target-series) ",
        "[d](/posts/does-not-exist) ",
        "@[img:pic-7]"
    )
    .to_string();
    cmd.media_usage = HashMap::from([("pic-7".to_string(), 0)]);
    let source_id = svc.new_post(cmd).await.unwrap();

    assert_eq!(
        link_rows(&pool, source_id).await,
        vec![
            (
                format!("content:{post_id}"),
                "content".to_string(),
                Some(post_id)
            ),
            (
                format!("content:{project_id}"),
                "content".to_string(),
                Some(project_id)
            ),
            ("media:7".to_string(), "media".to_string(), Some(7)),
            ("series:5".to_string(), "series".to_string(), Some(5)),
        ],
        "every resolvable link is registered; an unknown slug is skipped"
    );

    let snapshots: Vec<(Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT target_title, target_slug FROM content_links \
         WHERE source_post_id = ? AND target_kind = 'content' ORDER BY ref_key",
    )
    .bind(source_id)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        snapshots,
        vec![
            (Some("A title".to_string()), Some("target-post".to_string())),
            (
                Some("A title".to_string()),
                Some("target-project".to_string())
            ),
        ]
    );
}

#[tokio::test]
async fn update_post_refreshes_the_link_registry() {
    let pool = setup().await;
    let svc = PostServiceImpl { pool: pool.clone() };

    svc.new_post(new_post_cmd(1, "first-target", vec![]))
        .await
        .unwrap();
    let second = svc
        .new_post(new_post_cmd(1, "second-target", vec![]))
        .await
        .unwrap();
    seed_media(&pool, 9, "pic-9").await;

    let mut cmd = new_post_cmd(1, "fresh-links", vec![]);
    cmd.content = "[a](/posts/first-target) @[img:pic-9]".to_string();
    cmd.media_usage = HashMap::from([("pic-9".to_string(), 0)]);
    let source_id = svc.new_post(cmd).await.unwrap();
    assert_eq!(link_rows(&pool, source_id).await.len(), 2);

    // A body replacement with no media usage drops the media row and points at
    // the new target only.
    let mut cmd = update_cmd(1, source_id);
    cmd.content = Some("[b](/posts/second-target)".to_string());
    cmd.media_usage = Some(HashMap::new());
    svc.update_post(cmd).await.unwrap();

    assert_eq!(
        link_rows(&pool, source_id).await,
        vec![(
            format!("content:{second}"),
            "content".to_string(),
            Some(second)
        )],
        "stale content and media rows must not survive a body save"
    );
}

#[tokio::test]
async fn renaming_a_post_keeps_the_old_slug_redirecting() {
    let pool = setup().await;
    let svc = PostServiceImpl { pool: pool.clone() };

    let post_id = svc
        .new_post(new_post_cmd(1, "old-slug", vec![]))
        .await
        .unwrap();
    sqlx::query("UPDATE posts SET status = 'published' WHERE id = ?")
        .bind(post_id)
        .execute(&pool)
        .await
        .unwrap();

    let live = redirects::resolve(&pool, "post", "old-slug")
        .await
        .unwrap()
        .expect("a published post's live slug resolves");
    assert_eq!(live.slug, "old-slug");

    let mut cmd = update_cmd(1, post_id);
    cmd.slug = Some("new-slug".to_string());
    svc.update_post(cmd).await.unwrap();

    let moved = redirects::resolve(&pool, "post", "old-slug")
        .await
        .unwrap()
        .expect("the old slug should still resolve");
    assert_eq!(
        (moved.kind.as_str(), moved.slug.as_str()),
        ("post", "new-slug")
    );
    assert!(
        redirects::resolve(&pool, "project", "old-slug")
            .await
            .unwrap()
            .is_none(),
        "the kind in the URL has to match the row's kind"
    );
    let current = redirects::resolve(&pool, "post", "new-slug")
        .await
        .unwrap()
        .expect("the new slug is the live one");
    assert_eq!(current.slug, "new-slug");
}

#[tokio::test]
async fn renaming_a_project_records_the_project_kind() {
    let pool = setup().await;
    let svc = PostServiceImpl { pool: pool.clone() };

    let project_id = svc
        .new_post(NewPostCommand {
            content_kind: "project".to_string(),
            ..new_post_cmd(1, "old-project", vec![])
        })
        .await
        .unwrap();
    sqlx::query("UPDATE posts SET status = 'published' WHERE id = ?")
        .bind(project_id)
        .execute(&pool)
        .await
        .unwrap();

    let mut cmd = update_cmd(1, project_id);
    cmd.slug = Some("new-project".to_string());
    svc.update_post(cmd).await.unwrap();

    let moved = redirects::resolve(&pool, "project", "old-project")
        .await
        .unwrap()
        .expect("a renamed project redirects under its own kind");
    assert_eq!(moved.slug, "new-project");
    assert!(
        redirects::resolve(&pool, "post", "old-project")
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn app_embeds_and_glb_media_are_registered() {
    let pool = setup().await;
    let svc = PostServiceImpl { pool: pool.clone() };

    let project_id = svc
        .new_post(NewPostCommand {
            content_kind: "project".to_string(),
            ..new_post_cmd(1, "embedded-project", vec![])
        })
        .await
        .unwrap();
    let game_id = svc
        .new_post(NewPostCommand {
            content_kind: "game".to_string(),
            ..new_post_cmd(1, "embedded-game", vec![])
        })
        .await
        .unwrap();
    seed_media(&pool, 11, "model").await;

    let mut cmd = new_post_cmd(1, "embed-source", vec![]);
    cmd.content =
        ":::app project embedded-project\n\n:::app game embedded-game\n\n:::app glb-demo model"
            .to_string();
    // The handlers collect the glb name before saving; the body keeps the name
    // instead of an index.
    cmd.media_usage = HashMap::from([("model".to_string(), 0)]);
    let source_id = svc.new_post(cmd).await.unwrap();

    let mut expected = vec![
        (
            format!("content:{project_id}"),
            "content".to_string(),
            Some(project_id),
        ),
        (
            format!("content:{game_id}"),
            "content".to_string(),
            Some(game_id),
        ),
        ("media:11".to_string(), "media".to_string(), Some(11)),
    ];
    expected.sort();
    assert_eq!(
        link_rows(&pool, source_id).await,
        expected,
        "app embeds register their project or game, and a glb embed registers its media"
    );
}

#[tokio::test]
async fn link_health_reports_moved_and_missing_targets() {
    let pool = setup().await;
    let svc = PostServiceImpl { pool: pool.clone() };

    let moved = svc
        .new_post(NewPostCommand {
            content_kind: "project".to_string(),
            ..new_post_cmd(1, "target-project", vec![])
        })
        .await
        .unwrap();
    let missing = svc
        .new_post(NewPostCommand {
            content_kind: "game".to_string(),
            ..new_post_cmd(1, "target-game", vec![])
        })
        .await
        .unwrap();
    for id in [moved, missing] {
        sqlx::query("UPDATE posts SET status = 'published' WHERE id = ?")
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
    }

    let mut cmd = new_post_cmd(1, "health-source", vec![]);
    cmd.content = "[a](/projects/target-project) [b](/games/target-game)".to_string();
    let source_id = svc.new_post(cmd).await.unwrap();
    sqlx::query("UPDATE posts SET status = 'published' WHERE id = ?")
        .bind(source_id)
        .execute(&pool)
        .await
        .unwrap();

    let mut rename = update_cmd(1, moved);
    rename.slug = Some("renamed-project".to_string());
    svc.update_post(rename).await.unwrap();

    sqlx::query("UPDATE posts SET deleted_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(missing)
        .execute(&pool)
        .await
        .unwrap();

    let states = content_links::health_for(&pool, "health-source")
        .await
        .unwrap()
        .expect("a published source has link health");
    assert_eq!(states.len(), 2);

    let moved_state = states
        .iter()
        .find(|state| state.entity_kind == "project")
        .unwrap();
    assert_eq!(
        (
            moved_state.status.as_str(),
            moved_state.target_slug.as_deref(),
            moved_state.current_slug.as_deref()
        ),
        ("moved", Some("target-project"), Some("renamed-project"))
    );

    let missing_state = states
        .iter()
        .find(|state| state.entity_kind == "game")
        .unwrap();
    assert_eq!(
        (
            missing_state.status.as_str(),
            missing_state.current_slug.as_deref()
        ),
        ("missing", None)
    );

    assert!(
        content_links::health_for(&pool, "nobody")
            .await
            .unwrap()
            .is_none(),
        "an unknown or unpublished source has no link report"
    );
}

#[tokio::test]
async fn a_reused_slug_serves_the_live_post_instead_of_the_history() {
    let pool = setup().await;
    let svc = PostServiceImpl { pool: pool.clone() };

    let first = svc
        .new_post(new_post_cmd(1, "shared-slug", vec![]))
        .await
        .unwrap();
    sqlx::query("UPDATE posts SET status = 'published' WHERE id = ?")
        .bind(first)
        .execute(&pool)
        .await
        .unwrap();

    let mut cmd = update_cmd(1, first);
    cmd.slug = Some("renamed-away".to_string());
    svc.update_post(cmd).await.unwrap();

    let second = svc
        .new_post(new_post_cmd(1, "shared-slug", vec![]))
        .await
        .unwrap();
    sqlx::query("UPDATE posts SET status = 'published' WHERE id = ?")
        .bind(second)
        .execute(&pool)
        .await
        .unwrap();

    let resolved = redirects::resolve(&pool, "post", "shared-slug")
        .await
        .unwrap()
        .expect("the live holder resolves");
    assert_eq!(
        resolved.slug, "shared-slug",
        "the live row wins over slug_history, so the loader does not redirect"
    );
}
