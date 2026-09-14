//! End-to-end persistence tests for the audiobook module.
//!
//! These run the real migrations against a throwaway SQLite file and drive the
//! actual `AudiobookServiceImpl`, so they cover the SQL, the ordering
//! bookkeeping, and the permission/validation rules rather than a mock.

use std::path::PathBuf;

use axum::body::Bytes;
use backend::{
    application::{
        commands::audiobook::{
            AddTrackCommand, ChangeAudiobookStatusCommand, CheckAudiobookSlugCommand,
            DeleteAudiobookCommand, GetAudiobookCommand, GetAudiobooksCommand,
            GetPublicAudiobookCommand, GetPublicAudiobooksCommand, NewAudiobookCommand,
            RemoveTrackCommand, ReorderTracksCommand, UpdateAudiobookCommand, UpdateTrackCommand,
        },
        services::audiobook::AudiobookService,
    },
    domain::{
        entities::{media::MediaType, media::MediumDetails},
        errors::audiobook::AudiobookError,
    },
    infrastructure::{
        persistence::audiobook::AudiobookServiceImpl, web::server::MediaConfig,
    },
};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};

struct Fixture {
    db_path: PathBuf,
    media_dir: PathBuf,
    service: AudiobookServiceImpl,
}

async fn fixture(name: &str) -> Fixture {
    let stamp = format!("{}-{}", std::process::id(), name);
    let db_path = std::env::temp_dir().join(format!("abk-{stamp}.db"));
    let media_dir = std::env::temp_dir().join(format!("abk-media-{stamp}"));
    let _ = std::fs::remove_file(&db_path);
    let _ = std::fs::remove_dir_all(&media_dir);
    std::fs::create_dir_all(&media_dir).unwrap();

    let url = format!("sqlite://{}", db_path.display());
    let opts = url
        .parse::<SqliteConnectOptions>()
        .unwrap()
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await
        .unwrap();

    sqlx::migrate::Migrator::new(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("migrations"),
    )
    .await
    .unwrap()
    .run(&pool)
    .await
    .unwrap();

    sqlx::query("INSERT INTO users (id,username,email,password_hash) VALUES (1,'alice','a@x.com','h')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO users (id,username,email,password_hash) VALUES (2,'bob','b@x.com','h')")
        .execute(&pool)
        .await
        .unwrap();

    Fixture {
        db_path,
        media_dir,
        service: AudiobookServiceImpl::new(pool),
    }
}

fn config(fx: &Fixture) -> MediaConfig {
    MediaConfig {
        dir: fx.media_dir.clone(),
        allowed_file_types: vec![MediaType::ImagePng, MediaType::AudioMp3],
        allowed_avatar_types: vec![MediaType::ImagePng],
        allowed_cover_types: vec![MediaType::ImagePng, MediaType::ImageJpeg],
        allowed_audio_types: vec![MediaType::AudioMp3, MediaType::AudioOgg],
    }
}

fn audio(name: &str, seed: u8) -> MediumDetails {
    // Distinct byte payloads so each upload hashes to a different file.
    let bytes: Vec<u8> = (0..64u8).map(|i| i.wrapping_add(seed)).collect();
    MediumDetails {
        filename: name.to_string(),
        content_type: "audio/mpeg".to_string(),
        bytes: Bytes::from(bytes),
    }
}

impl Fixture {
    async fn cleanup(self) {
        self.service.pool.close().await;
        let _ = std::fs::remove_file(&self.db_path);
        let _ = std::fs::remove_file(self.db_path.with_extension("db-wal"));
        let _ = std::fs::remove_file(self.db_path.with_extension("db-shm"));
        let _ = std::fs::remove_dir_all(&self.media_dir);
    }

    async fn create(&self, slug: &str, tags: &[&str]) -> i64 {
        self.service
            .new_audiobook(
                NewAudiobookCommand {
                    user_id: 1,
                    title: format!("Book {slug}"),
                    slug: slug.to_string(),
                    description: "A test audiobook".to_string(),
                    translator: Some("Tran Slator".to_string()),
                    tags: tags.iter().map(|t| t.to_string()).collect(),
                    cover_image: None,
                },
                &config(self),
            )
            .await
            .unwrap()
    }

