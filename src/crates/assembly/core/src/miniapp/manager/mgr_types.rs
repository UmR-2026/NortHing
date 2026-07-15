//! Error mapping helpers shared across the [`super`] siblings.
//!
//! Translates between [`NortHingError`] and the `MiniAppPortError` contract type, plus a
//! small prefix-stripping helper that keeps downstream error messages consistent
//! regardless of which side produced them.

use crate::util::errors::NortHingError;
use northhing_product_domains::miniapp::ports::{MiniAppPortError, MiniAppPortErrorKind};

/// Convert a [`NortHingError`] into the miniapp-port error contract type.
pub(super) fn map_northhing_error_to_miniapp_port_error(error: NortHingError) -> MiniAppPortError {
    let kind = match &error {
        NortHingError::NotFound(_) => MiniAppPortErrorKind::NotFound,
        NortHingError::Validation(_) => MiniAppPortErrorKind::InvalidInput,
        NortHingError::Deserialization(_) | NortHingError::Serialization(_) => MiniAppPortErrorKind::Deserialization,
        NortHingError::Io(io_error) if io_error.kind() == std::io::ErrorKind::PermissionDenied => {
            MiniAppPortErrorKind::PermissionDenied
        }
        NortHingError::Io(_) => MiniAppPortErrorKind::Io,
        NortHingError::ProcessError(_) => MiniAppPortErrorKind::RuntimeUnavailable,
        _ => MiniAppPortErrorKind::Backend,
    };
    MiniAppPortError::new(kind, error.to_string())
}

/// Convert a miniapp-port error back into a [`NortHingError`].
pub(super) fn map_miniapp_port_error(error: MiniAppPortError) -> NortHingError {
    let message = strip_northhing_error_prefix(error.message);
    match error.kind {
        MiniAppPortErrorKind::NotFound => NortHingError::NotFound(message),
        MiniAppPortErrorKind::InvalidInput => NortHingError::validation(message),
        MiniAppPortErrorKind::Deserialization => NortHingError::parse(message),
        MiniAppPortErrorKind::PermissionDenied => {
            NortHingError::Io(std::io::Error::new(std::io::ErrorKind::PermissionDenied, message))
        }
        MiniAppPortErrorKind::RuntimeUnavailable => NortHingError::ProcessError(message),
        MiniAppPortErrorKind::Io => NortHingError::io(message),
        MiniAppPortErrorKind::Backend => NortHingError::service(message),
    }
}

/// Strip a leading `Not found: ` / `Validation error: ` / `IO error: ` / etc.
/// prefix so that downstream code sees the original message without
/// double-prefixing on the round-trip.
pub(super) fn strip_northhing_error_prefix(message: String) -> String {
    const PREFIXES: &[&str] = &[
        "Not found: ",
        "Validation error: ",
        "Deserialization error: ",
        "IO error: ",
        "Process error: ",
        "Service error: ",
    ];

    for prefix in PREFIXES {
        if let Some(stripped) = message.strip_prefix(prefix) {
            return stripped.to_string();
        }
    }
    message
}
