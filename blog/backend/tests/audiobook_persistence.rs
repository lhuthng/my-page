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
            RecordTrackPlayCommand, RemoveTrackCommand, ReplaceTrackMediumCommand,
            UpdateAudiobookCommand, UpdateTrackCommand,
        },
        commands::media::{UploadMediaWithoutDescriptionCommand, UploadMediumCommand},
        services::{audiobook::AudiobookService, media::MediaService},
    },
    domain::{
        entities::{audiobook::AudiobookDetails, media::MediaType, media::MediumDetails},
        errors::audiobook::AudiobookError,
    },
    infrastructure::{
        persistence::{
            audiobook::AudiobookServiceImpl,
            media::{MEDIA_REFERENCES, MediaServiceImpl, hash_bytes},
        },
        storage::{chunk_dir, chunk_key, manifest_key, media_key},
        web::server::MediaConfig,
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

    sqlx::migrate::Migrator::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("migrations"))
        .await
        .unwrap()
        .run(&pool)
        .await
        .unwrap();

    sqlx::query(
        "INSERT INTO users (id,username,email,password_hash) VALUES (1,'alice','a@x.com','h')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO users (id,username,email,password_hash) VALUES (2,'bob','b@x.com','h')",
    )
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
        audio_bucket: None,
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

    async fn add_track(
        &self,
        audiobook_id: i64,
        title: &str,
        seed: u8,
        number: Option<i64>,
    ) -> i64 {
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
async fn chapters_keep_the_slot_they_are_given() {
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

    // A chapter's number is a slot, not a position: removing the middle one
    // leaves the gap where it was instead of closing up to 1, 2.
    fx.service
        .remove_track(
            RemoveTrackCommand {
                audiobook_id: book,
                track_id: second,
                user_id: 1,
                is_admin: false,
            },
            &config(&fx),
        )
        .await
        .unwrap();

    assert_eq!(
        fx.track_numbers(book).await,
        vec![
            ("Chapter One".to_string(), 1),
            ("Chapter Three".to_string(), 3),
        ]
    );

    // A chapter named 5 takes slot 5 and leaves 1 and 3 exactly as they are —
    // which is the whole point of slots: the editor sends the number it read
    // off the file name, and a batch must not renumber the rest of the book.
    let inserted = fx.add_track(book, "Interlude", 9, Some(5)).await;
    assert_eq!(
        fx.track_numbers(book).await,
        vec![
            ("Chapter One".to_string(), 1),
            ("Chapter Three".to_string(), 3),
            ("Interlude".to_string(), 5),
        ]
    );

    // With no slot named it lands past the highest, rather than at `count + 1`,
    // which would land on a slot another chapter already holds.
    let appended = fx.add_track(book, "Epilogue", 10, None).await;
    assert_eq!(
        fx.track_numbers(book).await,
        vec![
            ("Chapter One".to_string(), 1),
            ("Chapter Three".to_string(), 3),
            ("Interlude".to_string(), 5),
            ("Epilogue".to_string(), 6),
        ]
    );

    let _ = (first, third, inserted, appended);
    fx.cleanup().await;
}

#[tokio::test]
async fn moving_a_chapter_moves_only_that_chapter() {
    let fx = fixture("move").await;
    let book = fx.create("move-book", &[]).await;
    let a = fx.add_track(book, "A", 1, None).await;
    let _b = fx.add_track(book, "B", 2, None).await;
    let _c = fx.add_track(book, "C", 3, None).await;

    // A moves from slot 1 to slot 4: a free slot, so nothing gives way.
    fx.service
        .update_track(UpdateTrackCommand {
            audiobook_id: book,
            track_id: a,
            user_id: 1,
            is_admin: false,
            title: Some("A renamed".to_string()),
            number: Some(4),
            duration_seconds: None,
        })
        .await
        .unwrap();

    assert_eq!(
        fx.track_numbers(book).await,
        vec![
            ("B".to_string(), 2),
            ("C".to_string(), 3),
            ("A renamed".to_string(), 4),
        ]
    );

    // A slot another chapter holds is a collision rather than a move: B keeps
    // its slot and A stays where it is.
    let err = fx
        .service
        .update_track(UpdateTrackCommand {
            audiobook_id: book,
            track_id: a,
            user_id: 1,
            is_admin: false,
            title: None,
            number: Some(2),
            duration_seconds: None,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, AudiobookError::Validation(_)), "got {err:?}");

    assert_eq!(
        fx.track_numbers(book).await,
        vec![
            ("B".to_string(), 2),
            ("C".to_string(), 3),
            ("A renamed".to_string(), 4),
        ]
    );
    fx.cleanup().await;
}

#[tokio::test]
async fn replacing_a_track_swaps_its_audio_without_moving_it() {
    let fx = fixture("replace").await;
    let book = fx.create("replace-book", &[]).await;
    let first = fx.add_track(book, "One", 1, None).await;
    let _second = fx.add_track(book, "Two", 2, None).await;

    let before = fx
        .service
        .get_audiobook(GetAudiobookCommand {
            audiobook_id: book,
            user_id: 1,
            is_admin: false,
        })
        .await
        .unwrap();
    let old_short_name = before
        .tracks
        .iter()
        .find(|t| t.id == first)
        .unwrap()
        .short_name
        .clone();

    fx.service
        .replace_track_medium(
            ReplaceTrackMediumCommand {
                audiobook_id: book,
                track_id: first,
                user_id: 1,
                is_admin: false,
                title: Some("One (remastered)".to_string()),
                duration_seconds: Some(300),
                medium: audio("one-take-two.mp3", 33),
            },
            &config(&fx),
        )
        .await
        .unwrap();

    let after = fx
        .service
        .get_audiobook(GetAudiobookCommand {
            audiobook_id: book,
            user_id: 1,
            is_admin: false,
        })
        .await
        .unwrap();
    assert_eq!(after.tracks.len(), 2);
    let track = after.tracks.iter().find(|t| t.id == first).unwrap();
    assert_eq!(track.number, 1, "replacement must not move the track");
    assert_eq!(track.title, "One (remastered)");
    assert_eq!(track.duration_seconds, Some(300));
    assert_ne!(
        track.short_name, old_short_name,
        "the track must point at the new file"
    );
    assert_eq!(after.tracks[1].title, "Two", "other tracks are untouched");

    // A non-audio payload is refused and leaves the current audio in place.
    let err = fx
        .service
        .replace_track_medium(
            ReplaceTrackMediumCommand {
                audiobook_id: book,
                track_id: first,
                user_id: 1,
                is_admin: false,
                title: None,
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

    let unchanged = fx
        .service
        .get_audiobook(GetAudiobookCommand {
            audiobook_id: book,
            user_id: 1,
            is_admin: false,
        })
        .await
        .unwrap();
    assert_eq!(
        unchanged
            .tracks
            .iter()
            .find(|t| t.id == first)
            .unwrap()
            .short_name,
        track.short_name,
        "a rejected replacement must not touch the track"
    );

    // Another user may not replace audio on someone else's book.
    let err = fx
        .service
        .replace_track_medium(
            ReplaceTrackMediumCommand {
                audiobook_id: book,
                track_id: first,
                user_id: 2,
                is_admin: false,
                title: None,
                duration_seconds: None,
                medium: audio("hijack.mp3", 44),
            },
            &config(&fx),
        )
        .await
        .unwrap_err();
    assert!(
        matches!(err, AudiobookError::PermissionDenied),
        "got {err:?}"
    );

    fx.cleanup().await;
}

#[tokio::test]
async fn dedicated_tags_are_separate_from_global_tags_and_dedupe_by_slug() {
    let fx = fixture("tags").await;
    let book = fx
        .create(
            "tagged-book",
            &["Science Fiction", "science fiction", "Drama"],
        )
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
        details
            .tags
            .iter()
            .map(|t| t.name.as_str())
            .collect::<Vec<_>>(),
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
    assert!(public.audiobooks.is_empty());

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
    assert_eq!(public.audiobooks.len(), 1);
    assert_eq!(public.audiobooks[0].slug, "publish-book");
    assert_eq!(public.audiobooks[0].track_count, 1);
    assert_eq!(public.audiobooks[0].total_duration_seconds, 120);
    assert_eq!(
        public.audiobooks[0].translator.as_deref(),
        Some("Tran Slator")
    );
    assert_eq!(public.audiobooks[0].tags, vec!["Fiction".to_string()]);
    // The cover-less audiobook has no media URL to render.
    assert!(public.audiobooks[0].url.is_none());

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
    assert_eq!(filtered.audiobooks.len(), 1);

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
    assert!(no_match.audiobooks.is_empty());

    // The public detail feed resolves by slug and exposes playable track URLs.
    let details = fx
        .service
        .get_public_audiobook(GetPublicAudiobookCommand {
            slug: "publish-book".to_string(),
            tracks_offset: 0,
            tracks_limit: None,
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
            tracks_offset: 0,
            tracks_limit: None,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, AudiobookError::NotFound), "got {err:?}");
    fx.cleanup().await;
}

#[tokio::test]
async fn public_chapter_windows_page_a_book_without_loading_it_all() {
    let fx = fixture("windows").await;
    let book = fx.create("window-book", &[]).await;
    for index in 1..=5u8 {
        fx.add_track(book, &format!("Chapter {index}"), index, None)
            .await;
    }
    fx.service
        .change_audiobook_status(ChangeAudiobookStatusCommand {
            audiobook_id: book,
            user_id: 1,
            is_admin: false,
            status: "published".to_string(),
        })
        .await
        .unwrap();

    async fn window(fx: &Fixture, offset: i64, limit: Option<i64>) -> AudiobookDetails {
        fx.service
            .get_public_audiobook(GetPublicAudiobookCommand {
                slug: "window-book".to_string(),
                tracks_offset: offset,
                tracks_limit: limit,
            })
            .await
            .unwrap()
    }

    // A window in the middle carries the book's own chapter numbers, so the
    // player can line it up with the rows it has not fetched yet.
    let middle = window(&fx, 2, Some(2)).await;
    assert_eq!(middle.track_count, 5);
    assert_eq!(
        middle.tracks.iter().map(|t| t.number).collect::<Vec<_>>(),
        vec![3, 4]
    );
    assert!(middle.has_more_tracks);

    // The last window is short, and says the book ends there.
    let last = window(&fx, 4, Some(2)).await;
    assert_eq!(last.tracks.len(), 1);
    assert_eq!(last.tracks[0].number, 5);
    assert!(!last.has_more_tracks);

    // Past the end: nothing left, and no invitation to ask again — otherwise a
    // player paging on `has_more_tracks` would loop forever.
    let past = window(&fx, 9, Some(2)).await;
    assert!(past.tracks.is_empty());
    assert_eq!(past.track_count, 5);
    assert!(!past.has_more_tracks);

    // Asking for no window is still the whole playlist, so a caller that has
    // not been taught to page keeps working.
    let whole = window(&fx, 0, None).await;
    assert_eq!(whole.tracks.len(), 5);
    assert_eq!(whole.track_count, 5);
    assert!(!whole.has_more_tracks);

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
    assert!(
        matches!(err, AudiobookError::PermissionDenied),
        "got {err:?}"
    );

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
async fn deleting_an_audiobook_takes_its_chapter_audio_with_it() {
    let fx = fixture("delete").await;
    let book = fx.create("delete-book", &["Temp"]).await;
    let first = fx.add_track(book, "One", 1, None).await;
    let second = fx.add_track(book, "Two", 2, None).await;
    let (_, first_path) = track_medium(&fx, first).await;
    let (_, second_path) = track_medium(&fx, second).await;
    assert!(first_path.exists() && second_path.exists());

    fx.service
        .delete_audiobook(
            DeleteAudiobookCommand {
                audiobook_id: book,
                user_id: 1,
                is_admin: false,
            },
            &config(&fx),
        )
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

    // Tag vocabulary survives: it is reusable by the next book.
    let tags: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audiobook_tags")
        .fetch_one(&fx.service.pool)
        .await
        .unwrap();
    assert_eq!(tags, 1);

    // The audio does not: nothing plays it any more, and the bucket has no
    // other owner, so both the rows and the files go with the book.
    let media: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM media")
        .fetch_one(&fx.service.pool)
        .await
        .unwrap();
    assert_eq!(media, 0, "a deleted book leaves no media rows behind");
    assert!(
        !first_path.exists() && !second_path.exists(),
        "nor the audio files they named"
    );

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
    assert_eq!(alice.audiobooks.len(), 2);
    assert!(alice.audiobooks.iter().all(|a| a.slug.starts_with("alice")));

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
    assert_eq!(admin.audiobooks.len(), 3);

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
    assert_eq!(searched.audiobooks.len(), 1);
    assert_eq!(searched.audiobooks[0].slug, "bob-one");

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

    // Identical audio is encoded once too. The chunk objects hang off the same
    // content-addressed key as the plain file, so a second row naming the same
    // bytes adds no second file, no second set of chunks and no second
    // manifest — which is the property this test exists for.
    let entries: Vec<std::path::PathBuf> = std::fs::read_dir(&fx.media_dir)
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
                        .map(|e| e.path())
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        })
        .collect();

    let count_with_extension = |extension: &str| {
        entries
            .iter()
            .filter(|path| path.extension().and_then(|e| e.to_str()) == Some(extension))
            .count()
    };
    assert_eq!(
        count_with_extension("mp3"),
        1,
        "identical audio must be stored once"
    );
    assert_eq!(
        count_with_extension("zm"),
        1,
        "and carry exactly one chunk manifest"
    );
    assert_eq!(
        count_with_extension("d"),
        1,
        "and exactly one chunk directory"
    );

    fx.cleanup().await;
}

/// The chapter counter measures listening time, so every report counts and
/// nothing caps it: this is the regression test for the rule. A per-listener or
/// per-day dedup would fail it at the second report, which is exactly the
/// behaviour it exists to prevent from coming back.
#[tokio::test]
async fn play_reports_accumulate_without_a_cap() {
    let fx = fixture("plays").await;
    let book = fx.create("plays-book", &[]).await;
    let track = fx.add_track(book, "Chapter One", 1, None).await;

    // A draft is not counted, and the beacon stays a silent no-op rather than
    // erroring — it must not be usable to probe which tracks exist.
    assert_eq!(
        fx.service
            .record_track_play(RecordTrackPlayCommand { track_id: track })
            .await
            .unwrap(),
        None
    );
    assert_eq!(
        fx.service
            .record_track_play(RecordTrackPlayCommand { track_id: 999_999 })
            .await
            .unwrap(),
        None
    );

    fx.service
        .change_audiobook_status(ChangeAudiobookStatusCommand {
            audiobook_id: book,
            user_id: 1,
            is_admin: false,
            status: "published".to_string(),
        })
        .await
        .unwrap();

    // Ten reports are ten plays — a listener who stays in a chapter for a
    // hundred seconds. Each answer carries the new total, which is what the
    // player renders instead of assuming its own report moved the counter.
    for expected in 1..=10 {
        let counted = fx
            .service
            .record_track_play(RecordTrackPlayCommand { track_id: track })
            .await
            .unwrap();
        assert_eq!(counted, Some(expected), "report {expected} must count");
    }

    let details = fx
        .service
        .get_public_audiobook(GetPublicAudiobookCommand {
            slug: "plays-book".to_string(),
            tracks_offset: 0,
            tracks_limit: None,
        })
        .await
        .unwrap();
    assert_eq!(details.tracks[0].play_count, 10);

    fx.cleanup().await;
}

// ── Per-chapter modified time ────────────────────────────────────────────────
//
// `CURRENT_TIMESTAMP` has one-second resolution, so "did this change?" cannot be
// answered by comparing two stamps taken moments apart. Each test pins every
// row to a sentinel first and then asserts which sentinel moved — an answer that
// is the same whatever the clock does.

/// Stamp one chapter, so a later comparison can tell "untouched" from "bumped".
async fn pin_track_time(fx: &Fixture, track_id: i64, stamp: &str) {
    sqlx::query("UPDATE audiobook_tracks SET updated_at = ? WHERE id = ?")
        .bind(stamp)
        .bind(track_id)
        .execute(&fx.service.pool)
        .await
        .unwrap();
}

/// The stamps the read path reports, keyed by chapter title.
async fn track_times(fx: &Fixture, audiobook_id: i64) -> Vec<(String, Option<String>)> {
    let details = fx
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
        .map(|t| (t.title, t.updated_at))
        .collect()
}

#[tokio::test]
async fn a_new_chapter_reports_a_modified_time() {
    let fx = fixture("track-updated-at-new").await;
    let book = fx.create("time-new", &[]).await;
    fx.add_track(book, "Chapter One", 1, None).await;

    let times = track_times(&fx, book).await;
    assert_eq!(times.len(), 1);
    // The insert supplies it explicitly, since the column has no default to
    // fall back on.
    assert!(
        times[0].1.is_some(),
        "a freshly added chapter must carry a modified time"
    );

    fx.cleanup().await;
}

#[tokio::test]
async fn editing_a_chapter_stamps_only_that_chapter() {
    let fx = fixture("track-updated-at-edit").await;
    let book = fx.create("time-edit", &[]).await;
    let first = fx.add_track(book, "One", 1, None).await;
    let second = fx.add_track(book, "Two", 2, None).await;

    pin_track_time(&fx, first, "2000-01-01 00:00:00").await;
    pin_track_time(&fx, second, "2000-01-01 00:00:00").await;

    fx.service
        .update_track(UpdateTrackCommand {
            audiobook_id: book,
            track_id: second,
            user_id: 1,
            is_admin: false,
            title: Some("Two, revised".to_string()),
            number: None,
            duration_seconds: None,
        })
        .await
        .unwrap();

    let times = track_times(&fx, book).await;
    let stamp = |title: &str| {
        times
            .iter()
            .find(|(t, _)| t == title)
            .unwrap_or_else(|| panic!("missing chapter {title}"))
            .1
            .clone()
    };

    assert_ne!(
        stamp("Two, revised"),
        Some("2000-01-01 00:00:00".to_string()),
        "the edited chapter must be stamped"
    );
    assert_eq!(
        stamp("One"),
        Some("2000-01-01 00:00:00".to_string()),
        "an untouched neighbour must keep its own time"
    );

    fx.cleanup().await;
}

#[tokio::test]
async fn moving_a_chapter_does_not_restamp_the_others() {
    let fx = fixture("track-updated-at-move").await;
    let book = fx.create("time-drag", &[]).await;
    let a = fx.add_track(book, "A", 1, None).await;
    let b = fx.add_track(book, "B", 2, None).await;
    let c = fx.add_track(book, "C", 3, None).await;

    for (id, stamp) in [
        (a, "2000-01-01 00:00:00"),
        (b, "2000-01-02 00:00:00"),
        (c, "2000-01-03 00:00:00"),
    ] {
        pin_track_time(&fx, id, stamp).await;
    }

    // The single-track move path (`number` on update_track) is separate from
    // `reorder_tracks`, so it needs its own case: moving a chapter to another
    // slot is not an edit, and must leave every other chapter's time alone.
    fx.service
        .update_track(UpdateTrackCommand {
            audiobook_id: book,
            track_id: a,
            user_id: 1,
            is_admin: false,
            title: None,
            number: Some(4),
            duration_seconds: None,
        })
        .await
        .unwrap();

    let times = track_times(&fx, book).await;
    for (title, stamp) in [
        ("A", "2000-01-01 00:00:00"),
        ("B", "2000-01-02 00:00:00"),
        ("C", "2000-01-03 00:00:00"),
    ] {
        let found = times
            .iter()
            .find(|(t, _)| t == title)
            .unwrap_or_else(|| panic!("missing {title}"));
        assert_eq!(
            found.1,
            Some(stamp.to_string()),
            "a chapter that only moved is not a chapter that changed"
        );
    }

    fx.cleanup().await;
}

#[tokio::test]
async fn replacing_a_chapter_audio_stamps_only_that_chapter() {
    let fx = fixture("track-updated-at-replace").await;
    let book = fx.create("time-replace", &[]).await;
    let first = fx.add_track(book, "One", 1, None).await;
    let second = fx.add_track(book, "Two", 2, None).await;

    pin_track_time(&fx, first, "2000-01-01 00:00:00").await;
    pin_track_time(&fx, second, "2000-01-01 00:00:00").await;

    fx.service
        .replace_track_medium(
            ReplaceTrackMediumCommand {
                audiobook_id: book,
                track_id: first,
                user_id: 1,
                is_admin: false,
                title: None,
                duration_seconds: None,
                medium: audio("One-rerecorded.mp3", 9),
            },
            &config(&fx),
        )
        .await
        .unwrap();

    let times = track_times(&fx, book).await;
    let stamp = |title: &str| {
        times
            .iter()
            .find(|(t, _)| t == title)
            .unwrap_or_else(|| panic!("missing {title}"))
            .1
            .clone()
    };

    assert_ne!(
        stamp("One"),
        Some("2000-01-01 00:00:00".to_string()),
        "a replaced audio file is the clearest edit there is"
    );
    assert_eq!(
        stamp("Two"),
        Some("2000-01-01 00:00:00".to_string()),
        "replacing one chapter's audio must not touch another"
    );

    fx.cleanup().await;
}

#[tokio::test]
async fn listing_a_chapter_does_not_restamp_it() {
    let fx = fixture("track-updated-at-play").await;
    let book = fx.create("time-play", &[]).await;
    let track = fx.add_track(book, "Popular", 1, None).await;

    fx.service
        .change_audiobook_status(ChangeAudiobookStatusCommand {
            audiobook_id: book,
            user_id: 1,
            is_admin: false,
            status: "published".to_string(),
        })
        .await
        .unwrap();

    pin_track_time(&fx, track, "2000-01-01 00:00:00").await;

    for _ in 0..3 {
        fx.service
            .record_track_play(RecordTrackPlayCommand { track_id: track })
            .await
            .unwrap();
    }

    let times = track_times(&fx, book).await;
    assert_eq!(
        times[0].1,
        Some("2000-01-01 00:00:00".to_string()),
        "listening is not editing: a popular chapter must not read as a recently changed one"
    );

    fx.cleanup().await;
}

/// The media row behind a chapter, and where its bytes sit on disk.
async fn track_medium(fx: &Fixture, track_id: i64) -> (i64, PathBuf) {
    let (media_id, hash, file_type, uploader_id): (i64, String, String, i64) = sqlx::query_as(
        "SELECT m.id, m.hash, m.file_type, m.uploader_id
         FROM audiobook_tracks t JOIN media m ON m.id = t.media_id
         WHERE t.id = ?",
    )
    .bind(track_id)
    .fetch_one(&fx.service.pool)
    .await
    .unwrap();

    let key = media_key(&hash, &file_type, uploader_id).expect("a chapter's audio has a key");
    (media_id, fx.media_dir.join(&key.key))
}

#[tokio::test]
async fn a_chapter_cannot_take_a_slot_another_chapter_holds() {
    let fx = fixture("slot-taken").await;
    let book = fx.create("slot-book", &[]).await;
    fx.add_track(book, "One", 1, None).await;

    let rows_before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM media")
        .fetch_one(&fx.service.pool)
        .await
        .unwrap();

    let err = fx
        .service
        .add_track(
            AddTrackCommand {
                audiobook_id: book,
                user_id: 1,
                is_admin: false,
                title: "Imposter".to_string(),
                number: Some(1),
                duration_seconds: Some(120),
                medium: audio("Imposter.mp3", 3),
            },
            &config(&fx),
        )
        .await
        .unwrap_err();
    assert!(matches!(err, AudiobookError::Validation(_)), "got {err:?}");

    assert_eq!(
        fx.track_numbers(book).await,
        vec![("One".to_string(), 1)],
        "a rejected slot must not have added a chapter"
    );

    // The slot is checked before the upload is stored, so a rejected one leaves
    // no file behind for the backfill to trip over later.
    let rows_after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM media")
        .fetch_one(&fx.service.pool)
        .await
        .unwrap();
    assert_eq!(
        rows_before, rows_after,
        "a rejected slot must not leave an upload behind"
    );

    fx.cleanup().await;
}

#[tokio::test]
async fn replacing_a_chapter_removes_the_audio_it_replaced() {
    let fx = fixture("replace-cleans").await;
    let book = fx.create("clean-book", &[]).await;
    let first = fx.add_track(book, "One", 1, None).await;
    fx.add_track(book, "Two", 2, None).await;

    let (old_media_id, old_path) = track_medium(&fx, first).await;
    assert!(old_path.exists(), "the original upload starts on disk");

    // The chunked form of that same upload, addressed off the medium's own
    // key. Its presence here is the write path's side of the contract: an
    // upload that produced no manifest would leave every read on the plain
    // file and quietly never chunk anything.
    let old_key = old_path
        .strip_prefix(&fx.media_dir)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let old_manifest = fx
        .media_dir
        .join(backend::infrastructure::storage::manifest_key(&old_key));
    let old_chunk_dir = fx
        .media_dir
        .join(backend::infrastructure::storage::chunk_dir(&old_key));
    assert!(
        old_manifest.is_file(),
        "an upload writes its chunk manifest"
    );
    assert!(old_chunk_dir.is_dir(), "and the chunks the manifest names");

    fx.service
        .replace_track_medium(
            ReplaceTrackMediumCommand {
                audiobook_id: book,
                track_id: first,
                user_id: 1,
                is_admin: false,
                title: None,
                duration_seconds: None,
                medium: audio("One-rerecorded.mp3", 9),
            },
            &config(&fx),
        )
        .await
        .unwrap();

    assert!(
        !old_path.exists(),
        "the audio a chapter no longer plays must not survive the swap"
    );
    assert!(
        !old_manifest.exists(),
        "nor its manifest, or the chunks it named would be unreachable"
    );
    assert!(
        !old_chunk_dir.exists(),
        "nor the chunks themselves: the swap releases every copy of the old take"
    );

    let old_rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM media WHERE id = ?")
        .bind(old_media_id)
        .fetch_one(&fx.service.pool)
        .await
        .unwrap();
    assert_eq!(old_rows, 0, "the replaced media row goes with its file");

    // The chapter itself is untouched and plays the new take.
    let (new_media_id, new_path) = track_medium(&fx, first).await;
    assert_ne!(new_media_id, old_media_id);
    assert!(
        new_path.exists(),
        "the replacement is what the chapter plays now"
    );

    fx.cleanup().await;
}

#[tokio::test]
async fn replacing_keeps_the_file_another_chapter_still_plays() {
    let fx = fixture("replace-shared").await;
    let book = fx.create("shared-book", &[]).await;
    // The same bytes twice: content addressing gives both chapters one file.
    let first = fx.add_track(book, "One", 7, None).await;
    let second = fx.add_track(book, "Two", 7, None).await;

    let (first_media, shared_path) = track_medium(&fx, first).await;
    let (second_media, second_path) = track_medium(&fx, second).await;
    assert_ne!(first_media, second_media, "each chapter gets its own row");
    assert_eq!(shared_path, second_path, "but the same bytes are one file");

    fx.service
        .replace_track_medium(
            ReplaceTrackMediumCommand {
                audiobook_id: book,
                track_id: first,
                user_id: 1,
                is_admin: false,
                title: None,
                duration_seconds: None,
                medium: audio("One-rerecorded.mp3", 9),
            },
            &config(&fx),
        )
        .await
        .unwrap();

    assert!(
        second_path.exists(),
        "the other chapter is still playing those bytes"
    );

    // Which is only meaningful if that chapter can still reach its audio.
    let (second_media_after, _) = track_medium(&fx, second).await;
    assert_eq!(second_media_after, second_media);

    fx.cleanup().await;
}

#[tokio::test]
async fn removing_a_chapter_takes_its_audio_with_it() {
    let fx = fixture("remove-cleans").await;
    let book = fx.create("remove-book", &[]).await;
    let first = fx.add_track(book, "One", 1, None).await;
    let second = fx.add_track(book, "Two", 2, None).await;

    let (gone_media, gone_path) = track_medium(&fx, first).await;
    let (kept_media, kept_path) = track_medium(&fx, second).await;

    let gone_key = gone_path
        .strip_prefix(&fx.media_dir)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let gone_chunk_dir = fx
        .media_dir
        .join(backend::infrastructure::storage::chunk_dir(&gone_key));
    assert!(gone_chunk_dir.is_dir(), "the upload chunked itself");

    fx.service
        .remove_track(
            RemoveTrackCommand {
                audiobook_id: book,
                track_id: first,
                user_id: 1,
                is_admin: false,
            },
            &config(&fx),
        )
        .await
        .unwrap();

    assert!(
        !gone_path.exists(),
        "nothing plays that audio once its only chapter is gone"
    );
    assert!(
        !gone_chunk_dir.exists(),
        "its disk chunks must also be removed"
    );
    assert!(!fx.media_dir.join(manifest_key(&gone_key)).exists());

    let gone_rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM media WHERE id = ?")
        .bind(gone_media)
        .fetch_one(&fx.service.pool)
        .await
        .unwrap();
    assert_eq!(gone_rows, 0, "the media row goes with the files");

    // The chapter that stayed is untouched.
    let (kept_media_after, _) = track_medium(&fx, second).await;
    assert_eq!(kept_media_after, kept_media);
    assert!(kept_path.exists());

    fx.cleanup().await;
}

#[tokio::test]
async fn removing_a_chapter_keeps_the_file_another_chapter_still_plays() {
    let fx = fixture("remove-shared").await;
    let book = fx.create("remove-shared-book", &[]).await;
    // The same bytes twice: two rows, one file.
    let first = fx.add_track(book, "One", 5, None).await;
    let second = fx.add_track(book, "Two", 5, None).await;

    let (first_media, shared_path) = track_medium(&fx, first).await;
    let (_, second_path) = track_medium(&fx, second).await;
    assert_eq!(shared_path, second_path);

    fx.service
        .remove_track(
            RemoveTrackCommand {
                audiobook_id: book,
                track_id: first,
                user_id: 1,
                is_admin: false,
            },
            &config(&fx),
        )
        .await
        .unwrap();

    assert!(
        shared_path.exists(),
        "the remaining chapter still plays those bytes"
    );
    let first_rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM media WHERE id = ?")
        .bind(first_media)
        .fetch_one(&fx.service.pool)
        .await
        .unwrap();
    assert_eq!(first_rows, 0, "only the row of the removed chapter goes");

    fx.cleanup().await;
}

#[tokio::test]
async fn media_reference_query_covers_every_foreign_key() {
    let fx = fixture("reference-schema").await;
    let foreign_keys: Vec<(String, String)> = sqlx::query_as(
        "SELECT s.name, f.\"from\" FROM sqlite_schema s
         JOIN pragma_foreign_key_list(s.name) f
         WHERE s.type = 'table' AND f.\"table\" = 'media'
         ORDER BY s.name, f.\"from\"",
    )
    .fetch_all(&fx.service.pool)
    .await
    .unwrap();
    let mut covered: Vec<(String, String)> = MEDIA_REFERENCES
        .split("UNION ALL")
        .map(|branch| {
            let words: Vec<_> = branch.split_whitespace().collect();
            match words.as_slice() {
                ["SELECT", column, "FROM", table] => (table.to_string(), column.to_string()),
                _ => panic!("unexpected reference query branch: {branch}"),
            }
        })
        .collect();
    covered.sort();
    assert_eq!(
        covered, foreign_keys,
        "every media FK must protect ownership"
    );
    fx.cleanup().await;
}

#[tokio::test]
async fn removing_and_replacing_preserve_every_kind_of_media_reference() {
    for (name, reference) in [
        (
            "alias",
            "INSERT INTO media_aliases (media_id, alias) VALUES (?, 'kept')",
        ),
        (
            "post-use",
            "INSERT INTO post_media_usages (post_id, medium_id, code) VALUES (1, ?, 1)",
        ),
        (
            "post-cover",
            "UPDATE posts SET cover_media_id = ? WHERE id = 1",
        ),
        (
            "series-cover",
            "INSERT INTO series (user_id, title, slug, cover_image_id) VALUES (1, 'Series', 'series', ?)",
        ),
        (
            "book-cover",
            "UPDATE audiobooks SET cover_image_id = ? WHERE slug = 'holder'",
        ),
        (
            "avatar",
            "INSERT INTO user_meta (user_id, display_name, avatar_image_id) VALUES (1, 'Alice', ?)",
        ),
        (
            "track",
            "INSERT INTO audiobook_tracks (audiobook_id, media_id, title, number) VALUES ((SELECT id FROM audiobooks WHERE slug = 'holder'), ?, 'Kept', 1)",
        ),
    ] {
        for replace in [false, true] {
            let fx = fixture(&format!("reference-{name}-{replace}")).await;
            let book = fx.create("source", &[]).await;
            fx.create("holder", &[]).await;
            sqlx::query(
                "INSERT INTO posts (id, user_id, title, slug) VALUES (1, 1, 'Post', 'post')",
            )
            .execute(&fx.service.pool)
            .await
            .unwrap();
            let track = fx.add_track(book, "Original", 7, None).await;
            let (media_id, path) = track_medium(&fx, track).await;
            sqlx::query(reference)
                .bind(media_id)
                .execute(&fx.service.pool)
                .await
                .unwrap();

            if replace {
                fx.service
                    .replace_track_medium(
                        ReplaceTrackMediumCommand {
                            audiobook_id: book,
                            track_id: track,
                            user_id: 1,
                            is_admin: false,
                            title: None,
                            duration_seconds: None,
                            medium: audio("replacement.mp3", 9),
                        },
                        &config(&fx),
                    )
                    .await
                    .unwrap();
                assert_ne!(track_medium(&fx, track).await.0, media_id);
            } else {
                fx.service
                    .remove_track(
                        RemoveTrackCommand {
                            audiobook_id: book,
                            track_id: track,
                            user_id: 1,
                            is_admin: false,
                        },
                        &config(&fx),
                    )
                    .await
                    .unwrap();
            }

            let retained: bool =
                sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM media WHERE id = ?)")
                    .bind(media_id)
                    .fetch_one(&fx.service.pool)
                    .await
                    .unwrap();
            assert!(
                retained,
                "{name} must retain the media row after replace={replace}"
            );
            let references: i64 = sqlx::query_scalar(&format!(
                "SELECT COUNT(*) FROM ({MEDIA_REFERENCES}) WHERE media_id = ?"
            ))
            .bind(media_id)
            .fetch_one(&fx.service.pool)
            .await
            .unwrap();
            assert_eq!(references, 1, "the {name} reference must survive");
            assert_eq!(
                std::fs::read(&path).unwrap(),
                audio("original.mp3", 7).bytes
            );
            let key = path.strip_prefix(&fx.media_dir).unwrap().to_str().unwrap();
            assert!(fx.media_dir.join(manifest_key(key)).is_file());
            assert!(fx.media_dir.join(chunk_key(key, 0)).is_file());
            fx.cleanup().await;
        }
    }
}

#[tokio::test]
async fn deleting_a_book_preserves_audio_shared_with_another_book() {
    for same_row in [false, true] {
        let fx = fixture(&format!("delete-cross-book-{same_row}")).await;
        let gone = fx.create("gone", &[]).await;
        let kept = fx.create("kept", &[]).await;
        let first = fx.add_track(gone, "Gone", 7, None).await;
        let second = fx.add_track(kept, "Kept", 7, None).await;
        let (first_media, path) = track_medium(&fx, first).await;
        let (second_media, second_path) = track_medium(&fx, second).await;
        assert_eq!(path, second_path);
        if same_row {
            sqlx::query("UPDATE audiobook_tracks SET media_id = ? WHERE id = ?")
                .bind(first_media)
                .bind(second)
                .execute(&fx.service.pool)
                .await
                .unwrap();
            sqlx::query("DELETE FROM media WHERE id = ?")
                .bind(second_media)
                .execute(&fx.service.pool)
                .await
                .unwrap();
        }

        fx.service
            .delete_audiobook(
                DeleteAudiobookCommand {
                    audiobook_id: gone,
                    user_id: 1,
                    is_admin: false,
                },
                &config(&fx),
            )
            .await
            .unwrap();

        assert_eq!(
            track_medium(&fx, second).await.0,
            if same_row { first_media } else { second_media }
        );
        assert_eq!(std::fs::read(&path).unwrap(), audio("same.mp3", 7).bytes);
        let key = path.strip_prefix(&fx.media_dir).unwrap().to_str().unwrap();
        assert!(fx.media_dir.join(manifest_key(key)).is_file());
        assert!(fx.media_dir.join(chunk_dir(key)).is_dir());
        assert!(fx.media_dir.join(chunk_key(key, 0)).is_file());
        let remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM media")
            .fetch_one(&fx.service.pool)
            .await
            .unwrap();
        assert_eq!(remaining, 1);
        fx.cleanup().await;
    }
}

#[tokio::test]
async fn shared_path_writers_lock_before_touching_disk() {
    let fx = fixture("writer-lock").await;
    let book = fx.create("locked", &[]).await;
    let track = fx.add_track(book, "Original", 1, None).await;
    let writer_pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(&fx.db_path)
                .busy_timeout(std::time::Duration::from_millis(100)),
        )
        .await
        .unwrap();
    let media_service = MediaServiceImpl::new(writer_pool.clone());
    let audiobook_service = AudiobookServiceImpl::new(writer_pool.clone());

    for (kind, seed) in [("single", 70), ("bulk", 71), ("add", 72), ("replace", 73)] {
        let cfg = config(&fx);
        let medium = audio("locked.mp3", seed);
        let hashed = hash_bytes(&medium.bytes, &cfg.dir, ".mp3".to_string(), true)
            .await
            .unwrap();
        assert!(!hashed.dir_path.exists());
        let write = || async {
            match kind {
                "single" => media_service
                    .upload(
                        UploadMediumCommand {
                            uploader_id: 1,
                            short_name: "locked-single".to_string(),
                            description: String::new(),
                            file_name: medium.filename.clone(),
                            content_type: medium.content_type.clone(),
                            bytes: medium.bytes.clone(),
                        },
                        &cfg,
                    )
                    .await
                    .map_err(|e| format!("{e:?}")),
                "bulk" => media_service
                    .bulk_upload(
                        UploadMediaWithoutDescriptionCommand {
                            uploader_id: 1,
                            number_of_files: 1,
                            short_names: vec!["locked-bulk".to_string()],
                            file_names: vec![medium.filename.clone()],
                            content_types: vec![medium.content_type.clone()],
                            bytes_list: vec![medium.bytes.clone()],
                        },
                        &cfg,
                    )
                    .await
                    .map_err(|e| format!("{e:?}")),
                "add" => audiobook_service
                    .add_track(
                        AddTrackCommand {
                            audiobook_id: book,
                            user_id: 1,
                            is_admin: false,
                            title: "Locked".to_string(),
                            number: None,
                            duration_seconds: None,
                            medium: medium.clone(),
                        },
                        &cfg,
                    )
                    .await
                    .map(|_| ())
                    .map_err(|e| format!("{e:?}")),
                "replace" => audiobook_service
                    .replace_track_medium(
                        ReplaceTrackMediumCommand {
                            audiobook_id: book,
                            track_id: track,
                            user_id: 1,
                            is_admin: false,
                            title: None,
                            duration_seconds: None,
                            medium: medium.clone(),
                        },
                        &cfg,
                    )
                    .await
                    .map_err(|e| format!("{e:?}")),
                _ => unreachable!(),
            }
        };
        let guard = fx.service.pool.begin_with("BEGIN IMMEDIATE").await.unwrap();
        let result = tokio::time::timeout(std::time::Duration::from_secs(5), write())
            .await
            .unwrap();
        assert!(
            result.is_err(),
            "{kind} must not acquire the held write lock"
        );
        assert!(!hashed.file_path.exists(), "{kind} wrote before locking");
        // Directories survive error cleanup, exposing even a write that was immediately undone.
        assert!(
            !hashed.dir_path.exists(),
            "{kind} touched disk before locking"
        );
        guard.rollback().await.unwrap();
        write().await.unwrap();
        assert_eq!(std::fs::read(&hashed.file_path).unwrap(), medium.bytes);
    }
    writer_pool.close().await;
    fx.cleanup().await;
}

