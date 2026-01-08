use std::str::FromStr;

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
}

impl MediaType {
    // pub fn as_str(&self) -> &'static str {
    //     match self {
    //         MediaType::ImagePng => "image/png",
    //         MediaType::ImageJpeg => "image/jpeg",
    //         MediaType::ImageGif => "image/gif",
    //         MediaType::ImageWebp => "image/webp",
    //         MediaType::VideoMp4 => "video/mp4",
    //         MediaType::VideoWebm => "video/webm",
    //     }
    // }
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