    async fn add_track(&self, audiobook_id: i64, title: &str, seed: u8, number: Option<i64>) -> i64 {
        self.service
            .add_track(
                AddTrackCommand {
                    audiobook_id,
                    user_id: 1,
                    is_admin: false,
                    title: title.to_string(),
                    number,
                    duration_seconds: Some(120),
                    medium: audio(&format!("{title}.mp3"), seed),
                },
                &config(self),
            )
            .await
            .unwrap()
    }

    async fn track_numbers(&self, audiobook_id: i64) -> Vec<(String, i64)> {
        let details = self
            .service
            .get_audiobook(GetAudiobookCommand {
                audiobook_id,
                user_id: 1,
                is_admin: false,
            })
            .await
            .unwrap();
        details
            .tracks
            .into_iter()
            .map(|t| (t.title, t.number))
            .collect()
    }
}

#[tokio::test]
async fn migration_creates_audiobook_schema() {
    let fx = fixture("schema").await;
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type='table'
         AND name IN ('audiobooks','audiobook_tracks','audiobook_tags','audiobook_tag_links')
         ORDER BY name",
    )
    .fetch_all(&fx.service.pool)
    .await
    .unwrap();

    assert_eq!(
        tables,
        vec![
            "audiobook_tag_links",
            "audiobook_tags",
            "audiobook_tracks",
            "audiobooks"
        ]
    );
    fx.cleanup().await;
}

#[tokio::test]
async fn tracks_append_in_order_and_keep_contiguous_numbering() {
    let fx = fixture("order").await;
    let book = fx.create("ordered-book", &[]).await;

    let first = fx.add_track(book, "Chapter One", 1, None).await;
    let second = fx.add_track(book, "Chapter Two", 2, None).await;
    let third = fx.add_track(book, "Chapter Three", 3, None).await;

    assert_eq!(
        fx.track_numbers(book).await,
        vec![
            ("Chapter One".to_string(), 1),
            ("Chapter Two".to_string(), 2),
            ("Chapter Three".to_string(), 3),
        ]
    );

    // Removing the middle track must close the gap, not leave 1,3.
    fx.service
        .remove_track(RemoveTrackCommand {
            audiobook_id: book,
            track_id: second,
            user_id: 1,
            is_admin: false,
        })
        .await
        .unwrap();

    assert_eq!(
        fx.track_numbers(book).await,
        vec![
            ("Chapter One".to_string(), 1),
            ("Chapter Three".to_string(), 2),
        ]
    );

    // Inserting at an explicit position shifts the following tracks down.
    let inserted = fx.add_track(book, "Interlude", 9, Some(1)).await;
    assert_eq!(
        fx.track_numbers(book).await,
        vec![
            ("Interlude".to_string(), 1),
            ("Chapter One".to_string(), 2),
            ("Chapter Three".to_string(), 3),
        ]
    );

    let _ = (first, third, inserted);
    fx.cleanup().await;
}

#[tokio::test]
async fn reorder_requires_an_exact_permutation() {
    let fx = fixture("reorder").await;
    let book = fx.create("reorder-book", &[]).await;
    let a = fx.add_track(book, "A", 1, None).await;
    let b = fx.add_track(book, "B", 2, None).await;
    let c = fx.add_track(book, "C", 3, None).await;

    fx.service
        .reorder_tracks(ReorderTracksCommand {
            audiobook_id: book,
            user_id: 1,
            is_admin: false,
            order: vec![c, a, b],
        })
        .await
        .unwrap();

    assert_eq!(
        fx.track_numbers(book).await,
        vec![
            ("C".to_string(), 1),
            ("A".to_string(), 2),
            ("B".to_string(), 3),
        ]
    );

    // A partial list must be rejected rather than silently dropping a track.
    let err = fx
        .service
        .reorder_tracks(ReorderTracksCommand {
            audiobook_id: book,
            user_id: 1,
            is_admin: false,
            order: vec![a, b],
        })
        .await
        .unwrap_err();
    assert!(matches!(err, AudiobookError::Validation(_)), "got {err:?}");

    // A duplicated id is not a permutation either.
    let err = fx
        .service
        .reorder_tracks(ReorderTracksCommand {
            audiobook_id: book,
            user_id: 1,
            is_admin: false,
            order: vec![a, a, b],
        })
        .await
        .unwrap_err();
    assert!(matches!(err, AudiobookError::Validation(_)), "got {err:?}");

    // Ordering survived the rejected calls.
    assert_eq!(fx.track_numbers(book).await[0].0, "C");
    fx.cleanup().await;
}

