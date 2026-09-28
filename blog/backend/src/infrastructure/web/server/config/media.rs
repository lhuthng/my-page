// Media storage locations, accepted upload types, and where the bytes are
// served from.
//
// Note the scope of the bucket: **only audio is stored there.** Everything else
// — images, covers, avatars, video, models — stays on `MEDIA_PATH`, which is why
// the switch and the types below are named for audio rather than for media.
use std::env;
use std::path::PathBuf;

use crate::domain::entities::media::MediaType;
use crate::infrastructure::storage::{S3Settings, r2::R2Client};

/// How a request for audio is answered when the bytes are in the bucket.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AudioReadMode {
    /// Answer with a 302 to the object's public URL, so the bytes never pass
    /// through this process. The default.
    Redirect,
    /// Stream the bytes through the backend. Slower — every byte crosses the
    /// VM — but it needs nothing of the client, which makes it the fallback for
    /// debugging and for a player that mishandles redirects.
    Proxy,
}

/// The bucket audio lives in, plus the serving policy that goes with it.
///
/// One bucket, shared with the v86 artifacts: audio is kept apart by the
/// `audio/` prefix rather than by a bucket of its own. That is enough here
/// because R2 access tokens are scoped to buckets and never to prefixes, so a
/// second bucket's only extra power would be separate credentials — which
/// nothing in this system needs, since one process writes both — while R2
/// lifecycle rules *do* take a prefix, which is the part worth having.
pub struct AudioStore {
    /// Used to write, and to read in `Proxy` mode. `Redirect` never touches it.
    pub client: R2Client,
    /// Public base URL of the bucket, e.g. `https://disk.huuthangle.site`.
    /// Objects are served from here, which is why nothing is signed.
    pub public_base_url: String,
    pub read_mode: AudioReadMode,
}

impl AudioStore {
    /// The public URL of an object, from its bucket object key — the value of
    /// [`audio_object_key`](crate::infrastructure::storage::audio_object_key).
    ///
    /// This type knows about URLs and nothing about the layout, so the one
    /// place that decides where an object sits stays
    /// [`audio_object_key`](crate::infrastructure::storage::audio_object_key).
    pub fn public_url(&self, object_key: &str) -> String {
        format!(
            "{}/{}",
            self.public_base_url.trim_end_matches('/'),
            object_key
        )
    }
}

pub struct MediaConfig {
    pub dir: PathBuf,
    pub allowed_file_types: Vec<MediaType>,
    pub allowed_avatar_types: Vec<MediaType>,
    pub allowed_cover_types: Vec<MediaType>,
    /// Accepted audio containers for audiobook tracks. Kept separate from
    /// `allowed_file_types` so track uploads cannot smuggle in an image or a
    /// video by mislabelling the content type.
    pub allowed_audio_types: Vec<MediaType>,
    /// Where audio is stored, when it is not on the disk. `None` is the default
    /// and the behavior before the migration. Non-audio media never uses this —
    /// it has no object key at all, see `storage::audio_object_key`.
    pub audio_bucket: Option<AudioStore>,
}

impl MediaConfig {
    pub fn from_env() -> Result<Self, String> {
        let media_path = env::var("MEDIA_PATH").expect("MEDIA_PATH must be set");

        let dir = PathBuf::from(&media_path);
        if !dir.exists() {
            std::fs::create_dir_all(&dir).expect("Failed to create media directory");
        }

        let allowed_file_types = vec![
            MediaType::ImagePng,
            MediaType::ImageGif,
            MediaType::ImageWebp,
            MediaType::ImageJpeg,
            MediaType::VideoMp4,
            MediaType::VideoWebm,
            MediaType::AudioMp3,
            MediaType::AudioOgg,
            MediaType::AudioMp3,
            MediaType::ModelGlb,
            MediaType::Lottie,
        ];

        let allowed_avatar_types = vec![
            MediaType::ImagePng,
            MediaType::ImageGif,
            MediaType::ImageWebp,
            MediaType::ImageJpeg,
        ];

        let allowed_cover_types = vec![
            MediaType::ImagePng,
            MediaType::ImageGif,
            MediaType::ImageWebp,
            MediaType::ImageJpeg,
            MediaType::VideoMp4,
            MediaType::VideoWebm,
        ];

        let allowed_audio_types = vec![
            MediaType::AudioMp3,
            MediaType::AudioOgg,
            MediaType::AudioWav,
        ];

        // `fs` or `r2`, matching `STORAGE_BACKEND`'s vocabulary. There is
        // deliberately no `auto`: `STORAGE_BACKEND=auto` may pick R2 as a side
        // effect of the credentials existing, and audio moving to the bucket is
        // a decision rather than something that should happen because someone
        // configured R2 for the artifacts.
        //
        // Only audio is affected by this — see the module comment.
        let audio_bucket = match env::var("AUDIO_BACKEND")
            .unwrap_or_else(|_| "fs".to_string())
            .as_str()
        {
            "fs" => None,
            "r2" => {
                let settings = S3Settings::require_r2_env()
                    .map_err(|missing| format!("AUDIO_BACKEND=r2 but {missing}"))?;
                // The bucket is public and that is how audio is served, so
                // without the public URL there is no way to answer a request.
                // Failing at startup beats a 500 on the first play.
                let public_base_url = env::var("R2_PUBLIC_URL").map_err(|_| {
                    "AUDIO_BACKEND=r2 needs R2_PUBLIC_URL: audio is served from the \
                     bucket's public domain"
                        .to_string()
                })?;

                // Printed rather than logged, like the storage-backend line, so
                // it is readable in `docker compose logs` at startup.
                println!(
                    "Audio backend: r2 (bucket {}, prefix {}/, public {})",
                    settings.bucket,
                    crate::infrastructure::storage::AUDIO_OBJECT_PREFIX,
                    public_base_url.trim_end_matches('/')
                );

                Some(AudioStore {
                    client: R2Client::from_settings(settings),
                    public_base_url,
                    read_mode: match env::var("AUDIO_READ")
                        .unwrap_or_else(|_| "redirect".to_string())
                        .as_str()
                    {
                        "redirect" => AudioReadMode::Redirect,
                        "proxy" => AudioReadMode::Proxy,
                        other => {
                            return Err(format!(
                                "Unsupported AUDIO_READ '{other}' (expected redirect or proxy)"
                            ));
                        }
                    },
                })
            }
            other => {
                return Err(format!(
                    "Unsupported AUDIO_BACKEND '{other}' (expected fs or r2)"
                ));
            }
        };

        Ok(Self {
            dir,
            allowed_file_types,
            allowed_avatar_types,
            allowed_cover_types,
            allowed_audio_types,
            audio_bucket,
        })
    }
}
