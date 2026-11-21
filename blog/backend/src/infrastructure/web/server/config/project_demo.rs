// Demo upload limits and layout for js-dos and v86 artifacts.
use std::env;
use std::path::PathBuf;

#[derive(Clone)]
pub struct ProjectDemoConfig {
    pub dir: PathBuf,
    pub max_archive_size: u64,
    pub max_extracted_size: u64,
    pub max_files: usize,
    pub max_jsdos_size: u64,
    pub jsdos_chunk_size: u64,
    pub upload_session_ttl_hours: u64,
    pub max_v86_base_size: u64,
    pub max_v86_game_extracted_size: u64,
    pub v86_upload_chunk_size: u64,
    pub v86_download_chunk_size: u64,
    pub v86_assets_dir: PathBuf,
    pub max_payload_size: u64,
    pub max_payload_extracted_size: u64,
    pub payload_chunk_size: u64,
    pub max_payload_files: usize,
}

impl ProjectDemoConfig {
    pub fn from_env() -> Self {
        let demo_path = env::var("PROJECT_DEMOS_PATH").expect("PROJECT_DEMOS_PATH must be set");
        let dir = PathBuf::from(&demo_path);
        if !dir.exists() {
            std::fs::create_dir_all(&dir).expect("Failed to create project demo directory");
        }

        let max_archive_size = env::var("PROJECT_DEMO_MAX_ARCHIVE_BYTES")
            .unwrap_or_else(|_| (100 * 1024 * 1024_u64).to_string())
            .parse::<u64>()
            .expect("PROJECT_DEMO_MAX_ARCHIVE_BYTES must be an integer");
        let max_extracted_size = env::var("PROJECT_DEMO_MAX_EXTRACTED_BYTES")
            .unwrap_or_else(|_| (200 * 1024 * 1024_u64).to_string())
            .parse::<u64>()
            .expect("PROJECT_DEMO_MAX_EXTRACTED_BYTES must be an integer");
        let max_files = env::var("PROJECT_DEMO_MAX_FILES")
            .unwrap_or_else(|_| "2000".to_string())
            .parse::<usize>()
            .expect("PROJECT_DEMO_MAX_FILES must be an integer");
        let max_jsdos_size = env::var("PROJECT_JSDOS_MAX_BYTES")
            .unwrap_or_else(|_| (500 * 1024 * 1024_u64).to_string())
            .parse::<u64>()
            .expect("PROJECT_JSDOS_MAX_BYTES must be an integer");
        let jsdos_chunk_size = env::var("PROJECT_JSDOS_CHUNK_BYTES")
            .unwrap_or_else(|_| (8 * 1024 * 1024_u64).to_string())
            .parse::<u64>()
            .expect("PROJECT_JSDOS_CHUNK_BYTES must be an integer");
        let upload_session_ttl_hours = env::var("PROJECT_JSDOS_UPLOAD_TTL_HOURS")
            .unwrap_or_else(|_| "24".to_string())
            .parse::<u64>()
            .expect("PROJECT_JSDOS_UPLOAD_TTL_HOURS must be an integer");
        let max_v86_base_size = env::var("PROJECT_V86_BASE_MAX_BYTES")
            .unwrap_or_else(|_| (2 * 1024 * 1024 * 1024_u64).to_string())
            .parse::<u64>()
            .expect("PROJECT_V86_BASE_MAX_BYTES must be an integer");
        let max_v86_game_extracted_size = env::var("PROJECT_V86_GAME_EXTRACTED_MAX_BYTES")
            .unwrap_or_else(|_| (1024 * 1024 * 1024_u64).to_string())
            .parse::<u64>()
            .expect("PROJECT_V86_GAME_EXTRACTED_MAX_BYTES must be an integer");
        let v86_upload_chunk_size = env::var("PROJECT_V86_UPLOAD_CHUNK_BYTES")
            .unwrap_or_else(|_| (8 * 1024 * 1024_u64).to_string())
            .parse::<u64>()
            .expect("PROJECT_V86_UPLOAD_CHUNK_BYTES must be an integer");
        let v86_download_chunk_size = env::var("PROJECT_V86_DOWNLOAD_CHUNK_BYTES")
            .unwrap_or_else(|_| (256 * 1024_u64).to_string())
            .parse::<u64>()
            .expect("PROJECT_V86_DOWNLOAD_CHUNK_BYTES must be an integer");
        let v86_assets_dir = env::var("PROJECT_V86_ASSETS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets"));
        // Payload bundles ('shimkit', 'wasmshell'): one zip per game. The cap is
        // a source port's size — Vice City's bundle is about 700 MB — and the
        // extracted cap what unpacking may place on disk for one game.
        let max_payload_size = env::var("PROJECT_PAYLOAD_MAX_BYTES")
            .unwrap_or_else(|_| (2 * 1024 * 1024 * 1024_u64).to_string())
            .parse::<u64>()
            .expect("PROJECT_PAYLOAD_MAX_BYTES must be an integer");
        let max_payload_extracted_size = env::var("PROJECT_PAYLOAD_EXTRACTED_MAX_BYTES")
            .unwrap_or_else(|_| (4 * 1024 * 1024 * 1024_u64).to_string())
            .parse::<u64>()
            .expect("PROJECT_PAYLOAD_EXTRACTED_MAX_BYTES must be an integer");
        let payload_chunk_size = env::var("PROJECT_PAYLOAD_CHUNK_BYTES")
            .unwrap_or_else(|_| (8 * 1024 * 1024_u64).to_string())
            .parse::<u64>()
            .expect("PROJECT_PAYLOAD_CHUNK_BYTES must be an integer");
        // A source port's bundle holds the whole install plus its block store —
        // Vice City's is ~2,050 entries — far past the html5 demo's 2,000-file
        // budget, so payload bundles count against their own cap.
        let max_payload_files = env::var("PROJECT_PAYLOAD_MAX_FILES")
            .unwrap_or_else(|_| "20000".to_string())
            .parse::<usize>()
            .expect("PROJECT_PAYLOAD_MAX_FILES must be an integer");

        Self {
            dir,
            max_archive_size,
            max_extracted_size,
            max_files,
            max_jsdos_size,
            jsdos_chunk_size,
            upload_session_ttl_hours,
            max_v86_base_size,
            max_v86_game_extracted_size,
            v86_upload_chunk_size,
            v86_download_chunk_size,
            v86_assets_dir,
            max_payload_size,
            max_payload_extracted_size,
            payload_chunk_size,
            max_payload_files,
        }
    }
}