#[tokio::test]
async fn moving_a_track_up_and_down_shifts_the_others() {
    let fx = fixture("move").await;
    let book = fx.create("move-book", &[]).await;
    let a = fx.add_track(book, "A", 1, None).await;
    let _b = fx.add_track(book, "B", 2, None).await;
    let _c = fx.add_track(book, "C", 3, None).await;

    // A moves from 1 to 3: B and C shift up.
    fx.service
        .update_track(UpdateTrackCommand {
            audiobook_id: book,
            track_id: a,
            user_id: 1,
            is_admin: false,
            title: None,
            number: Some(3),
            duration_seconds: None,
        })
        .await
        .unwrap();

    assert_eq!(
        fx.track_numbers(book).await,
        vec![
            ("B".to_string(), 1),
            ("C".to_string(), 2),
            ("A".to_string(), 3),
        ]
    );

    // A moves back to 1: B and C shift down again.
    fx.service
        .update_track(UpdateTrackCommand {
            audiobook_id: book,
            track_id: a,
            user_id: 1,
            is_admin: false,
            title: Some("A renamed".to_string()),
            number: Some(1),
            duration_seconds: None,
        })
        .await
        .unwrap();

    assert_eq!(
        fx.track_numbers(book).await,
        vec![
            ("A renamed".to_string(), 1),
            ("B".to_string(), 2),
            ("C".to_string(), 3),
        ]
    );
    fx.cleanup().await;
}

#[tokio::test]
async fn dedicated_tags_are_separate_from_global_tags_and_dedupe_by_slug() {
    let fx = fixture("tags").await;
    let book = fx
        .create("tagged-book", &["Science Fiction", "science fiction", "Drama"])
        .await;

    let details = fx
        .service
        .get_audiobook(GetAudiobookCommand {
            audiobook_id: book,
            user_id: 1,
            is_admin: false,
        })
        .await
        .unwrap();

    // "Science Fiction" and "science fiction" collapse onto one slug.
    let names: Vec<&str> = details.tags.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names.len(), 2, "got {names:?}");
    assert!(names.contains(&"Drama"));

    // The audiobook vocabulary must not appear in the global tags table.
    let global: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tags")
        .fetch_one(&fx.service.pool)
        .await
        .unwrap();
    assert_eq!(global, 0, "audiobook tags must not touch global tags");

    let scoped: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audiobook_tags")
        .fetch_one(&fx.service.pool)
        .await
        .unwrap();
    assert_eq!(scoped, 2);

    // Replacing tags removes the old links.
    fx.service
        .update_audiobook(UpdateAudiobookCommand {
            audiobook_id: book,
            user_id: 1,
            is_admin: false,
            title: None,
            slug: None,
            description: None,
            translator: None,
            tags: Some(vec!["Mystery".to_string()]),
        })
        .await
        .unwrap();

    let details = fx
        .service
        .get_audiobook(GetAudiobookCommand {
            audiobook_id: book,
            user_id: 1,
            is_admin: false,
        })
        .await
        .unwrap();
    assert_eq!(
        details.tags.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(),
        vec!["Mystery"]
    );
    fx.cleanup().await;
}

