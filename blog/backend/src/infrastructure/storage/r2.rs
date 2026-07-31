use std::{env, path::Path};

use aws_sdk_s3::{
    Client, config::{BehaviorVersion, Credentials, Region},
    operation::create_multipart_upload::CreateMultipartUploadOutput,
    primitives::ByteStream,
    types::{CompletedMultipartUpload, CompletedPart},
};
use tokio::io::AsyncWriteExt;

/// R2 storage backed by the S3 API. Everything the browser or the build step
/// writes lives in Cloudflare R2; the VM only keeps transient local files
/// while compressing/building.
#[derive(Clone)]
pub struct R2Client {
    client: Client,
    pub bucket: String,
}

#[derive(Debug)]
pub struct R2Error(pub String);

impl std::fmt::Display for R2Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "R2: {}", self.0)
    }
}

impl std::error::Error for R2Error {}

impl R2Client {
    /// Builds the client from R2_* environment variables. Returns None when no
    /// R2 account is configured (e.g. local tests without a .env).
    pub fn from_env() -> Option<Self> {
        let account_id = env::var("R2_ACCOUNT_ID").ok()?;
        let access_key = env::var("R2_ACCESS_KEY_ID").ok()?;
        let secret_key = env::var("R2_SECRET_ACCESS_KEY").ok()?;
        let bucket = env::var("R2_BUCKET").ok()?;

        let endpoint = format!("https://{account_id}.r2.cloudflarestorage.com");
        let credentials = Credentials::new(
            access_key,
            secret_key,
            None,
            None,
            "backend",
        );
        let config = aws_sdk_s3::config::Builder::new()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new("auto"))
            .endpoint_url(endpoint)
            .force_path_style(true)
            .credentials_provider(credentials)
            .build();

