//! Reversible registration and disposable RAII primitives.
//!
//! Provides `Disposable`, `DisposableList`, and `DisposalGuard` for managing
//! reversible side effects, resource lifecycles, and cleanup in LIFO order.

use std::fmt;
use std::sync::{Arc, Mutex, Weak};

/// A one-shot cleanup callback.
pub type Disposable = Box<dyn FnOnce() + Send + 'static>;

/// Error returned when attempting an operation on an already disposed list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisposableListError {
    /// The disposable list has already been disposed.
    Disposed,
}

impl fmt::Display for DisposableListError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disposed => write!(f, "disposable list has already been disposed"),
        }
    }
}

impl std::error::Error for DisposableListError {}

struct DisposableEntry {
    id: u64,
    cell: Arc<Mutex<Option<Disposable>>>,
}

struct DisposableListInner {
    disposed: bool,
    next_id: u64,
    items: Vec<DisposableEntry>,
}

fn lock_inner(inner: &Mutex<DisposableListInner>) -> std::sync::MutexGuard<'_, DisposableListInner> {
    match inner.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn lock_cell(cell: &Mutex<Option<Disposable>>) -> std::sync::MutexGuard<'_, Option<Disposable>> {
    match cell.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// An ordered stack of disposable items that are unwound in reverse (LIFO) order.
pub struct DisposableList {
    inner: Arc<Mutex<DisposableListInner>>,
}

impl fmt::Debug for DisposableList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let inner = lock_inner(&self.inner);
        f.debug_struct("DisposableList")
            .field("disposed", &inner.disposed)
            .field("item_count", &inner.items.len())
            .finish()
    }
}

impl Default for DisposableList {
    fn default() -> Self {
        Self::new()
    }
}

impl DisposableList {
    /// Creates a new, empty disposable list.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(DisposableListInner {
                disposed: false,
                next_id: 1,
                items: Vec::new(),
            })),
        }
    }

    /// Registers a disposable callback and returns a guard.
    ///
    /// If the returned guard is dropped before the list is disposed, the callback
    /// is executed immediately and removed from the list.
    ///
    /// # Errors
    ///
    /// Returns `Err(DisposableListError::Disposed)` if this list has already been disposed.
    pub fn push(&mut self, d: Disposable) -> Result<DisposalGuard, DisposableListError> {
        let mut inner = lock_inner(&self.inner);
        if inner.disposed {
            return Err(DisposableListError::Disposed);
        }

        let id = inner.next_id;
        inner.next_id = inner.next_id.wrapping_add(1);

        let cell = Arc::new(Mutex::new(Some(d)));
        inner.items.push(DisposableEntry {
            id,
            cell: Arc::clone(&cell),
        });

        Ok(DisposalGuard {
            id,
            cell,
            list: Arc::downgrade(&self.inner),
        })
    }

    /// Drains and executes all registered disposables in reverse (LIFO) order,
    /// marking the list as disposed.
    pub fn dispose(&mut self) {
        let entries = {
            let mut inner = lock_inner(&self.inner);
            if inner.disposed {
                return;
            }
            inner.disposed = true;
            std::mem::take(&mut inner.items)
        };

        // Execute in reverse order (LIFO), releasing the list lock beforehand.
        for entry in entries.into_iter().rev() {
            let action = {
                let mut guard = lock_cell(&entry.cell);
                guard.take()
            };
            if let Some(d) = action {
                d();
            }
        }
    }

    /// Returns `true` if this list has already been disposed.
    pub fn is_disposed(&self) -> bool {
        lock_inner(&self.inner).disposed
    }

    /// Returns the number of active disposables remaining in the list.
    pub fn len(&self) -> usize {
        lock_inner(&self.inner).items.len()
    }

    /// Returns `true` if there are no active disposables remaining in the list.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[doc(hidden)]
    pub fn __test_poison_list_lock(&self) {
        let inner = Arc::clone(&self.inner);
        let _handle = std::thread::spawn(move || {
            let _lock = inner.lock();
            panic!("intentional poisoning for test");
        })
        .join();
    }
}

impl Drop for DisposableList {
    fn drop(&mut self) {
        self.dispose();
    }
}

/// An RAII guard representing a registered disposable action.
///
/// When dropped or explicitly disposed, the underlying cleanup action is executed
/// idempotently at most once.
pub struct DisposalGuard {
    id: u64,
    cell: Arc<Mutex<Option<Disposable>>>,
    list: Weak<Mutex<DisposableListInner>>,
}

impl fmt::Debug for DisposalGuard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let is_active = lock_cell(&self.cell).is_some();
        f.debug_struct("DisposalGuard")
            .field("id", &self.id)
            .field("is_active", &is_active)
            .finish()
    }
}

impl Default for DisposalGuard {
    fn default() -> Self {
        Self::noop()
    }
}

impl DisposalGuard {
    /// Creates a standalone guard that runs `d` upon drop or disposal.
    pub fn new(d: Disposable) -> Self {
        Self {
            id: 0,
            cell: Arc::new(Mutex::new(Some(d))),
            list: Weak::new(),
        }
    }

    /// Creates a no-op guard that performs no action upon drop.
    pub fn noop() -> Self {
        Self {
            id: 0,
            cell: Arc::new(Mutex::new(None)),
            list: Weak::new(),
        }
    }

    /// Explicitly executes the disposable action immediately (idempotent).
    pub fn dispose(&mut self) {
        let action = {
            let mut guard = lock_cell(&self.cell);
            guard.take()
        };

        if let Some(d) = action {
            if let Some(list_arc) = self.list.upgrade() {
                let mut list_inner = lock_inner(&list_arc);
                list_inner.items.retain(|item| item.id != self.id);
            }
            d();
        }
    }

    /// Disarms the guard, consuming it without running its cleanup action.
    pub fn disarm(self) {
        let mut guard = lock_cell(&self.cell);
        guard.take();
        if let Some(list_arc) = self.list.upgrade() {
            let mut list_inner = lock_inner(&list_arc);
            list_inner.items.retain(|item| item.id != self.id);
        }
    }

    /// Returns `true` if the disposable action has already been executed or disarmed.
    pub fn is_disposed(&self) -> bool {
        lock_cell(&self.cell).is_none()
    }
}

impl Drop for DisposalGuard {
    fn drop(&mut self) {
        self.dispose();
    }
}
