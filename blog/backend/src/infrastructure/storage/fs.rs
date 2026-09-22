// Filesystem object store. Keys map to paths under the project-demos root
// (`{PROJECT_DEMOS_PATH}/{key}`), the same layout `sync_v86_to_r2.sh` mirrors
// into R2, so artifacts stay compatible with either backend and are covered
// by the compose volume and the backup download. The operations are split
// across `objects`, `multipart`, and `prefix`; this module owns the type and
// its constructor.
use std::path::PathBuf;

mod multipart;
mod objects;
mod prefix;

#[derive(Clone)]
pub struct FsStore {
    pub(super) root: PathBuf,
}

impl FsStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

#[cfg(test)]
mod tests;
