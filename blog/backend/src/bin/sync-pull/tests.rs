use super::*;

    #[test]
    fn update_env_preserves_and_overrides() {
        let dir = std::env::temp_dir().join(format!("sync-pull-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(".env");
        std::fs::write(
            &path,
            "# comment\nJWT_SECRET=abc\nDATABASE_URL=sqlite:old/blog.db\nMEDIA_PATH=./media\n",
        )
        .unwrap();

        update_env_file(
            &path,
            &[
                ("DATABASE_URL", "sqlite:data/blog.db".into()),
                ("MEDIA_PATH", "media".into()),
                ("STORAGE_BACKEND", "fs".into()),
            ],
        )
        .unwrap();

        let out = std::fs::read_to_string(&path).unwrap();
        assert!(out.contains("# comment"));
        assert!(out.contains("JWT_SECRET=abc"));
        assert!(out.contains("DATABASE_URL=sqlite:data/blog.db"));
        assert!(out.contains("MEDIA_PATH=media"));
        assert!(out.contains("STORAGE_BACKEND=fs"));
        assert!(!out.contains("old/blog.db"));

        // commented-out lines must not be touched
        std::fs::write(&path, "# DATABASE_URL=postgresql://x\n").unwrap();
        update_env_file(&path, &[("DATABASE_URL", "sqlite:d.db".into())]).unwrap();
        let out = std::fs::read_to_string(&path).unwrap();
        assert!(out.contains("# DATABASE_URL=postgresql://x"));
        assert!(out.contains("DATABASE_URL=sqlite:d.db"));

        // fresh file: keys land even though nothing existed
        std::fs::remove_file(&path).unwrap();
        update_env_file(&path, &[("MEDIA_PATH", "m".into())]).unwrap();
        let out = std::fs::read_to_string(&path).unwrap();
        assert!(out.ends_with("MEDIA_PATH=m\n"));

        std::fs::remove_dir_all(&dir).unwrap();
    }
