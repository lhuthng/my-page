// Object-store backend selection: R2 when configured, filesystem otherwise.
use std::path::Path;

use crate::infrastructure::storage::ObjectStore;

pub fn from_env(demos_dir: &Path) -> Result<ObjectStore, String> {
    ObjectStore::from_env(demos_dir)
}
