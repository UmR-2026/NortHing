//! Async bridging helper for synchronous event handling.
use std::future::Future;

/// Bridge a future onto the current tokio runtime from a synchronous context.
#[inline]
pub(crate) fn bridge<F, T>(rt_handle: &tokio::runtime::Handle, fut: F) -> T
where
    F: Future<Output = T>,
{
    tokio::task::block_in_place(|| rt_handle.block_on(fut))
}