#[tokio::test]
async fn publishing_requires_a_track_and_gates_the_public_feed() {
    let fx = fixture("publish").await;
    let book = fx.create("publish-book", &["Fiction"]).await;

    // Draft with no tracks cannot be published.
    let err = fx
        .service
        .change_audiobook_status(ChangeAudiobookStatusCommand {
            audiobook_id: book,
            user_id: 1,
            is_admin: false,
            status: "published".to_string(),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, AudiobookError::Validation(_)), "got {err:?}");

    fx.add_track(book, "Chapter One", 1, None).await;

    // An invalid status is rejected.
    let err = fx
        .service
        .change_audiobook_status(ChangeAudiobookStatusCommand {
            audiobook_id: book,
            user_id: 1,
            is_admin: false,
            status: "live".to_string(),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, AudiobookError::Validation(_)), "got {err:?}");

    // Still hidden from the public feed while it is a draft.
    let public = fx
        .service
        .get_public_audiobooks(GetPublicAudiobooksCommand {
            term: None,
            tag: None,
            limit: 50,
            offset: 0,
        })
        .await
        .unwrap();
    assert!(public.is_empty());

    fx.service
        .change_audiobook_status(ChangeAudiobookStatusCommand {
            audiobook_id: book,
            user_id: 1,
            is_admin: false,
            status: "published".to_string(),
        })
        .await
        .unwrap();

    let public = fx
        .service
        .get_public_audiobooks(GetPublicAudiobooksCommand {
            term: None,
            tag: None,
            limit: 50,
            offset: 0,
        })
        .await
        .unwrap();
    assert_eq!(public.len(), 1);
    assert_eq!(public[0].slug, "publish-book");
    assert_eq!(public[0].track_count, 1);
    assert_eq!(public[0].total_duration_seconds, 120);
    assert_eq!(public[0].translator.as_deref(), Some("Tran Slator"));
    assert_eq!(public[0].tags, vec!["Fiction".to_string()]);
    // The cover-less audiobook has no media URL to render.
    assert!(public[0].url.is_none());

    // The tag filter matches the dedicated tag slug.
    let filtered = fx
        .service
        .get_public_audiobooks(GetPublicAudiobooksCommand {
            term: None,
            tag: Some("fiction".to_string()),
            limit: 50,
            offset: 0,
        })
        .await
        .unwrap();
    assert_eq!(filtered.len(), 1);

    let no_match = fx
        .service
        .get_public_audiobooks(GetPublicAudiobooksCommand {
            term: None,
            tag: Some("nonexistent".to_string()),
            limit: 50,
            offset: 0,
        })
        .await
        .unwrap();
    assert!(no_match.is_empty());

    // The public detail feed resolves by slug and exposes playable track URLs.
    let details = fx
        .service
        .get_public_audiobook(GetPublicAudiobookCommand {
            slug: "publish-book".to_string(),
        })
        .await
        .unwrap();
    assert_eq!(details.tracks.len(), 1);
    assert_eq!(details.tracks[0].number, 1);
    assert!(details.tracks[0].url.starts_with("media/i/"));
    assert_eq!(details.tracks[0].file_type, "audio/mpeg");
    assert_eq!(details.owner_username, "alice");
    fx.cleanup().await;
}

#[tokio::test]
async fn unpublished_audiobooks_are_not_reachable_publicly() {
    let fx = fixture("hidden").await;
    fx.create("draft-book", &[]).await;

    let err = fx
        .service
        .get_public_audiobook(GetPublicAudiobookCommand {
            slug: "draft-book".to_string(),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, AudiobookError::NotFound), "got {err:?}");
    fx.cleanup().await;
}

#[tokio::test]
async fn ownership_is_enforced_and_slugs_are_unique() {
    let fx = fixture("authz").await;
    let book = fx.create("owned-book", &[]).await;

    // Another user cannot read or mutate it.
    let err = fx
        .service
        .get_audiobook(GetAudiobookCommand {
            audiobook_id: book,
            user_id: 2,
            is_admin: false,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, AudiobookError::NotFound), "got {err:?}");

    let err = fx
        .service
        .update_audiobook(UpdateAudiobookCommand {
            audiobook_id: book,
            user_id: 2,
            is_admin: false,
            title: Some("Hijacked".to_string()),
            slug: None,
            description: None,
            translator: None,
            tags: None,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, AudiobookError::PermissionDenied), "got {err:?}");

    // An admin can read and mutate anything.
    fx.service
        .update_audiobook(UpdateAudiobookCommand {
            audiobook_id: book,
            user_id: 2,
            is_admin: true,
            title: Some("Admin edit".to_string()),
            slug: None,
            description: None,
            translator: None,
            tags: None,
        })
        .await
        .unwrap();

    // Duplicate slugs are rejected on create...
    let err = fx
        .service
        .new_audiobook(
            NewAudiobookCommand {
                user_id: 1,
                title: "Another".to_string(),
                slug: "owned-book".to_string(),
                description: String::new(),
                translator: None,
                tags: vec![],
                cover_image: None,
            },
            &config(&fx),
        )
        .await
        .unwrap_err();
    assert!(matches!(err, AudiobookError::Duplication), "got {err:?}");

    // ...and on rename.
    let other = fx.create("other-book", &[]).await;
    let err = fx
        .service
        .update_audiobook(UpdateAudiobookCommand {
            audiobook_id: other,
            user_id: 1,
            is_admin: false,
            title: None,
            slug: Some("owned-book".to_string()),
            description: None,
            translator: None,
            tags: None,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, AudiobookError::Duplication), "got {err:?}");

    // Renaming to its own slug is allowed (it is not a conflict with itself).
    fx.service
        .update_audiobook(UpdateAudiobookCommand {
            audiobook_id: other,
            user_id: 1,
            is_admin: false,
            title: None,
            slug: Some("other-book".to_string()),
            description: None,
            translator: None,
            tags: None,
        })
        .await
        .unwrap();

    fx.cleanup().await;
}

