//! Single instance process lock for northhing Desktop (P2-2).
//!
//! Uses a Windows named mutex in the `Local\` session namespace to ensure only
//! one instance of the desktop application runs per user session.
//! Non-Windows platforms use a no-op stub that always succeeds.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SingleInstanceError {
    AlreadyRunning,
    CreationFailed(u32),
    InvalidName,
}

impl std::fmt::Display for SingleInstanceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyRunning => write!(f, "another instance is already running in this session"),
            Self::CreationFailed(code) => {
                write!(f, "failed to create single instance mutex: win32 error {code}")
            }
            Self::InvalidName => write!(f, "single instance mutex name contains an interior null character"),
        }
    }
}

impl std::error::Error for SingleInstanceError {}

pub const DEFAULT_MUTEX_NAME: &str = "Local\\NorthHingDesktopSingleton";

/// Attempts to acquire the default session-scoped single instance lock.
pub fn try_acquire() -> Result<SingleInstanceGuard, SingleInstanceError> {
    try_acquire_named(DEFAULT_MUTEX_NAME)
}

#[cfg(target_os = "windows")]
mod win {
    use std::ffi::c_void;

    pub const ERROR_ALREADY_EXISTS: u32 = 183;

    unsafe extern "system" {
        pub fn CreateMutexW(lp_mutex_attributes: *mut c_void, b_initial_owner: i32, lp_name: *const u16)
            -> *mut c_void;

        pub fn GetLastError() -> u32;

        pub fn SetLastError(dw_err_code: u32);

        pub fn CloseHandle(h_object: *mut c_void) -> i32;
    }
}

#[cfg(target_os = "windows")]
#[derive(Debug)]
pub struct SingleInstanceGuard {
    handle: *mut std::ffi::c_void,
}

#[cfg(target_os = "windows")]
// SAFETY: SingleInstanceGuard uniquely owns the Win32 mutex HANDLE. The handle
// can be transferred to another thread or dropped safely from any thread.
unsafe impl Send for SingleInstanceGuard {}

#[cfg(target_os = "windows")]
// SAFETY: Drop requires &mut self, so &shared access cannot trigger CloseHandle;
// the handle field has no accessor via &self, so Sync cannot cause aliasing of
// the raw pointer.
unsafe impl Sync for SingleInstanceGuard {}

#[cfg(target_os = "windows")]
impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            // SAFETY: self.handle is a valid non-null handle opened by CreateMutexW
            // and owned solely by this SingleInstanceGuard.
            unsafe {
                win::CloseHandle(self.handle);
            }
            self.handle = std::ptr::null_mut();
        }
    }
}

#[cfg(target_os = "windows")]
pub fn try_acquire_named(name: &str) -> Result<SingleInstanceGuard, SingleInstanceError> {
    if name.contains('\0') {
        return Err(SingleInstanceError::InvalidName);
    }

    let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();

    // SAFETY: SetLastError has no preconditions and clears thread-local error state
    // so any subsequent check on GetLastError reflects only CreateMutexW.
    unsafe {
        win::SetLastError(0);
    }

    // SAFETY: lp_mutex_attributes is null (default security), b_initial_owner is 0
    // (we do not request lock ownership, only existence token), and wide is a valid
    // null-terminated UTF-16 buffer.
    let handle = unsafe { win::CreateMutexW(std::ptr::null_mut(), 0, wide.as_ptr()) };

    if handle.is_null() {
        // SAFETY: GetLastError has no preconditions.
        let err = unsafe { win::GetLastError() };
        return Err(SingleInstanceError::CreationFailed(err));
    }

    // SAFETY: GetLastError has no preconditions.
    let last_error = unsafe { win::GetLastError() };

    if last_error == win::ERROR_ALREADY_EXISTS {
        // SAFETY: CreateMutexW returned an opened handle to an existing mutex.
        // We close the handle immediately so we do not retain a reference to it.
        unsafe {
            win::CloseHandle(handle);
        }
        return Err(SingleInstanceError::AlreadyRunning);
    }

    Ok(SingleInstanceGuard { handle })
}

#[cfg(not(target_os = "windows"))]
#[derive(Debug)]
pub struct SingleInstanceGuard;

#[cfg(not(target_os = "windows"))]
pub fn try_acquire_named(_name: &str) -> Result<SingleInstanceGuard, SingleInstanceError> {
    Ok(SingleInstanceGuard)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "windows")]
    fn test_second_acquire_fails() {
        let name = format!("Local\\NorthHingTest_SecondAcquireFails_{}", std::process::id());
        let guard1 = try_acquire_named(&name).expect("first acquire should succeed");
        let result2 = try_acquire_named(&name);
        assert_eq!(
            result2.err(),
            Some(SingleInstanceError::AlreadyRunning),
            "second acquire of the same mutex must fail with AlreadyRunning"
        );
        drop(guard1);
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_acquire_after_drop_succeeds() {
        let name = format!("Local\\NorthHingTest_AcquireAfterDrop_{}", std::process::id());
        let guard1 = try_acquire_named(&name).expect("first acquire should succeed");
        assert_eq!(
            try_acquire_named(&name).err(),
            Some(SingleInstanceError::AlreadyRunning)
        );
        drop(guard1);

        let guard2 = try_acquire_named(&name).expect("acquire after drop should succeed");
        drop(guard2);
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_invalid_name_rejected() {
        let name = "Local\\NorthHingTest\0Invalid";
        let result = try_acquire_named(name);
        assert_eq!(result.err(), Some(SingleInstanceError::InvalidName));
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn test_non_windows_stub_always_succeeds() {
        let name = "test_stub_mutex";
        let guard1 = try_acquire_named(name).expect("non-windows stub must succeed");
        let guard2 = try_acquire_named(name).expect("non-windows stub must succeed repeatedly");
        drop(guard1);
        drop(guard2);
    }
}
