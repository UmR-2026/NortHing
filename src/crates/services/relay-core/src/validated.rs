//! Validated input types for relay disk-path containment.
//!
//! `ValidatedRoomId`, `ValidatedRelPath`, and `ContentHash` are opaque
//! wrappers that guarantee their invariants at construction time. They are
//! first-class arguments throughout the `WebAssetStore` API, so dangerous
//! path sequences are rejected before any filesystem operation runs.

use std::fmt;
use std::path::{Component, Path};

use sha2::Digest;

// ── ValidatedRoomId ───────────────────────────────────────────────────

/// Error from validating a room ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomIdError {
    /// The string is empty.
    Empty,
    /// More than 64 characters.
    TooLong,
    /// Contains a character outside ASCII letters, digits, `-`, `_`.
    InvalidCharacter,
}

impl fmt::Display for RoomIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RoomIdError::Empty => write!(f, "room ID must not be empty"),
            RoomIdError::TooLong => write!(f, "room ID must be at most 64 characters"),
            RoomIdError::InvalidCharacter => write!(
                f,
                "room ID may only contain ASCII letters, digits, hyphen, and underscore"
            ),
        }
    }
}

impl std::error::Error for RoomIdError {}

/// A validated room ID.
///
/// Invariant: ASCII letters, digits, hyphen, or underscore; length 1..=64.
/// Compatible with `generate_room_id()` output (32 lowercase hex chars) and
/// legacy test IDs such as `stale-room`.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ValidatedRoomId(String);

impl ValidatedRoomId {
    /// The validated ID as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn validate(s: &str) -> Result<(), RoomIdError> {
        if s.is_empty() {
            return Err(RoomIdError::Empty);
        }
        if s.len() > 64 {
            return Err(RoomIdError::TooLong);
        }
        for c in s.chars() {
            if !(c.is_ascii_alphanumeric() || c == '-' || c == '_') {
                return Err(RoomIdError::InvalidCharacter);
            }
        }
        Ok(())
    }
}

impl TryFrom<&str> for ValidatedRoomId {
    type Error = RoomIdError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::validate(s)?;
        Ok(Self(s.to_string()))
    }
}

impl TryFrom<String> for ValidatedRoomId {
    type Error = RoomIdError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::validate(&s)?;
        Ok(Self(s))
    }
}

impl fmt::Debug for ValidatedRoomId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ValidatedRoomId({})", self.0)
    }
}

impl fmt::Display for ValidatedRoomId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ── ValidatedRelPath ──────────────────────────────────────────────────

/// Error from validating a relative file path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelPathError {
    /// The path is empty.
    Empty,
    /// Contains a NUL or control character.
    ControlChar,
    /// Contains a Windows drive-letter or UNC prefix.
    Prefix,
    /// Is absolute (root directory).
    RootDir,
    /// Contains a `..` parent-directory component.
    ParentDir,
    /// Contains a `.` current-directory component.
    CurDir,
}

impl fmt::Display for RelPathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RelPathError::Empty => write!(f, "path must not be empty"),
            RelPathError::ControlChar => write!(f, "path contains a control character"),
            RelPathError::Prefix => write!(f, "path must not contain a drive letter or UNC prefix"),
            RelPathError::RootDir => write!(f, "path must be relative"),
            RelPathError::ParentDir => write!(f, "path must not contain a parent-directory component"),
            RelPathError::CurDir => write!(f, "path must not contain a current-directory component"),
        }
    }
}

impl std::error::Error for RelPathError {}

/// A validated relative file path.
///
/// Invariant: after normalizing `\` to `/` and running `Path::components`,
/// every component is `Component::Normal`. `Prefix`, `RootDir`, `ParentDir`,
/// and `CurDir` components are rejected, as are empty paths and paths with
/// NUL/control characters. The stored value uses `/` separators.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ValidatedRelPath(String);

impl ValidatedRelPath {
    /// The validated path as a string slice (normalized to `/`).
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn validate(s: &str) -> Result<(), RelPathError> {
        if s.is_empty() {
            return Err(RelPathError::Empty);
        }
        if s.bytes().any(|b| b.is_ascii_control()) {
            return Err(RelPathError::ControlChar);
        }
        let normalized = s.replace('\\', "/");
        // Windows drive-letter and current-directory checks applied upfront so
        // they survive on non-Windows where Path::components sees `X:` as a plain
        // Normal component. This keeps the rule cross-platform.
        for seg in normalized.split('/') {
            if is_drive_letter(seg) {
                return Err(RelPathError::Prefix);
            }
            if seg == "." {
                return Err(RelPathError::CurDir);
            }
        }
        let mut saw_normal = false;
        for component in Path::new(&normalized).components() {
            match component {
                Component::Normal(_) => {
                    saw_normal = true;
                }
                Component::Prefix(_) => return Err(RelPathError::Prefix),
                Component::RootDir => return Err(RelPathError::RootDir),
                Component::ParentDir => return Err(RelPathError::ParentDir),
                Component::CurDir => return Err(RelPathError::CurDir),
            }
        }
        if !saw_normal {
            return Err(RelPathError::Empty);
        }
        Ok(())
    }
}

/// A single-component `X:` drive-letter reference. On Windows this is a
/// `Component::Prefix` and is rejected there; on other platforms it would
/// surface as a normal component, so it is rejected explicitly to keep the
/// rule cross-platform.
fn is_drive_letter(part: &str) -> bool {
    let b = part.as_bytes();
    b.len() == 2 && b[0].is_ascii_alphabetic() && b[1] == b':'
}