#[tokio::test]
async fn prune_command_rechecks_approval_and_holds_lock_through_verification() {
    use axum::{Router, extract::Request, routing::any};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use tokio::sync::{mpsc, oneshot};

    for mode in [
        "success",
        "stale-alias",
        "stale-shared",
        "partial",
        "verification",
    ] {
        let fx = fixture(&format!("prune-command-{mode}")).await;
        let book = fx.create("prune-book", &[]).await;
        let track = fx.add_track(book, "Original", 7, None).await;
        let (media_id, path) = track_medium(&fx, track).await;
        let key = path.strip_prefix(&fx.media_dir).unwrap().to_str().unwrap();
        let mut files = Vec::new();
        let mut selected = Vec::new();
        for key in [key.to_string(), manifest_key(key), chunk_key(key, 0)] {
            let path = fx.media_dir.join(&key);
            files.push((path.clone(), std::fs::read(path).unwrap()));
            selected.push(format!("audio/{key}"));
        }
        selected.sort();
        sqlx::query("DELETE FROM audiobook_tracks WHERE id = ?")
            .bind(track)
            .execute(&fx.service.pool)
            .await
            .unwrap();
        let objects = Arc::new(Mutex::new(selected.clone()));
        objects
            .lock()
            .unwrap()
            .push("audio/manual-review.mp3".into());
        let submitted = Arc::new(Mutex::new(Vec::<String>::new()));
        let (stage_tx, mut stage_rx) = mpsc::unbounded_channel::<(&str, oneshot::Sender<()>)>();
        let app = Router::new().route("/{*path}", any({
            let objects = objects.clone();
            let submitted = submitted.clone();
            move |request: Request| {
                let objects = objects.clone();
                let submitted = submitted.clone();
                let stage_tx = stage_tx.clone();
                async move {
                    assert_eq!(request.uri().path().trim_end_matches('/'), "/bucket");
                    let deleting = request.method() == http::Method::POST;
                    let body = axum::body::to_bytes(request.into_body(), 1024 * 1024).await.unwrap();
                    let body = String::from_utf8(body.to_vec()).unwrap();
                    let verification = !deleting && !submitted.lock().unwrap().is_empty();
                    if deleting || verification {
                        let (resume_tx, resume_rx) = oneshot::channel();
                        stage_tx.send((if deleting { "delete" } else { "verify" }, resume_tx)).unwrap();
                        resume_rx.await.unwrap();
                    }
                    let response = if deleting {
                        let keys: Vec<_> = body.split("<Key>").skip(1)
                            .map(|part| part.split("</Key>").next().unwrap().to_string()).collect();
                        submitted.lock().unwrap().extend(keys.clone());
                        let mut objects = objects.lock().unwrap();
                        let mut results = String::new();
                        for (index, key) in keys.iter().enumerate() {
                            if mode == "partial" && index + 1 == keys.len() {
                                results.push_str(&format!("<Error><Key>{key}</Key><Code>AccessDenied</Code></Error>"));
                            } else {
                                objects.retain(|object| object != key);
                                results.push_str(&format!("<Deleted><Key>{key}</Key></Deleted>"));
                            }
                        }
                        format!("<DeleteResult>{results}</DeleteResult>")
                    } else {
                        let mut objects = objects.lock().unwrap();
                        if verification && mode == "verification" {
                            objects.push("audio/unexpected.mp3".into());
                        }
                        let contents: String = objects.iter().map(|key| format!(
                            "<Contents><Key>{key}</Key><Size>64</Size><ETag>test-etag</ETag><LastModified>2020-01-01T00:00:00Z</LastModified></Contents>"
                        )).collect();
                        format!("<ListBucketResult><IsTruncated>false</IsTruncated>{contents}</ListBucketResult>")
                    };
                    ([(http::header::CONTENT_TYPE, "application/xml")], response)
                }
            }
        }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let command = |fingerprint: Option<&str>| {
            let mut command = tokio::process::Command::new(env!("CARGO_BIN_EXE_backfill-audio"));
            command
                .env_clear()
                .current_dir(&fx.media_dir)
                .env("DATABASE_URL", format!("sqlite://{}", fx.db_path.display()))
                .env("MEDIA_PATH", &fx.media_dir)
                .env("R2_ACCOUNT_ID", "test")
                .env("R2_ACCESS_KEY_ID", "test-key")
                .env("R2_SECRET_ACCESS_KEY", "test-secret")
                .env("R2_BUCKET", "bucket")
                .env("R2_ENDPOINT", &endpoint)
                .kill_on_drop(true);
            if let Some(fingerprint) = fingerprint {
                command.args(["--prune", fingerprint]);
            }
            command
        };
        let dry = command(None).output().await.unwrap();
        assert!(
            dry.status.success(),
            "{}",
            String::from_utf8_lossy(&dry.stderr)
        );
        assert!(submitted.lock().unwrap().is_empty());
        assert_eq!(objects.lock().unwrap().len(), selected.len() + 1);
        let report = String::from_utf8(dry.stdout).unwrap();
        let approval = report
            .lines()
            .find_map(|line| line.strip_prefix("Fingerprint: "))
            .unwrap();
        let planned: Vec<_> = report
            .lines()
            .filter(|line| line.starts_with('{'))
            .map(|line| {
                serde_json::from_str::<serde_json::Value>(line).unwrap()["key"]
                    .as_str()
                    .unwrap()
                    .to_owned()
            })
            .collect();
        assert_eq!(planned, selected);
        if mode == "stale-alias" {
            sqlx::query("INSERT INTO media_aliases (media_id, alias) VALUES (?, 'keep')")
                .bind(media_id)
                .execute(&fx.service.pool)
                .await
                .unwrap();
        } else if mode == "stale-shared" {
            sqlx::query("INSERT INTO media (hash, short_name, file_name, file_type, url, size, uploader_id)
                SELECT hash, 'library-copy', file_name, file_type, url, size, uploader_id FROM media WHERE id = ?")
                .bind(media_id).execute(&fx.service.pool).await.unwrap();
        }
        let mut apply_command = command(Some(approval));
        let applying = tokio::spawn(async move { apply_command.output().await.unwrap() });
        if !mode.starts_with("stale-") {
            let writer_pool = SqlitePoolOptions::new()
                .max_connections(1)
                .connect_with(
                    SqliteConnectOptions::new()
                        .filename(&fx.db_path)
                        .busy_timeout(Duration::from_millis(100)),
                )
                .await
                .unwrap();
            for expected in ["delete", "verify"] {
                let (stage, resume) =
                    tokio::time::timeout(Duration::from_secs(10), stage_rx.recv())
                        .await
                        .unwrap()
                        .unwrap();
                assert_eq!(stage, expected);
                assert!(
                    writer_pool.begin_with("BEGIN IMMEDIATE").await.is_err(),
                    "prune released the write lock during {stage}"
                );
                resume.send(()).unwrap();
            }
            let output = tokio::time::timeout(Duration::from_secs(10), applying)
                .await
                .unwrap()
                .unwrap();
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert_eq!(
                output.status.success(),
                mode == "success",
                "{mode}: {stderr}"
            );
            if mode == "partial" {
                assert!(stderr.contains("possibly partial"));
                assert!(objects.lock().unwrap().contains(selected.last().unwrap()));
            } else if mode == "verification" {
                assert!(stderr.contains("verification failed"));
            }
            assert_eq!(*submitted.lock().unwrap(), selected);
            assert!(
                objects
                    .lock()
                    .unwrap()
                    .contains(&"audio/manual-review.mp3".into())
            );
            writer_pool
                .begin_with("BEGIN IMMEDIATE")
                .await
                .unwrap()
                .rollback()
                .await
                .unwrap();
            writer_pool.close().await;
        } else {
            let output = tokio::time::timeout(Duration::from_secs(10), applying)
                .await
                .unwrap()
                .unwrap();
            assert!(!output.status.success());
            assert!(String::from_utf8_lossy(&output.stderr).contains("fingerprint changed"));
            assert!(submitted.lock().unwrap().is_empty());
            assert_eq!(objects.lock().unwrap().len(), selected.len() + 1);
        }
        let row_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM media")
            .fetch_one(&fx.service.pool)
            .await
            .unwrap();
        assert_eq!(row_count, if mode == "stale-shared" { 2 } else { 1 });
        for (path, bytes) in files {
            assert_eq!(std::fs::read(path).unwrap(), bytes);
        }
        server.abort();
        fx.cleanup().await;
    }
}
