// Media storage locations and accepted upload types.
use std::env;
use std::path::PathBuf;

use crate::domain::entities::media::MediaType;

pub struct MediaConfig {
    pub dir: PathBuf,
    pub allowed_file_types: Vec<MediaType>,
    pub allowed_avatar_types: Vec<MediaType>,
    pub allowed_cover_types: Vec<MediaType>,
    /// Accepted audio containers for audiobook tracks. Kept separate from
    /// `allowed_file_types` so track uploads cannot smuggle in an image or a
    /// video by mislabelling the content type.
    pub allowed_audio_types: Vec<MediaType>,
}

impl MediaConfig {
    pub fn from_env() -> Self {
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

        Self {
            dir,
            allowed_file_types,
            allowed_avatar_types,
            allowed_cover_types,
            allowed_audio_types,
        }
    }
}

