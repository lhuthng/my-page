//! Integration tests for the post save path against a real migrated SQLite DB.
//!
//! These cover the two regressions fixed alongside the editor hardening work:
//! the tag/media placeholder-bind mismatch, and the missing ownership predicate
//! on `update_post`.

use std::collections::HashMap;

use backend::application::commands::post::{NewPostCommand, UpdatePostCommand};
use backend::application::services::post::PostService;
use backend::domain::errors::post::PostError;
use backend::infrastructure::persistence::post::PostServiceImpl;
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
        draft: None,
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