impl TryFrom<&str> for ValidatedRelPath {
    type Error = RelPathError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::validate(s)?;
        Ok(Self(s.replace('\\', "/")))
    }
}

impl TryFrom<String> for ValidatedRelPath {
    type Error = RelPathError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::validate(&s)?;
        Ok(Self(s.replace('\\', "/")))
    }
}

impl fmt::Debug for ValidatedRelPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ValidatedRelPath({})", self.0)
    }
}

impl fmt::Display for ValidatedRelPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ── ContentHash ───────────────────────────────────────────────────────

/// Error from validating a content hash.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContentHashError;

impl fmt::Display for ContentHashError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "content hash must be exactly 64 lowercase hex characters")
    }
}

impl std::error::Error for ContentHashError {}

/// A content hash: exactly 64 lowercase hex characters (SHA-256 hex digest).
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ContentHash(String);

impl ContentHash {
    /// The validated hash as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Compute a SHA-256 hash of given data and return the ContentHash.
    pub fn from_data(data: &[u8]) -> Self {
        let mut hasher = sha2::Sha256::new();
        hasher.update(data);
        let result = format!("{:x}", hasher.finalize());
        Self(result)
    }

    fn validate(s: &str) -> Result<(), ContentHashError> {
        if s.len() != 64 || !s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
            return Err(ContentHashError);
        }
        Ok(())
    }
}

impl TryFrom<&str> for ContentHash {
    type Error = ContentHashError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::validate(s)?;
        Ok(Self(s.to_string()))
    }
}

impl TryFrom<String> for ContentHash {
    type Error = ContentHashError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::validate(&s)?;
        Ok(Self(s))
    }
}

impl fmt::Debug for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ContentHash({})", self.0)
    }
}

impl fmt::Display for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_HEX32: &str = "0123456789abcdef0123456789abcdef";

    #[test]
    fn room_id_accepts_legacy_and_generated_ids() {
        assert!(ValidatedRoomId::try_from("stale-room").is_ok());
        assert!(ValidatedRoomId::try_from(VALID_HEX32).is_ok());
        assert!(ValidatedRoomId::try_from("A_b-c1").is_ok());
        assert!(ValidatedRoomId::try_from("a".repeat(64)).is_ok());
        assert_eq!(ValidatedRoomId::try_from("stale-room").unwrap().as_str(), "stale-room");
    }

    #[test]
    fn room_id_rejects_unsafe_inputs() {
        let rejects = ["..", "a/b", "a\\b", "/etc", "C:\\x", "\\\\unc\\x", "", "a b", "房间"];
        for input in rejects {
            assert!(ValidatedRoomId::try_from(input).is_err(), "expected reject: {input:?}");
        }
        assert!(ValidatedRoomId::try_from("a".repeat(65)).is_err());
    }

    #[test]
    fn rel_path_accepts_relative_files() {
        let ok = ["index.html", "assets/app.js", "a/b/c.txt", "a\\b\\c.txt", "a b.txt"];
        for input in ok {
            let p = ValidatedRelPath::try_from(input).unwrap_or_else(|e| panic!("expected accept {input:?}: {e}"));
            assert_eq!(p.as_str(), input.replace('\\', "/"), "normalized form of {input:?}");
        }
    }

    #[test]
    fn rel_path_rejects_escapes_and_absolutes() {
        let rejects = [
            "../x", "..\\x", "/abs", "C:\\abs", "\\\\unc", "a/./b", "a/../b", "", "a\0b",
        ];
        for input in rejects {
            assert!(ValidatedRelPath::try_from(input).is_err(), "expected reject: {input:?}");
        }
    }

    #[test]
    fn rel_path_rejects_control_characters() {
        for c in ['\n', '\t', '\x01', '\x1f', '\x7f'] {
            let input = format!("a{}b", c);
            assert!(
                ValidatedRelPath::try_from(input.as_str()).is_err(),
                "expected reject: {input:?}"
            );
        }
    }

    #[test]
    fn content_hash_accepts_exact_lowercase_hex() {
        let h = ContentHash::try_from("a".repeat(64)).unwrap();
        assert_eq!(h.as_str(), "a".repeat(64));
        assert!(ContentHash::try_from("0123456789abcdef".repeat(4)).is_ok());
    }

    #[test]
    fn content_hash_rejects_wrong_length_and_non_hex() {
        assert!(ContentHash::try_from("a".repeat(63)).is_err());
        assert!(ContentHash::try_from("a".repeat(65)).is_err());
        assert!(ContentHash::try_from("a".repeat(63) + "g").is_err());
        assert!(ContentHash::try_from("A".repeat(64)).is_err());
        assert!(ContentHash::try_from("").is_err());
    }

    #[test]
    fn room_id_error_kinds_are_precise() {
        assert_eq!(ValidatedRoomId::try_from("").unwrap_err(), RoomIdError::Empty);
        assert_eq!(
            ValidatedRoomId::try_from("a".repeat(65)).unwrap_err(),
            RoomIdError::TooLong
        );
        assert_eq!(
            ValidatedRoomId::try_from("房间").unwrap_err(),
            RoomIdError::InvalidCharacter
        );
    }
}
