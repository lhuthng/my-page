use std::str::FromStr;

use axum::body::Bytes;

use crate::domain::errors::media::MediaError;

#[derive(Debug, PartialEq)]
pub enum MediaType {
    ImagePng,
    ImageJpeg,
    ImageGif,
    ImageWebp,
    VideoMp4,
    VideoWebm,
    AudioMp3,
    AudioOgg,
    AudioWav,
    ModelGlb,
}

impl MediaType {
    pub fn get_extension(&self) -> &'static str {
        match self {
            MediaType::ImagePng => ".png",
            MediaType::ImageJpeg => ".jpeg",
            MediaType::ImageGif => ".gif",
            MediaType::ImageWebp => ".webp",
            MediaType::VideoMp4 => ".mp4",
            MediaType::VideoWebm => ".webm",
            MediaType::AudioMp3 => ".mp3",
            MediaType::AudioOgg => ".ogg",
            MediaType::AudioWav => ".wav",
            MediaType::ModelGlb => ".glb",
        }
    }
}

impl FromStr for MediaType {
    type Err = MediaError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "image/png" => Ok(Self::ImagePng),
            "image/jpeg" => Ok(Self::ImageJpeg),
            "image/gif" => Ok(Self::ImageGif),
            "image/webp" => Ok(Self::ImageWebp),
            "video/mp4" => Ok(Self::VideoMp4),
            "video/webm" => Ok(Self::VideoWebm),
            "audio/mpeg" => Ok(Self::AudioMp3),
            "audio/ogg" => Ok(Self::AudioOgg),
            "audio/wav" => Ok(Self::AudioWav),
            "model/gltf-binary" => Ok(Self::ModelGlb),
            _ => Err(MediaError::InvalidFileType),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LinkResult {
    pub short_name: Option<String>,
    pub url: String,
    pub file_type: String,
}

#[derive(Debug, Clone)]
pub struct MediaDetailResult {
    pub short_name: String,
    pub file_type: String,
    pub description: String,
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct MediumDetails {
    pub filename: String,
    pub content_type: String,
    pub bytes: Bytes,
}