#[tokio::test]
async fn slug_availability_and_validation() {
    let fx = fixture("slug").await;
    fx.create("taken-slug", &[]).await;

    let free = fx
        .service
        .check_audiobook_slug(CheckAudiobookSlugCommand {
            slug: "free-slug".to_string(),
        })
        .await
        .unwrap();
    assert!(free);

    let taken = fx
        .service
        .check_audiobook_slug(CheckAudiobookSlugCommand {
            slug: "taken-slug".to_string(),
        })
        .await
        .unwrap();
    assert!(!taken);

    // A slug that cannot be normalized is an error, not a false "available".
    let err = fx
        .service
        .check_audiobook_slug(CheckAudiobookSlugCommand {
            slug: "Not A Slug".to_string(),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, AudiobookError::Validation(_)), "got {err:?}");
    fx.cleanup().await;
}

#[tokio::test]
async fn track_uploads_reject_non_audio_and_metadata_is_validated() {
    let fx = fixture("validation").await;
    let book = fx.create("validation-book", &[]).await;

    // An image dressed up as a track must be refused.
    let err = fx
        .service
        .add_track(
            AddTrackCommand {
                audiobook_id: book,
                user_id: 1,
                is_admin: false,
                title: "Sneaky".to_string(),
                number: None,
                duration_seconds: None,
                medium: MediumDetails {
                    filename: "cover.png".to_string(),
                    content_type: "image/png".to_string(),
                    bytes: Bytes::from_static(b"not-audio"),
                },
            },
            &config(&fx),
        )
        .await
        .unwrap_err();
    assert!(matches!(err, AudiobookError::Media(_)), "got {err:?}");

    // A blank track title is refused.
    let err = fx
        .service
        .add_track(
            AddTrackCommand {
                audiobook_id: book,
                user_id: 1,
                is_admin: false,
                title: "   ".to_string(),
                number: None,
                duration_seconds: None,
                medium: audio("ok.mp3", 5),
            },
            &config(&fx),
        )
        .await
        .unwrap_err();
    assert!(matches!(err, AudiobookError::Validation(_)), "got {err:?}");

    // A blank title is also refused on create.
    let err = fx
        .service
        .new_audiobook(
            NewAudiobookCommand {
                user_id: 1,
                title: "  ".to_string(),
                slug: "blank-title".to_string(),
                description: String::new(),
                translator: None,
                tags: vec![],
                cover_image: None,
            },
            &config(&fx),
        )
        .await
        .unwrap_err();
    assert!(matches!(err, AudiobookError::Validation(_)), "got {err:?}");

    // A too-long tag name is refused.
    let err = fx
        .service
        .update_audiobook(UpdateAudiobookCommand {
            audiobook_id: book,
            user_id: 1,
            is_admin: false,
            title: None,
            slug: None,
            description: None,
            translator: None,
            tags: Some(vec!["x".repeat(200)]),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, AudiobookError::Validation(_)), "got {err:?}");

    fx.cleanup().await;
}

