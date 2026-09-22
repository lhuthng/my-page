use std::fs;

use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::constants::MANIFEST_MAX_BYTES;
use super::manifest::{parse_mouse_config, save_files_from_manifest, validate_manifest};
use super::upload_session::split_asset;

#[test]
fn manifest_is_exact_but_rejects_nul_and_oversize() {
    let text = "exe=Doraemon.exe\r\nargs=-m -5";
    assert_eq!(
        validate_manifest(text).ok().unwrap(),
        hex::encode(Sha256::digest(text.as_bytes()))
    );
    assert!(validate_manifest("exe=a.exe\0args=-m").is_err());
    assert!(validate_manifest(&"x".repeat(MANIFEST_MAX_BYTES + 1)).is_err());
}

#[test]
fn immutable_parts_use_v86_range_names() {
    use std::io::Read;
    let root = std::env::temp_dir().join(format!("v86-parts-test-{}", Uuid::new_v4()));
    fs::create_dir_all(&root).unwrap();
    let source = root.join("disk.img");
    fs::write(&source, b"abcdefghij").unwrap();
    let parts = root.join("parts");
    assert_eq!(
        split_asset(&source, &parts, 4, "img.zst", None, 6)
            .ok()
            .unwrap(),
        3
    );
    let read_decompressed = |name| {
        let compressed = fs::read(parts.join(name)).unwrap();
        let mut decoder = zstd::stream::read::Decoder::new(&compressed[..]).unwrap();
        let mut buf = Vec::new();
        decoder.read_to_end(&mut buf).unwrap();
        buf
    };
    assert_eq!(read_decompressed("0-4.img.zst"), b"abcd");
    assert_eq!(read_decompressed("4-8.img.zst"), b"efgh");
    assert_eq!(read_decompressed("8-12.img.zst"), b"ij\0\0");
    fs::remove_dir_all(root).ok();
}

#[test]
fn save_files_parse_and_validate() {
    assert_eq!(
        save_files_from_manifest(
            "exe=a.exe\nsave_paths=Save0001.dat; A/save0001.dat; backup/what.bak"
        )
        .ok()
        .unwrap(),
        vec![
            "Save0001.dat".to_string(),
            r"A\save0001.dat".to_string(),
            r"backup\what.bak".to_string(),
        ]
    );
    assert_eq!(
        save_files_from_manifest("exe=a.exe\nsaves=Save0001.dat,save0001.dat")
            .ok()
            .unwrap(),
        vec!["Save0001.dat".to_string()]
    );
    assert_eq!(
        save_files_from_manifest("exe=a.exe\nsaves=")
            .ok()
            .unwrap()
            .len(),
        0
    );
    for bad in [
        "save_paths=/abs.dat",
        "save_paths=a\\b\\",
        "save_paths=a//b.dat",
        "save_paths=../x.dat",
        "save_paths=./x.dat",
        "save_paths=a\\b\\..\\c.dat",
        "save_paths=a=b.dat",
        "save_paths=a:b.dat",
        "save_paths=*.dat",
        "save_paths=?.dat",
        "save_paths=a b.dat",
    ] {
        assert!(
            save_files_from_manifest(&format!("exe=a.exe\n{bad}")).is_err(),
            "{bad} should be rejected"
        );
    }
}

#[test]
fn mouse_config_defaults_and_parses() {
    let default = parse_mouse_config("exe=a.exe").ok().unwrap();
    assert!(!default.revert_mouse_y);
    assert_eq!(default.mouse_speed, 1.0);

    let inverted = parse_mouse_config("exe=a.exe\nrevert_mouse_y=1")
        .ok()
        .unwrap();
    assert!(inverted.revert_mouse_y);
    assert_eq!(inverted.mouse_speed, 1.0);

    let fast = parse_mouse_config("exe=a.exe\nmouse_speed=2.5")
        .ok()
        .unwrap();
    assert!(!fast.revert_mouse_y);
    assert_eq!(fast.mouse_speed, 2.5);

    let both = parse_mouse_config("exe=a.exe\nrevert_mouse_y=true\nmouse_speed=0.5")
        .ok()
        .unwrap();
    assert!(both.revert_mouse_y);
    assert_eq!(both.mouse_speed, 0.5);

    for bad in [
        "revert_mouse_y=banana",
        "mouse_speed=nope",
        "mouse_speed=-1",
        "mouse_speed=0",
    ] {
        assert!(
            parse_mouse_config(&format!("exe=a.exe\n{bad}")).is_err(),
            "{bad}"
        );
    }
}
