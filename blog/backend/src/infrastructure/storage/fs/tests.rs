use super::*;

fn store() -> (FsStore, PathBuf) {
    let root = std::env::temp_dir().join(format!("fs-store-test-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    (FsStore::new(root.clone()), root)
}

#[tokio::test]
async fn put_get_roundtrip_and_missing_object() {
    let (store, root) = store();
    store
        .put_object_bytes("v86/saves/1/2/save.zst", b"hello save".to_vec())
        .await
        .unwrap();
    assert_eq!(store.get_object("v86/saves/1/2/save.zst").await.unwrap(), b"hello save");
    assert_eq!(
        store.object_size("v86/saves/1/2/save.zst").await.unwrap(),
        Some(10)
    );
    assert_eq!(store.object_size("v86/saves/1/2/missing").await.unwrap(), None);
    assert!(store.get_object("v86/saves/1/2/missing").await.is_err());
    std::fs::remove_dir_all(root).ok();
}

#[tokio::test]
async fn multipart_roundtrip_with_out_of_order_parts() {
    let (store, root) = store();
    let key = "v86/tmp/snapshots/upload-1.zst";
    let session = store.create_multipart(key).await.unwrap();
    let etag1 = store
        .upload_part(key, &session.upload_id, 1, b"aaaa".to_vec())
        .await
        .unwrap();
    let etag2 = store
        .upload_part(key, &session.upload_id, 2, b"bb".to_vec())
        .await
        .unwrap();
    store
        .complete_multipart(key, &session.upload_id, vec![(2, etag2), (1, etag1)])
        .await
        .unwrap();
    assert_eq!(store.get_object(key).await.unwrap(), b"bbaaaa");
    // The session dir is gone and the final object replaced the tmp files.
    assert!(!root.join("v86/tmp/snapshots/upload-1.zst.multipart").exists());
    std::fs::remove_dir_all(root).ok();
}

#[tokio::test]
async fn multipart_abort_removes_session() {
    let (store, root) = store();
    let key = "v86/tmp/snapshots/upload-2.zst";
    let session = store.create_multipart(key).await.unwrap();
    store
        .upload_part(key, &session.upload_id, 1, b"data".to_vec())
        .await
        .unwrap();
    store
        .abort_multipart(key, &session.upload_id)
        .await
        .unwrap();
    assert!(!root.join("v86/tmp/snapshots/upload-2.zst.multipart").exists());
    assert_eq!(store.object_size(key).await.unwrap(), None);
    // Aborting an unknown session is a no-op, like S3.
    store.abort_multipart(key, "does-not-exist").await.unwrap();
    std::fs::remove_dir_all(root).ok();
}

#[tokio::test]
async fn delete_object_and_prefix_are_idempotent() {
    let (store, root) = store();
    store
        .put_object_bytes("v86/games/sha1/full.iso", b"iso".to_vec())
        .await
        .unwrap();
    store
        .put_object_bytes("v86/games/sha1/0-4.img.zst", b"part".to_vec())
        .await
        .unwrap();
    store.delete_object("v86/games/sha1/full.iso").await.unwrap();
    store.delete_object("v86/games/sha1/full.iso").await.unwrap();
    store.delete_prefix("v86/games/sha1").await.unwrap();
    store.delete_prefix("v86/games/sha1").await.unwrap();
    assert_eq!(store.object_size("v86/games/sha1/0-4.img.zst").await.unwrap(), None);
    std::fs::remove_dir_all(root).ok();
}

#[tokio::test]
async fn traversal_keys_are_rejected() {
    let (store, root) = store();
    for key in ["../escape", "a/../../b", "/absolute", "a/./b", "a/../b", ""] {
        assert!(
            store.put_object_bytes(key, b"x".to_vec()).await.is_err(),
            "'{key}' should be rejected"
        );
    }
    // Nothing escaped the root.
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 0);
    std::fs::remove_dir_all(root).ok();
}

#[tokio::test]
async fn get_object_range_reads_inclusive_bounds() {
    let (store, root) = store();
    store
        .put_object_bytes("v86/assets/systems/sha/base.img.zst", b"0123456789".to_vec())
        .await
        .unwrap();
    assert_eq!(
        store
            .get_object_range("v86/assets/systems/sha/base.img.zst", 2, 4)
            .await
            .unwrap(),
        b"234"
    );
    std::fs::remove_dir_all(root).ok();
}

#[tokio::test]
async fn put_object_from_file_copies_content() {
    let (store, root) = store();
    let source = root.join("source.bin");
    std::fs::write(&source, b"built artifact").unwrap();
    store
        .put_object_from_file("v86/games/sha2/base.img.zst", &source)
        .await
        .unwrap();
    assert_eq!(
        store.get_object("v86/games/sha2/base.img.zst").await.unwrap(),
        b"built artifact"
    );
    std::fs::remove_dir_all(root).ok();
}
