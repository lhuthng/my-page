// Argument parsing: flags and usage text.
use std::path::PathBuf;

#[derive(Default)]
struct Args {
    url: Option<String>,
    key: Option<String>,
    env_file: Option<PathBuf>,
    db_path: Option<PathBuf>,
    media_dir: Option<PathBuf>,
    demos_dir: Option<PathBuf>,
    skip: HashSet<String>,
    prune: bool,
    dry_run: bool,
    yes: bool,
}

fn print_usage() {
    println!(
        "Usage: cargo run --bin sync-pull -- --url <site-url> --key <bsk_…|@file> [options]

Required:
  --url <site-url>        Public URL of the source site (requests go to {{url}}/api/sync/…).
  --key <key|@file>       Sync key from the admin dashboard, or @path to a file holding it.

Options:
  --env <path>            Backend .env to read DATABASE_URL / MEDIA_PATH / PROJECT_DEMOS_PATH
                          from (default: ./backend/.env then ./.env).
  --db <path>             Override the local SQLite database path.
  --media-dir <path>      Override the local media directory.
  --demos-dir <path>      Override the local project-demos directory.
  --skip <what>           Skip a section: media, demos, artifacts (repeatable).
  --prune                 Delete local files that are absent from the source manifest.
  --dry-run               Show what would happen without writing anything.
  --yes                   Skip the confirmation prompt (the overwrite warning
                          is still printed)."
    );
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args::default();
    let mut iter = std::env::args().skip(1);
    while let Some(flag) = iter.next() {
        let mut value = |name: &str| -> Result<String, String> {
            iter.next().ok_or_else(|| format!("{name} needs a value"))
        };
        match flag.as_str() {
            "--url" => args.url = Some(value("--url")?),
            "--key" => args.key = Some(value("--key")?),
            "--env" => args.env_file = Some(PathBuf::from(value("--env")?)),
            "--db" => args.db_path = Some(PathBuf::from(value("--db")?)),
            "--media-dir" => args.media_dir = Some(PathBuf::from(value("--media-dir")?)),
            "--demos-dir" => args.demos_dir = Some(PathBuf::from(value("--demos-dir")?)),
            "--skip" => {
                let what = value("--skip")?;
                if !matches!(what.as_str(), "media" | "demos" | "artifacts") {
                    return Err(format!(
                        "--skip must be media, demos, or artifacts (got '{what}')"
                    ));
                }
                args.skip.insert(what);
            }
            "--prune" => args.prune = true,
            "--dry-run" => args.dry_run = true,
            "--yes" => args.yes = true,
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument '{other}' (--help shows usage)")),
        }
    }
    Ok(args)
}

/// Reads KEY=VALUE lines from a .env file without touching real env vars.
