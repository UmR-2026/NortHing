use northhing_disposable::{DisposableList, DisposableListError, DisposalGuard};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

#[test]
fn test_lifo_reverse_order() {
    let sequence = Arc::new(Mutex::new(Vec::new()));
    let mut list = DisposableList::new();

    let s1 = Arc::clone(&sequence);
    let _g1 = list
        .push(Box::new(move || {
            s1.lock().unwrap().push(1);
        }))
        .unwrap();

    let s2 = Arc::clone(&sequence);
    let _g2 = list
        .push(Box::new(move || {
            s2.lock().unwrap().push(2);
        }))
        .unwrap();

    let s3 = Arc::clone(&sequence);
    let _g3 = list
        .push(Box::new(move || {
            s3.lock().unwrap().push(3);
        }))
        .unwrap();

    assert_eq!(list.len(), 3);
    list.dispose();

    assert!(list.is_disposed());
    assert_eq!(list.len(), 0);
    assert_eq!(*sequence.lock().unwrap(), vec![3, 2, 1]);
}

#[test]
fn test_idempotent_guard_drop() {
    let count = Arc::new(AtomicUsize::new(0));
    let c = Arc::clone(&count);

    let mut guard = DisposalGuard::new(Box::new(move || {
        c.fetch_add(1, Ordering::SeqCst);
    }));

    assert!(!guard.is_disposed());
    guard.dispose();
    assert!(guard.is_disposed());
    assert_eq!(count.load(Ordering::SeqCst), 1);

    // Second explicit dispose is a no-op
    guard.dispose();
    assert_eq!(count.load(Ordering::SeqCst), 1);

    // Drop is also a no-op
    drop(guard);
    assert_eq!(count.load(Ordering::SeqCst), 1);
}

#[test]
fn test_push_after_dispose_err() {
    let mut list = DisposableList::new();
    list.dispose();
    assert!(list.is_disposed());

    let res = list.push(Box::new(|| {}));
    assert_eq!(res.unwrap_err(), DisposableListError::Disposed);
}

#[test]
fn test_guard_early_drop_not_reexecuted_in_list() {
    let sequence = Arc::new(Mutex::new(Vec::new()));
    let mut list = DisposableList::new();

    let s1 = Arc::clone(&sequence);
    let _g1 = list
        .push(Box::new(move || {
            s1.lock().unwrap().push(1);
        }))
        .unwrap();

    let s2 = Arc::clone(&sequence);
    let g2 = list
        .push(Box::new(move || {
            s2.lock().unwrap().push(2);
        }))
        .unwrap();

    let s3 = Arc::clone(&sequence);
    let _g3 = list
        .push(Box::new(move || {
            s3.lock().unwrap().push(3);
        }))
        .unwrap();

    assert_eq!(list.len(), 3);

    // Early drop g2 -> executes 2 immediately
    drop(g2);
    assert_eq!(*sequence.lock().unwrap(), vec![2]);
    assert_eq!(list.len(), 2);

    // Now dispose list -> should execute 3 then 1, without repeating 2
    list.dispose();
    assert_eq!(*sequence.lock().unwrap(), vec![2, 3, 1]);
}

#[test]
fn test_list_drop_auto_disposes() {
    let executed = Arc::new(AtomicUsize::new(0));
    let mut list = DisposableList::new();

    let e1 = Arc::clone(&executed);
    let _g1 = list
        .push(Box::new(move || {
            e1.fetch_add(1, Ordering::SeqCst);
        }))
        .unwrap();

    let e2 = Arc::clone(&executed);
    let _g2 = list
        .push(Box::new(move || {
            e2.fetch_add(1, Ordering::SeqCst);
        }))
        .unwrap();

    drop(list);
    assert_eq!(executed.load(Ordering::SeqCst), 2);
}

#[test]
fn test_standalone_guard_disarm() {
    let executed = Arc::new(AtomicUsize::new(0));
    let e = Arc::clone(&executed);

    let guard = DisposalGuard::new(Box::new(move || {
        e.fetch_add(1, Ordering::SeqCst);
    }));

    guard.disarm();
    assert_eq!(executed.load(Ordering::SeqCst), 0);
}

#[test]
fn test_concurrent_disposal_thread_safety() {
    let count = Arc::new(AtomicUsize::new(0));
    let c = Arc::clone(&count);

    let guard = Arc::new(Mutex::new(DisposalGuard::new(Box::new(move || {
        c.fetch_add(1, Ordering::SeqCst);
    }))));

    let handles: Vec<_> = (0..8)
        .map(|_| {
            let g = Arc::clone(&guard);
            std::thread::spawn(move || {
                let mut guard = g.lock().unwrap();
                guard.dispose();
            })
        })
        .collect();

    for h in handles {
        h.join().unwrap();
    }

    assert_eq!(count.load(Ordering::SeqCst), 1);
}

#[test]
fn test_lock_poisoning_safety() {
    let count = Arc::new(AtomicUsize::new(0));
    let c = Arc::clone(&count);

    let mut list = DisposableList::new();
    let _g = list
        .push(Box::new(move || {
            c.fetch_add(1, Ordering::SeqCst);
        }))
        .unwrap();

    // Poison list inner lock in another thread
    list.__test_poison_list_lock();

    // Must not panic on poisoned lock
    assert!(!list.is_disposed());
    list.dispose();
    assert!(list.is_disposed());
    assert_eq!(count.load(Ordering::SeqCst), 1);
}
