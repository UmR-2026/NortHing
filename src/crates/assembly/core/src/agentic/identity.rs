use crate::util::errors::NortHingResult;
use std::path::PathBuf;

const IDENTITY_FILE_NAME: &str = "identity.md";

#[derive(Debug, Clone)]
pub struct IdentityConfig {
    pub user_name: String,
    pub agent_name: String,
    pub relationship: String,
    pub personality_keywords: String,
}

pub fn identity_path() -> PathBuf {
    #[cfg(test)]
    if let Some(path) = test_identity_path_override() {
        return path;
    }
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("northhing")
        .join(IDENTITY_FILE_NAME)
}

// ── Test-only isolation seam ────────────────────────────────────────
//
// `identity_path` normally points at `<config_dir>/northhing/identity.md`.
// In tests, writing to this path mutates the real user configuration directory.
// The seam below is a thread-local override mirroring `MemoryDbPathGuard`.

#[cfg(test)]
thread_local! {
    static TEST_IDENTITY_PATH: std::cell::RefCell<Option<PathBuf>> = std::cell::RefCell::new(None);
}

#[cfg(test)]
fn test_identity_path_override() -> Option<PathBuf> {
    TEST_IDENTITY_PATH.with(|c| c.borrow().clone())
}

/// RAII guard that redirects [`identity_path`] to an isolated path
/// for the lifetime of the guard on the calling thread only.
#[cfg(test)]
pub(crate) struct IdentityPathGuard {
    prev: Option<PathBuf>,
    path: Option<PathBuf>,
}

#[cfg(test)]
pub(crate) fn with_test_identity_path(path: PathBuf) -> IdentityPathGuard {
    let prev = TEST_IDENTITY_PATH.with(|c| c.borrow_mut().replace(path.clone()));
    IdentityPathGuard { prev, path: Some(path) }
}

/// Generates a unique temp-file path for an isolated identity file.
#[cfg(test)]
pub(crate) fn unique_test_identity_path() -> PathBuf {
    std::env::temp_dir().join(format!("northhing-test-identity-{}.md", uuid::Uuid::new_v4()))
}

#[cfg(test)]
impl Drop for IdentityPathGuard {
    fn drop(&mut self) {
        let path = self.path.take();
        TEST_IDENTITY_PATH.with(|c| *c.borrow_mut() = self.prev.take());
        if let Some(path) = path {
            std::fs::remove_file(&path).ok();
        }
    }
}

pub fn identity_exists() -> bool {
    identity_path().exists()
}

pub fn load_identity() -> Option<String> {
    let path = identity_path();
    if path.exists() {
        std::fs::read_to_string(path).ok()
    } else {
        None
    }
}

pub fn clear_identity() {
    let path = identity_path();
    if path.exists() {
        std::fs::remove_file(path).ok();
    }
}

pub fn save_identity(content: &str) -> NortHingResult<()> {
    let path = identity_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)?;
    Ok(())
}

pub fn build_identity_prompt(config: &IdentityConfig) -> String {
    format!(
        "根据以下配置生成一段自我认知提示词。\n\n\
        用户是【{user_name}】\n\
        你是【{agent_name}】\n\
        你是用户的【{relationship}】\n\
        你的性格更偏向大五人格中的【{personality_keywords}】性格\n\n\
        生成要求：\n\
        - 50-80 字中文\n\
        - 第一人称（\"我是...\"）\n\
        - 包含：自己的名字、用户称呼、关系定位、回复风格\n\
        - 用名字代替所有代词，避免人称歧义\n\
        - 语气：平静、自知、克制\n\
        - 不要提\"AI\"\"语言模型\"\"大五人格\"\"性格偏向\"等元信息\n\
        - 直接表达\"我就是这样的\"，不解释\n",
        user_name = config.user_name,
        agent_name = config.agent_name,
        relationship = config.relationship,
        personality_keywords = config.personality_keywords,
    )
}
