use std::{path::Path, time::Duration};

use aws_sdk_s3::{
    Client,
    config::{BehaviorVersion, Credentials, Region},
    operation::create_multipart_upload::CreateMultipartUploadOutput,
    presigning::PresigningConfig,
    primitives::ByteStream,
    types::{CompletedMultipartUpload, CompletedPart, Delete, ObjectIdentifier},
};
use tokio::io::AsyncWriteExt;

use super::{S3Settings, StorageError};

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct ListedObject {
    pub key: String,
    pub size: u64,
    pub etag: Option<String>,
    pub modified_at: Option<i64>,
}

/// S3-compatible storage backed by the S3 API — the `R2` variant of
/// [`ObjectStore`](super::ObjectStore). One bucket holds the v86 artifacts and
/// the audiobook audio, separated by their key prefixes (`v86/`, `audio/`); two
/// clients exist only because `STORAGE_BACKEND` and `AUDIO_BACKEND` are
/// independent switches.
#[derive(Clone)]
pub struct R2Client {
    client: Client,
    pub bucket: String,
}

impl R2Client {
    /// Builds the artifact client from R2_* environment variables. Returns
    /// None when no R2 account is configured (e.g. local tests without a
    /// .env).
    pub fn from_env() -> Option<Self> {
        S3Settings::from_r2_env().map(Self::from_settings)
    }

    /// Builds a client for one bucket from explicit settings, so the artifact
    /// store and the audio store can be configured independently.
    pub fn from_settings(settings: S3Settings) -> Self {
        let credentials = Credentials::new(
            settings.access_key,
            settings.secret_key,
            None,
            None,
            "backend",
        );
        let config = aws_sdk_s3::config::Builder::new()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new(settings.region))
            .endpoint_url(settings.endpoint)
            .force_path_style(true)
            .credentials_provider(credentials)
            .build();