        Some(Self {
            client: Client::from_conf(config),
            bucket,
        })
    }

    pub async fn create_multipart(
        &self,
        key: &str,
    ) -> Result<CreateMultipartUploadOutput, R2Error> {
        self.client
            .create_multipart_upload()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| R2Error(format!("create_multipart_upload {key}: {e}")))
    }

    pub async fn upload_part(
        &self,
        key: &str,
        upload_id: &str,
        part_number: i32,
        bytes: Vec<u8>,
    ) -> Result<String, R2Error> {
        self.client
            .upload_part()
            .bucket(&self.bucket)
            .key(key)
            .upload_id(upload_id)
            .part_number(part_number)
            .body(ByteStream::from(bytes))
            .send()
            .await
            .map_err(|e| R2Error(format!("upload_part {key}#{part_number}: {e}")))
            .and_then(|output| {
                output
                    .e_tag()
                    .map(str::to_string)
                    .ok_or_else(|| R2Error(format!("upload_part {key}#{part_number} missing etag")))
            })
    }

    pub async fn complete_multipart(
        &self,
        key: &str,
        upload_id: &str,
        parts: Vec<(i32, String)>,
    ) -> Result<(), R2Error> {
        let completed = CompletedMultipartUpload::builder()
            .set_parts(Some(
                parts
                    .into_iter()
                    .map(|(number, etag)| {
                        CompletedPart::builder()
                            .part_number(number)
                            .set_e_tag(Some(etag))
                            .build()
                    })
                    .collect(),
            ))
            .build();
        self.client
            .complete_multipart_upload()
            .bucket(&self.bucket)
            .key(key)
            .upload_id(upload_id)
            .multipart_upload(completed)
            .send()
            .await
            .map_err(|e| R2Error(format!("complete_multipart_upload {key}: {e}")))?;
        Ok(())
    }

    pub async fn abort_multipart(&self, key: &str, upload_id: &str) -> Result<(), R2Error> {
        self.client
            .abort_multipart_upload()
            .bucket(&self.bucket)
            .key(key)
            .upload_id(upload_id)
            .send()
            .await
            .map_err(|e| R2Error(format!("abort_multipart_upload {key}: {e}")))?;
        Ok(())
    }

    /// Streams a local file (produced by a transient build) up to R2.
    pub async fn put_object_from_file(&self, key: &str, path: &Path) -> Result<(), R2Error> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(ByteStream::from_path(path).await.map_err(|e| {
                R2Error(format!("put_object_from_file {key}: could not open {}: {e}", path.display()))
            })?)
            .send()
            .await
            .map_err(|e| R2Error(format!("put_object_from_file {key}: {e}")))?;
        Ok(())
    }

    /// Reads a byte range of an object (used for the boot-sector signature check).
    pub async fn get_object_range(
        &self,
        key: &str,
        start: u64,
        end: u64,
    ) -> Result<Vec<u8>, R2Error> {
        let output = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .range(format!("bytes={start}-{end}"))
            .send()
            .await
            .map_err(|e| R2Error(format!("get_object_range {key}: {e}")))?;
        let bytes = output
            .body
            .collect()
            .await
            .map_err(|e| R2Error(format!("get_object_range {key}: read body: {e}")))?;
        Ok(bytes.into_bytes().to_vec())
    }

    /// Reads an entire object into memory (used by the serving fallback).
    pub async fn get_object(&self, key: &str) -> Result<Vec<u8>, R2Error> {
        let output = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| R2Error(format!("get_object {key}: {e}")))?;
        let bytes = output
            .body
            .collect()
            .await
            .map_err(|e| R2Error(format!("get_object {key}: read body: {e}")))?;
        Ok(bytes.into_bytes().to_vec())
    }

    /// Streams an object's bytes (used by the ISO serving fallback).
    pub async fn get_object_reader(
        &self,
        key: &str,
    ) -> Result<Box<dyn tokio::io::AsyncRead + Send + Unpin>, R2Error> {
        let output = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| R2Error(format!("get_object {key}: {e}")))?;
        Ok(Box::new(output.body.into_async_read()))
    }

    /// Returns the size of an object, or None when it does not exist.
    pub async fn object_size(&self, key: &str) -> Result<Option<u64>, R2Error> {
        match self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
        {
            Ok(output) => Ok(output.content_length().map(|length| length as u64)),
            Err(error)
                if error
                    .as_service_error()
                    .is_some_and(|service| service.is_not_found()) =>
            {
                Ok(None)
            }
            Err(error) => Err(R2Error(format!("head_object {key}: {error}"))),
        }
    }

    /// Downloads an entire object to a transient local file for the build step.
    pub async fn download_to_file(&self, key: &str, path: &Path) -> Result<(), R2Error> {
        let output = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| R2Error(format!("download_to_file {key}: {e}")))?;
        let mut stream = output.body.into_async_read();
        let mut file = tokio::fs::File::create(path)
            .await
            .map_err(|e| R2Error(format!("download_to_file {key}: create {}: {e}", path.display())))?;
        tokio::io::copy(&mut stream, &mut file)
            .await
            .map_err(|e| R2Error(format!("download_to_file {key}: copy: {e}")))?;
        file.flush()
            .await
            .map_err(|e| R2Error(format!("download_to_file {key}: flush: {e}")))?;
        Ok(())
    }

    pub async fn delete_object(&self, key: &str) -> Result<(), R2Error> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| R2Error(format!("delete_object {key}: {e}")))?;
        Ok(())
    }

    /// Deletes every object under a prefix (used for version/system cleanup).
    pub async fn delete_prefix(&self, prefix: &str) -> Result<(), R2Error> {
        let mut token: Option<String> = None;
        loop {
            let mut request = self
                .client
                .list_objects_v2()
                .bucket(&self.bucket)
                .prefix(prefix);
            if let Some(token) = &token {
                request = request.continuation_token(token);
            }
            let output = request
                .send()
                .await
                .map_err(|e| R2Error(format!("list_objects_v2 {prefix}: {e}")))?;
            for object in output.contents().iter() {
                if let Some(key) = object.key() {
                    self.client
                        .delete_object()
                        .bucket(&self.bucket)
                        .key(key)
                        .send()
                        .await
                        .map_err(|e| R2Error(format!("delete_object {key}: {e}")))?;
                }
            }
            match output.next_continuation_token() {
                Some(next) if !next.is_empty() && output.is_truncated() == Some(true) => {
                    token = Some(next.to_string())
                }
                _ => break,
            }
        }
        Ok(())
    }
}
