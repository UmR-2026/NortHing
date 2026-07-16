use std::path::{Path, PathBuf};

pub struct TestTempDir {
    path: PathBuf,
}

impl TestTempDir {
    pub fn new(prefix: &str) -> Self {
        let nonce = uuid::Uuid::new_v4();
        let path = std::env::temp_dir().join(format!("northhing-{prefix}-{nonce}"));
        std::fs::create_dir_all(&path).expect("test temp dir should be created");
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestTempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