        Self {
            client: Client::from_conf(config),
            bucket: settings.bucket,
        }
    }

    pub async fn create_multipart(
        &self,
        key: &str,
    ) -> Result<CreateMultipartUploadOutput, StorageError> {
        self.client
            .create_multipart_upload()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| StorageError(format!("create_multipart_upload {key}: {e}")))
    }

    pub async fn upload_part(
        &self,
        key: &str,
        upload_id: &str,
        part_number: i32,
        bytes: Vec<u8>,
    ) -> Result<String, StorageError> {
        self.client
            .upload_part()
            .bucket(&self.bucket)
            .key(key)
            .upload_id(upload_id)
            .part_number(part_number)
            .body(ByteStream::from(bytes))
            .send()
            .await
            .map_err(|e| StorageError(format!("upload_part {key}#{part_number}: {e}")))
            .and_then(|output| {
                output.e_tag().map(str::to_string).ok_or_else(|| {
                    StorageError(format!("upload_part {key}#{part_number} missing etag"))
                })
            })
    }

    pub async fn complete_multipart(
        &self,
        key: &str,
        upload_id: &str,
        parts: Vec<(i32, String)>,
    ) -> Result<(), StorageError> {
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
            .map_err(|e| StorageError(format!("complete_multipart_upload {key}: {e}")))?;
        Ok(())
    }

    pub async fn abort_multipart(&self, key: &str, upload_id: &str) -> Result<(), StorageError> {
        self.client
            .abort_multipart_upload()
            .bucket(&self.bucket)
            .key(key)
            .upload_id(upload_id)
            .send()
            .await
            .map_err(|e| StorageError(format!("abort_multipart_upload {key}: {e}")))?;
        Ok(())
    }

    /// Streams a local file (produced by a transient build) up to R2.
    pub async fn put_object_from_file(&self, key: &str, path: &Path) -> Result<(), StorageError> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(ByteStream::from_path(path).await.map_err(|e| {
                StorageError(format!(
                    "put_object_from_file {key}: could not open {}: {e}",
                    path.display()
                ))
            })?)
            .send()
            .await
            .map_err(|e| StorageError(format!("put_object_from_file {key}: {e}")))?;
        Ok(())
    }

    /// Uploads a local file together with the metadata the object must carry.
    ///
    /// Both arguments matter for media: audio served as
    /// `application/octet-stream` is refused by Safari and AVPlayer, and the
    /// `Cache-Control` set here is what the client caches on once it is
    /// fetching from the bucket directly rather than from this backend.
    pub async fn put_file_with_metadata(
        &self,
        key: &str,
        path: &Path,
        content_type: &str,
        cache_control: &str,
    ) -> Result<(), StorageError> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .content_type(content_type)
            .cache_control(cache_control)
            .body(ByteStream::from_path(path).await.map_err(|e| {
                StorageError(format!(
                    "put_file_with_metadata {key}: could not open {}: {e}",
                    path.display()
                ))
            })?)
            .send()
            .await
            .map_err(|e| StorageError(format!("put_file_with_metadata {key}: {e}")))?;
        Ok(())
    }

    /// Uploads an in-memory blob (used for the small per-user floppy saves).
    pub async fn put_object_bytes(&self, key: &str, bytes: Vec<u8>) -> Result<(), StorageError> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(ByteStream::from(bytes))
            .send()
            .await
            .map_err(|e| StorageError(format!("put_object_bytes {key}: {e}")))?;
        Ok(())
    }

    /// Reads a byte range of an object (used for the boot-sector signature check).
    pub async fn get_object_range(
        &self,
        key: &str,
        start: u64,
        end: u64,
    ) -> Result<Vec<u8>, StorageError> {
        let output = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .range(format!("bytes={start}-{end}"))
            .send()
            .await
            .map_err(|e| StorageError(format!("get_object_range {key}: {e}")))?;
        let bytes = output
            .body
            .collect()
            .await
            .map_err(|e| StorageError(format!("get_object_range {key}: read body: {e}")))?;
        Ok(bytes.into_bytes().to_vec())
    }

    /// Reads an entire object into memory (used by the serving fallback).
    pub async fn get_object(&self, key: &str) -> Result<Vec<u8>, StorageError> {
        let output = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| StorageError(format!("get_object {key}: {e}")))?;
        let bytes = output
            .body
            .collect()
            .await
            .map_err(|e| StorageError(format!("get_object {key}: read body: {e}")))?;
        Ok(bytes.into_bytes().to_vec())
    }

    /// Streams an object's bytes (used by the ISO serving fallback).
    pub async fn get_object_reader(
        &self,
        key: &str,
    ) -> Result<Box<dyn tokio::io::AsyncRead + Send + Unpin>, StorageError> {
        let output = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| StorageError(format!("get_object {key}: {e}")))?;
        Ok(Box::new(output.body.into_async_read()))
    }

    /// Returns the size of an object, or None when it does not exist.
    pub async fn object_size(&self, key: &str) -> Result<Option<u64>, StorageError> {
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
            Err(error) => Err(StorageError(format!("head_object {key}: {error}"))),
        }
    }

    /// Downloads an entire object to a transient local file for the build step.
    pub async fn download_to_file(&self, key: &str, path: &Path) -> Result<(), StorageError> {
        let output = match self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
        {
            Ok(output) => output,
            Err(error) => {
                let status = error
                    .raw_response()
                    .map(|raw| raw.status().as_u16().to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                let detail = error
                    .as_service_error()
                    .map(|service| format!("service error (HTTP {status}): {service}"))
                    .unwrap_or_else(|| format!("{error} (HTTP {status})"));
                return Err(StorageError(format!("download_to_file {key}: {detail}")));
            }
        };
        let mut stream = output.body.into_async_read();
        let mut file = tokio::fs::File::create(path).await.map_err(|e| {
            StorageError(format!(
                "download_to_file {key}: create {}: {e}",
                path.display()
            ))
        })?;
        tokio::io::copy(&mut stream, &mut file)
            .await
            .map_err(|e| StorageError(format!("download_to_file {key}: copy: {e}")))?;
        file.flush()
            .await
            .map_err(|e| StorageError(format!("download_to_file {key}: flush: {e}")))?;
        Ok(())
    }

    /// A signed, expiring URL for an object, for a client that will fetch the
    /// bytes itself.
    ///
    /// Signing is what gives a URL a lifetime, so a link that travels — into a
    /// log, a screenshot, someone else's tab — stops working when it runs out.
    ///
    /// The bound on that promise is worth being explicit about: this bucket is
    /// served on a public domain, so an object reachable without a signature is
    /// reachable even once one has expired. Signing expires *access through this
    /// URL*; it is not secrecy. Deployments that need the latter have to take
    /// the public domain away first.
    pub async fn presigned_get_url(
        &self,
        key: &str,
        expires_in: Duration,
    ) -> Result<String, StorageError> {
        let config = PresigningConfig::expires_in(expires_in)
            .map_err(|e| StorageError(format!("presign {key}: {e}")))?;

        let request = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .presigned(config)
            .await
            .map_err(|e| StorageError(format!("presign {key}: {e}")))?;

        Ok(request.uri().to_string())
    }

    pub async fn delete_object(&self, key: &str) -> Result<(), StorageError> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| StorageError(format!("delete_object {key}: {e}")))?;
        Ok(())
    }

    pub async fn delete_objects(&self, keys: &[String]) -> Result<(), StorageError> {
        for batch in keys.chunks(1000) {
            let objects = batch
                .iter()
                .map(|key| ObjectIdentifier::builder().key(key).build())
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| StorageError(format!("delete_objects: {e}")))?;
            let delete = Delete::builder()
                .set_objects(Some(objects))
                .quiet(false)
                .build()
                .map_err(|e| StorageError(format!("delete_objects: {e}")))?;
            let output = self
                .client
                .delete_objects()
                .bucket(&self.bucket)
                .delete(delete)
                .send()
                .await
                .map_err(|e| StorageError(format!("delete_objects: {e}")))?;
            if !output.errors().is_empty() {
                return Err(StorageError(format!(
                    "delete_objects: {:?}",
                    output.errors()
                )));
            }
            let acknowledged: std::collections::HashSet<_> = output
                .deleted()
                .iter()
                .filter_map(|object| object.key())
                .collect();
            if acknowledged.len() != batch.len()
                || batch.iter().any(|key| !acknowledged.contains(key.as_str()))
            {
                return Err(StorageError(
                    "delete_objects: incomplete acknowledgements".into(),
                ));
            }
        }
        Ok(())
    }

    pub async fn list_prefix(&self, prefix: &str) -> Result<Vec<(String, u64)>, StorageError> {
        Ok(self
            .list_prefix_with_metadata(prefix)
            .await?
            .into_iter()
            .map(|object| (object.key, object.size))
            .collect())
    }

    pub async fn list_prefix_with_metadata(
        &self,
        prefix: &str,
    ) -> Result<Vec<ListedObject>, StorageError> {
        let mut objects = Vec::new();
        let mut tokens = std::collections::HashSet::new();
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
                .map_err(|e| StorageError(format!("list_objects_v2 {prefix}: {e}")))?;
            for object in output.contents() {
                let key = object
                    .key()
                    .filter(|key| key.starts_with(prefix))
                    .ok_or_else(|| {
                        StorageError("listing returned a missing or out-of-prefix key".into())
                    })?;
                let size = object.size().filter(|size| *size >= 0).ok_or_else(|| {
                    StorageError(format!("listing returned an invalid size for {key}"))
                })?;
                objects.push(ListedObject {
                    key: key.to_owned(),
                    size: size as u64,
                    etag: object.e_tag().map(str::to_owned),
                    modified_at: object.last_modified().map(|date| date.secs()),
                });
            }
            if output.is_truncated() != Some(true) {
                break;
            }
            let next = output
                .next_continuation_token()
                .filter(|next| !next.is_empty())
                .ok_or_else(|| {
                    StorageError("truncated listing has no continuation token".into())
                })?;
            if !tokens.insert(next.to_owned()) {
                return Err(StorageError("listing repeated a continuation token".into()));
            }
            token = Some(next.to_owned());
        }
        Ok(objects)
    }

    /// Deletes every object under a prefix (used for version/system cleanup).
    pub async fn delete_prefix(&self, prefix: &str) -> Result<(), StorageError> {
        for (key, _) in self.list_prefix(prefix).await? {
            self.delete_object(&key).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, body::Bytes, extract::Request, routing::any};
    use std::sync::{Arc, Mutex};

    struct FakeS3 {
        client: R2Client,
        task: tokio::task::JoinHandle<()>,
        requests: Arc<Mutex<Vec<(http::Uri, String)>>>,
    }

    impl Drop for FakeS3 {
        fn drop(&mut self) {
            self.task.abort();
        }
    }

    async fn fake_s3(reply: impl Fn(&str, usize) -> String + Send + Sync + 'static) -> FakeS3 {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let captured = requests.clone();
        let reply = Arc::new(reply);
        let app = Router::new().route(
            "/{*path}",
            any(move |request: Request| {
                let captured = captured.clone();
                let reply = reply.clone();
                async move {
                    let uri = request.uri().clone();
                    let body: Bytes = axum::body::to_bytes(request.into_body(), 1024 * 1024)
                        .await
                        .unwrap();
                    let body = String::from_utf8(body.to_vec()).unwrap();
                    let mut captured = captured.lock().unwrap();
                    captured.push((uri, body.clone()));
                    (
                        [(http::header::CONTENT_TYPE, "application/xml")],
                        reply(&body, captured.len()),
                    )
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let client = R2Client::from_settings(S3Settings {
            endpoint,
            region: "auto".into(),
            bucket: "bucket".into(),
            access_key: "test-key".into(),
            secret_key: "test-secret".into(),
        });
        FakeS3 {
            client,
            task,
            requests,
        }
    }

    #[tokio::test]
    async fn batch_delete_sends_only_exact_keys_in_bounded_batches() {
        let server = fake_s3(|body, _| {
            let deleted: String = body
                .split("<Key>")
                .skip(1)
                .map(|part| {
                    let key = part.split("</Key>").next().unwrap();
                    format!("<Deleted><Key>{key}</Key></Deleted>")
                })
                .collect();
            format!("<DeleteResult>{deleted}</DeleteResult>")
        })
        .await;
        let keys: Vec<_> = (0..1001).map(|i| format!("audio/{i:04}.mp3")).collect();
        server.client.delete_objects(&keys).await.unwrap();
        let requests = server.requests.lock().unwrap();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].1.matches("<Key>").count(), 1000);
        assert_eq!(requests[1].1.matches("<Key>").count(), 1);
        let sent: Vec<_> = requests
            .iter()
            .flat_map(|(_, body)| {
                body.split("<Key>")
                    .skip(1)
                    .map(|part| part.split("</Key>").next().unwrap().to_owned())
            })
            .collect();
        assert_eq!(sent, keys);
        assert!(
            requests
                .iter()
                .all(|(uri, _)| uri.query().unwrap().contains("delete"))
        );
    }

    #[tokio::test]
    async fn batch_delete_stops_on_per_object_errors_or_missing_acknowledgements() {
        for reply in [
            "<DeleteResult><Error><Key>audio/0000.mp3</Key><Code>AccessDenied</Code></Error></DeleteResult>",
            "<DeleteResult></DeleteResult>",
        ] {
            let server = fake_s3(move |_, _| reply.into()).await;
            let keys: Vec<_> = (0..1001).map(|i| format!("audio/{i:04}.mp3")).collect();
            assert!(server.client.delete_objects(&keys).await.is_err());
            assert_eq!(server.requests.lock().unwrap().len(), 1);
        }
    }

    #[tokio::test]
    async fn listing_retains_identity_and_uses_literal_prefix_across_pages() {
        let server = fake_s3(|_, page| match page {
            1 => "<ListBucketResult><IsTruncated>true</IsTruncated><NextContinuationToken>next</NextContinuationToken><Contents><Key>audio/a.mp3</Key><Size>4</Size><ETag>etag-a</ETag><LastModified>2020-01-01T00:00:00Z</LastModified></Contents></ListBucketResult>".into(),
            _ => "<ListBucketResult><IsTruncated>false</IsTruncated><Contents><Key>audio/b.mp3</Key><Size>8</Size><ETag>etag-b</ETag><LastModified>2020-01-02T00:00:00Z</LastModified></Contents></ListBucketResult>".into(),
        }).await;
        let objects = server
            .client
            .list_prefix_with_metadata("audio/")
            .await
            .unwrap();
        assert_eq!(objects.len(), 2);
        assert_eq!(objects[0].etag.as_deref(), Some("etag-a"));
        assert_eq!(objects[0].size, 4);
        assert_eq!(objects[0].modified_at, Some(1577836800));
        let requests = server.requests.lock().unwrap();
        for (uri, _) in requests.iter() {
            let url = reqwest::Url::parse(&format!("http://localhost{uri}")).unwrap();
            assert!(
                url.query_pairs()
                    .any(|(key, value)| key == "prefix" && value == "audio/")
            );
        }
        assert!(
            requests[1]
                .0
                .query()
                .unwrap()
                .contains("continuation-token=next")
        );
    }

    #[tokio::test]
    async fn incomplete_or_out_of_prefix_listing_is_an_error() {
        for reply in [
            "<ListBucketResult><IsTruncated>true</IsTruncated></ListBucketResult>",
            "<ListBucketResult><IsTruncated>true</IsTruncated><NextContinuationToken>same</NextContinuationToken></ListBucketResult>",
            "<ListBucketResult><Contents><Key>audio-backup/a.mp3</Key><Size>4</Size></Contents></ListBucketResult>",
            "<ListBucketResult><Contents><Key>audio/a.mp3</Key></Contents></ListBucketResult>",
        ] {
            let server = fake_s3(move |_, _| reply.into()).await;
            assert!(
                server
                    .client
                    .list_prefix_with_metadata("audio/")
                    .await
                    .is_err()
            );
        }
    }
}
