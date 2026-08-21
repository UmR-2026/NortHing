//! Shared time helper functions.
//!
//! Provides lightweight timestamp retrieval based on [`SystemTime::now`]
//! relative to [`UNIX_EPOCH`].

use std::time::{SystemTime, UNIX_EPOCH};

/// Returns the current Unix timestamp in milliseconds as `u64`.
///
/// If the system clock is set before [`UNIX_EPOCH`], this returns `0`.
/// If the millisecond representation exceeds [`u64::MAX`], it saturates at [`u64::MAX`].
#[inline]
pub fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

/// Returns the current Unix timestamp in milliseconds as `i64`.
///
/// If the system clock is set before [`UNIX_EPOCH`], this returns `0`.
/// If the millisecond representation exceeds [`i64::MAX`], it saturates at [`i64::MAX`].
#[inline]
pub fn now_unix_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_now_unix_ms_positive_and_monotonic() {
        let t1 = now_unix_ms();
        assert!(t1 > 0);
        let t2 = now_unix_ms();
        assert!(t2 >= t1);
    }

    #[test]
    fn test_now_unix_millis_positive_and_monotonic() {
        let t1 = now_unix_millis();
        assert!(t1 > 0);
        let t2 = now_unix_millis();
        assert!(t2 >= t1);
    }
}