#[tokio::test]
async fn deleting_an_audiobook_cascades_tracks_but_keeps_media() {
    let fx = fixture("delete").await;
    let book = fx.create("delete-book", &["Temp"]).await;
    fx.add_track(book, "One", 1, None).await;
    fx.add_track(book, "Two", 2, None).await;

    fx.service
        .delete_audiobook(DeleteAudiobookCommand {
            audiobook_id: book,
            user_id: 1,
            is_admin: false,
        })
        .await
        .unwrap();

    let tracks: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audiobook_tracks")
        .fetch_one(&fx.service.pool)
        .await
        .unwrap();
    assert_eq!(tracks, 0, "tracks cascade with the audiobook");

    let links: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audiobook_tag_links")
        .fetch_one(&fx.service.pool)
        .await
        .unwrap();
    assert_eq!(links, 0, "tag links cascade with the audiobook");

    // Tag vocabulary and media survive: both are reusable, and media files are
    // content-addressed and owned by the media manager.
    let tags: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audiobook_tags")
        .fetch_one(&fx.service.pool)
        .await
        .unwrap();
    assert_eq!(tags, 1);

    let media: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM media")
        .fetch_one(&fx.service.pool)
        .await
        .unwrap();
    assert_eq!(media, 2);

    let err = fx
        .service
        .get_audiobook(GetAudiobookCommand {
            audiobook_id: book,
            user_id: 1,
            is_admin: false,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, AudiobookError::NotFound), "got {err:?}");
    fx.cleanup().await;
}

#[tokio::test]
async fn dashboard_listing_is_scoped_to_the_caller() {
    let fx = fixture("listing").await;

    // Two of alice's, one of bob's.
    let a1 = fx.create("alice-one", &[]).await;
    fx.create("alice-two", &[]).await;
    fx.service
        .new_audiobook(
            NewAudiobookCommand {
                user_id: 2,
                title: "Bob's book".to_string(),
                slug: "bob-one".to_string(),
                description: String::new(),
                translator: None,
                tags: vec![],
                cover_image: None,
            },
            &config(&fx),
        )
        .await
        .unwrap();

    fx.add_track(a1, "Only track", 1, None).await;

    let alice = fx
        .service
        .get_audiobooks(GetAudiobooksCommand {
            user_id: 1,
            is_admin: false,
            term: None,
            limit: 50,
            offset: 0,
        })
        .await
        .unwrap();
    assert_eq!(alice.len(), 2);
    assert!(alice.iter().all(|a| a.slug.starts_with("alice")));

    let admin = fx
        .service
        .get_audiobooks(GetAudiobooksCommand {
            user_id: 1,
            is_admin: true,
            term: None,
            limit: 50,
            offset: 0,
        })
        .await
        .unwrap();
    assert_eq!(admin.len(), 3);

    // Search narrows by title.
    let searched = fx
        .service
        .get_audiobooks(GetAudiobooksCommand {
            user_id: 1,
            is_admin: true,
            term: Some("bob".to_string()),
            limit: 50,
            offset: 0,
        })
        .await
        .unwrap();
    assert_eq!(searched.len(), 1);
    assert_eq!(searched[0].slug, "bob-one");

    fx.cleanup().await;
}

#[tokio::test]
async fn duplicate_audio_bytes_share_one_file_on_disk() {
    let fx = fixture("dedupe").await;
    let book = fx.create("dedupe-book", &[]).await;

    // Same payload twice: two media rows, but one file on disk.
    let medium = audio("same.mp3", 7);
    for title in ["Take A", "Take B"] {
        fx.service
            .add_track(
                AddTrackCommand {
                    audiobook_id: book,
                    user_id: 1,
                    is_admin: false,
                    title: title.to_string(),
                    number: None,
                    duration_seconds: None,
                    medium: medium.clone(),
                },
                &config(&fx),
            )
            .await
            .unwrap();
    }

    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT short_name, hash FROM media ORDER BY id")
            .fetch_all(&fx.service.pool)
            .await
            .unwrap();
    assert_eq!(rows.len(), 2);
    // Content-addressed: identical bytes, identical hash, distinct short_names.
    assert_eq!(rows[0].1, rows[1].1);
    assert_ne!(rows[0].0, rows[1].0);

    let files = std::fs::read_dir(&fx.media_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .flat_map(|dir| {
            std::fs::read_dir(dir.path())
                .unwrap()
                .filter_map(|e| e.ok())
                .flat_map(|d| {
                    std::fs::read_dir(d.path())
                        .unwrap()
                        .filter_map(|e| e.ok())
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        })
        .count();
    assert_eq!(files, 1, "identical audio must be stored once");

    fx.cleanup().await;
}
