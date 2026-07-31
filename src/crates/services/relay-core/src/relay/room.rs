//! Room management for the relay server.
//!
//! NOTE: The second implementation that used to live at
//! `src/apps/relay-server/src/relay/room.rs` was merged into this crate
//! during the validated-path refactor (Task 1); this is now the single
//! implementation shared by the standalone relay and the embedded relay.
//! The former v0.1.0 surface-freeze note no longer applies — audit-driven
//! security hardening (Task 2) is applied here.
//!
//! Each room holds a single desktop participant connected via WebSocket.
//! Mobile clients interact through HTTP requests that the relay bridges
//! to the desktop via the WebSocket connection. The relay stores no
//! business data — it only routes messages.

use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use tracing::{info, warn};

pub type ConnId = u64;

/// Hard cap on concurrent WebSocket connections (audit H-2, resource
/// exhaustion). Enforced by [`RoomManager::try_acquire_connection`] before
/// a socket upgrade is accepted.
const MAX_CONNECTIONS: usize = 512;

/// A desktop connection whose last heartbeat is older than this is treated
/// as stale ("zombie"), so a new connection may take over its room.
/// The desktop client heartbeats every 30s; 90s means 3 missed beats.
/// Aligned with the WebSocket idle timeout (90s) in `routes/websocket.rs`.
const STALE_DESKTOP_AFTER_SECS: i64 = 90;

#[derive(Debug, Clone)]
pub struct OutboundMessage {
    pub text: String,
}

/// Payload returned by the desktop in response to a bridged HTTP request.
#[derive(Debug, Clone)]
pub struct ResponsePayload {
    pub encrypted_data: String,
    pub nonce: String,
}

#[derive(Debug)]
pub struct DesktopConnection {
    pub conn_id: ConnId,
    // reason: device_id is held for upcoming device-identification API (audit log, ban-list); not yet routed
    #[allow(dead_code)]
    pub device_id: String,
    // reason: public_key is held for the upcoming end-to-end key-exchange protocol; not yet exchanged
    #[allow(dead_code)]
    pub public_key: String,
    /// Bounded outbound queue to this desktop's WebSocket. `try_send`
    /// failures (queue full / channel closed) mean a slow consumer — the
    /// sender disconnects instead of blocking (see
    /// `routes/websocket.rs`).
    pub tx: mpsc::Sender<OutboundMessage>,
    // reason: joined_at is held for upcoming analytics/audit surface; not yet queried
    #[allow(dead_code)]
    pub joined_at: i64,
    pub last_heartbeat: i64,
}

#[derive(Debug)]
pub struct RelayRoom {
    pub room_id: String,
    // reason: created_at is held for upcoming analytics/audit surface (TTL uses last_activity instead)
    #[allow(dead_code)]
    pub created_at: i64,
    pub last_activity: i64,
    pub desktop: Option<DesktopConnection>,
}

impl RelayRoom {
    pub fn new(room_id: String) -> Self {
        let now = Utc::now().timestamp();
        Self {
            room_id,
            created_at: now,
            last_activity: now,
            desktop: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.desktop.is_none()
    }

    pub fn touch(&mut self) {
        self.last_activity = Utc::now().timestamp();
    }
}

/// Result of [`RoomManager::create_room`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreateRoomOutcome {
    /// The room was created, or an existing room with a disconnected /
    /// stale desktop was taken over (legitimate reconnect path). The
    /// caller should reply `RoomCreated`.
    Created,
    /// The room already exists with an *active* desktop. The existing room
    /// and its desktop connection are untouched. The caller should reply
    /// `Error { message: "room already exists" }`.
    Conflict,
}

pub struct RoomManager {
    rooms: DashMap<String, RelayRoom>,
    conn_to_room: DashMap<ConnId, String>,
    next_conn_id: std::sync::atomic::AtomicU64,
    pending_requests: DashMap<String, oneshot::Sender<ResponsePayload>>,
    /// Live WebSocket connections currently admitted (handshake accepted).
    active_connections: std::sync::atomic::AtomicUsize,
}

impl RoomManager {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            rooms: DashMap::new(),
            conn_to_room: DashMap::new(),
            next_conn_id: std::sync::atomic::AtomicU64::new(1),
            pending_requests: DashMap::new(),
            active_connections: std::sync::atomic::AtomicUsize::new(0),
        })
    }

    pub fn next_conn_id(&self) -> ConnId {
        self.next_conn_id.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    /// Try to admit a new WebSocket connection, enforcing [`MAX_CONNECTIONS`].
    /// The caller must pair a success with exactly one
    /// [`RoomManager::release_connection`] on teardown.
    pub fn try_acquire_connection(&self) -> bool {
        let previous = self
            .active_connections
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if previous >= MAX_CONNECTIONS {
            self.active_connections
                .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
            false
        } else {
            true
        }
    }

    /// Release a connection slot previously acquired by
    /// [`RoomManager::try_acquire_connection`].
    pub fn release_connection(&self) {
        self.active_connections
            .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Number of currently admitted WebSocket connections.
    pub fn active_connection_count(&self) -> usize {
        self.active_connections.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Create or take over a room for a desktop connection, atomically.
    ///
    /// The whole decision runs inside a single DashMap `entry` operation
    /// (one shard lock), so a concurrent `create_room` for the same
    /// `room_id` cannot interleave between the existence check and the
    /// insertion:
    ///
    /// 1. Room absent → insert, return [`CreateRoomOutcome::Created`].
    /// 2. Room present with an *active* desktop → return
    ///    [`CreateRoomOutcome::Conflict`]; the existing room and its
    ///    desktop connection are never removed or disconnected.
    /// 3. Room present with a disconnected or stale desktop → replace the
    ///    desktop entry (legitimate reconnect takeover), return
    ///    [`CreateRoomOutcome::Created`].
    ///
    /// Liveness of the registered desktop: `desktop.is_some()` AND the
    /// last heartbeat is fresher than [`STALE_DESKTOP_AFTER_SECS`]. Both
    /// flags live inside the `rooms` entry, so the check needs no second
    /// map lookup while the shard lock is held.
    ///
    /// A client-chosen `room_id` is never refused outright: the takeover
    /// branch keeps the desktop reconnect flow working (the desktop client
    /// re-creates its room with the original id after a disconnect).
    pub fn create_room(
        &self,
        room_id: &str,
        conn_id: ConnId,
        device_id: &str,
        public_key: &str,
        tx: mpsc::Sender<OutboundMessage>,
    ) -> CreateRoomOutcome {
        // Unregister this conn's previous room registration, if any
        // (a connection re-issuing CreateRoom with a different id).
        if let Some((_, old_room_id)) = self.conn_to_room.remove(&conn_id) {
            let should_remove = if let Some(mut room) = self.rooms.get_mut(&old_room_id) {
                room.desktop = None;
                room.is_empty()
            } else {
                false
            };
            if should_remove {
                self.rooms.remove(&old_room_id);
            }
        }

        match self.rooms.entry(room_id.to_string()) {
            dashmap::mapref::entry::Entry::Vacant(vacant) => {
                let now = Utc::now().timestamp();
                let mut room = RelayRoom::new(room_id.to_string());
                room.desktop = Some(DesktopConnection {
                    conn_id,
                    device_id: device_id.to_string(),
                    public_key: public_key.to_string(),
                    tx,
                    joined_at: now,
                    last_heartbeat: now,
                });
                // Keep the shard lock (via `_room_ref`) while the conn→room
                // index is updated, so both maps mutate inside one atomic
                // decision window.
                let _room_ref = vacant.insert(room);
                self.conn_to_room.insert(conn_id, room_id.to_string());
                info!("Room {room_id} created by desktop {device_id} (conn {conn_id})");
                CreateRoomOutcome::Created
            }
            dashmap::mapref::entry::Entry::Occupied(mut occupied) => {
                let room = occupied.get_mut();
                let now = Utc::now().timestamp();
                let desktop_alive = room
                    .desktop
                    .as_ref()
                    .is_some_and(|d| now - d.last_heartbeat < STALE_DESKTOP_AFTER_SECS);
                if desktop_alive {
                    let existing_conn = room.desktop.as_ref().map(|d| d.conn_id).unwrap_or(0);
                    warn!(
                        "Room {room_id} already exists with active desktop conn {existing_conn}; rejecting create from conn {conn_id}"
                    );
                    return CreateRoomOutcome::Conflict;
                }
                // Take over: previous desktop is disconnected (tombstone)
                // or stale (missed heartbeats).
                room.desktop = Some(DesktopConnection {
                    conn_id,
                    device_id: device_id.to_string(),
                    public_key: public_key.to_string(),
                    tx,
                    joined_at: now,
                    last_heartbeat: now,
                });
                room.last_activity = now;
                self.conn_to_room.insert(conn_id, room_id.to_string());
                info!("Room {room_id} taken over by desktop {device_id} (conn {conn_id}): previous desktop disconnected or stale");
                CreateRoomOutcome::Created
            }
        }
    }

    /// Bridge a message from a mobile HTTP request to the room's desktop.
    /// Returns `false` when the room has no desktop or the bounded outbound
    /// queue is full / closed (slow consumer); callers map this to 503.
    /// Never blocks: `try_send` is used.
    pub fn send_to_desktop(&self, room_id: &str, message: &str) -> bool {
        if let Some(mut room) = self.rooms.get_mut(room_id) {
            room.touch();
            match &room.desktop {
                Some(desktop) => match desktop.tx.try_send(OutboundMessage {
                    text: message.to_string(),
                }) {
                    Ok(()) => true,
                    Err(e) => {
                        warn!("Room {room_id}: desktop outbound queue full or closed, dropping message ({e})");
                        false
                    }
                },
                None => false,
            }
        } else {
            false
        }
    }

    // reason: get_desktop_public_key() is reserved for the upcoming key-exchange protocol; today clients exchange keys directly via WebSocket frames
    #[allow(dead_code)]
    pub fn get_desktop_public_key(&self, room_id: &str) -> Option<String> {
        self.rooms
            .get(room_id)
            .and_then(|r| r.desktop.as_ref().map(|d| d.public_key.clone()))
    }

    pub fn register_pending(&self, correlation_id: String) -> oneshot::Receiver<ResponsePayload> {
        let (tx, rx) = oneshot::channel();
        self.pending_requests.insert(correlation_id, tx);
        rx
    }

    pub fn resolve_pending(&self, correlation_id: &str, payload: ResponsePayload) -> bool {
        if let Some((_, tx)) = self.pending_requests.remove(correlation_id) {
            tx.send(payload).is_ok()
        } else {
            warn!("No pending request for correlation_id={correlation_id}");
            false
        }
    }

    pub fn cancel_pending(&self, correlation_id: &str) {
        self.pending_requests.remove(correlation_id);
    }

    /// Mark a connection as gone. The room itself is kept as a tombstone
    /// (`desktop = None`) so a reconnecting desktop can take it over via
    /// [`RoomManager::create_room`]; the TTL sweep
    /// ([`RoomManager::cleanup_stale_rooms`]) removes tombstones later.
    pub fn on_disconnect(&self, conn_id: ConnId) {
        if let Some((_, room_id)) = self.conn_to_room.remove(&conn_id) {
            if let Some(mut room) = self.rooms.get_mut(&room_id) {
                if room.desktop.as_ref().is_some_and(|d| d.conn_id == conn_id) {
                    info!("Desktop disconnected from room {room_id}");
                    room.desktop = None;
                }
            }
        }
    }

    pub fn heartbeat(&self, conn_id: ConnId) -> bool {
        // Copy the room id out of the index first: holding the conn_to_room
        // shard guard while acquiring a `rooms` shard lock would invert the
        // lock order used by `create_room` (rooms → conn_to_room) and could
        // deadlock under contention.
        let room_id = match self.conn_to_room.get(&conn_id) {
            Some(registered) => registered.value().clone(),
            None => return false,
        };
        if let Some(mut room) = self.rooms.get_mut(&room_id) {
            let is_match = room.desktop.as_ref().is_some_and(|d| d.conn_id == conn_id);
            if is_match {
                let now = Utc::now().timestamp();
                room.last_activity = now;
                if let Some(ref mut desktop) = room.desktop {
                    desktop.last_heartbeat = now;
                }
                return true;
            }
        }
        false
    }

    pub fn cleanup_stale_rooms(&self, ttl_secs: u64) -> Vec<String> {
        let now = Utc::now().timestamp();
        let mut stale_room_ids: Vec<String> = Vec::new();
        let mut stale_conn_ids: Vec<ConnId> = Vec::new();

        // Pass 1: walk `rooms` once, removing stale entries in place
        // via `retain`. Doing the `conn_to_room.remove()` AFTER this
        // pass (rather than inside the closure) prevents the previous
        // implementation's cross-shard lock contention — DashMap's
        // `retain` holds the shard lock for the duration of the closure,
        // so calling `conn_to_room.remove()` from inside could collide
        // with a shard that has already been locked by `retain`'s
        // iteration, depending on hash distribution. Doing it as a
        // second pass over `conn_to_room` is both cheaper (single shard
        // lock) and panic-free (no nested DashMap access).
        //
        // Review: `CODE_REVIEW_2026-06-26.md` §"Relay Server 的
        // cleanup_stale_rooms 存在迭代-修改竞义".
        self.rooms.retain(|room_id, room| {
            let is_stale = (now - room.last_activity) as u64 > ttl_secs;
            if is_stale {
                stale_room_ids.push(room_id.clone());
                if let Some(ref desktop) = room.desktop {
                    stale_conn_ids.push(desktop.conn_id);
                }
                info!("Stale room {room_id} cleaned up");
            }
            !is_stale
        });

        // Pass 2: clean up the conn→room index now that no shard lock
        // is held. Independent of `rooms`'s shard distribution.
        for conn_id in &stale_conn_ids {
            self.conn_to_room.remove(conn_id);
        }

        stale_room_ids
    }

    pub fn room_exists(&self, room_id: &str) -> bool {
        self.rooms.contains_key(room_id)
    }

    pub fn has_desktop(&self, room_id: &str) -> bool {
        self.rooms.get(room_id).is_some_and(|r| r.desktop.is_some())
    }

    pub fn room_count(&self) -> usize {
        self.rooms.len()
    }

    pub fn connection_count(&self) -> usize {
        self.conn_to_room.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    /// Create a bounded outbound channel like `handle_socket` does.
    fn make_conn(manager: &RoomManager) -> (ConnId, mpsc::Sender<OutboundMessage>) {
        let conn_id = manager.next_conn_id();
        let (tx, _rx) = mpsc::channel(256);
        (conn_id, tx)
    }

    /// `cleanup_stale_rooms` should remove rooms whose `last_activity` is
    /// older than the TTL, AND clean up their `conn_to_room` index
    /// entries. The previous implementation only cleaned `rooms` inside
    /// the `for room_id in &stale_ids` loop — `conn_to_room` got out of
    /// sync if a stale room still had a desktop connection attached.
    ///
    /// Review: `CODE_REVIEW_2026-06-26.md` §"`cleanup_stale_rooms`
    /// 缺少测试" + §"迭代-修改竞义".
    #[test]
    fn cleanup_stale_rooms_removes_room_and_conn_index() {
        let manager = RoomManager::new();

        // Set up a stale room (last_activity far in the past) and a
        // fresh room. We construct the stale one by hand because the
        // public constructor always uses Utc::now().
        let (conn_id_stale, tx_stale) = make_conn(&manager);
        manager.create_room("stale-room", conn_id_stale, "device-stale", "pk-stale", tx_stale);
        if let Some(mut room) = manager.rooms.get_mut("stale-room") {
            room.last_activity = Utc::now().timestamp() - 10_000;
        }

        let (conn_id_fresh, tx_fresh) = make_conn(&manager);
        manager.create_room("fresh-room", conn_id_fresh, "device-fresh", "pk-fresh", tx_fresh);

        assert_eq!(manager.room_count(), 2);
        assert_eq!(manager.connection_count(), 2);

        // TTL of 60s: stale-room is ~10000s old, fresh-room is "now".
        let removed = manager.cleanup_stale_rooms(60);

        assert_eq!(removed, vec!["stale-room".to_string()]);
        assert!(!manager.room_exists("stale-room"));
        assert!(manager.room_exists("fresh-room"));
        // `conn_to_room` index for the stale room's desktop is gone.
        assert!(!manager.conn_to_room.contains_key(&conn_id_stale));
        // `conn_to_room` for the fresh room is still there.
        assert!(manager.conn_to_room.contains_key(&conn_id_fresh));
    }

    /// `cleanup_stale_rooms` on an empty manager is a no-op.
    #[test]
    fn cleanup_stale_rooms_empty_manager() {
        let manager = RoomManager::new();
        let removed = manager.cleanup_stale_rooms(60);
        assert!(removed.is_empty());
        assert_eq!(manager.room_count(), 0);
        assert_eq!(manager.connection_count(), 0);
    }

    // ── create_room three-state semantics (H-1) ────────────────────────

    /// Fresh room: first create returns Created and registers the desktop.
    #[test]
    fn create_room_fresh_room_succeeds() {
        let manager = RoomManager::new();
        let (conn_id, tx) = make_conn(&manager);
        assert_eq!(
            manager.create_room("room-a", conn_id, "dev-a", "pk-a", tx),
            CreateRoomOutcome::Created
        );
        assert!(manager.room_exists("room-a"));
        assert!(manager.has_desktop("room-a"));
        assert_eq!(
            manager.rooms.get("room-a").unwrap().desktop.as_ref().unwrap().conn_id,
            conn_id
        );
        assert_eq!(manager.conn_to_room.get(&conn_id).unwrap().value(), "room-a");
    }

    /// Active desktop: a second create with the same id is rejected, and
    /// neither the original room nor its desktop connection is disturbed.
    #[test]
    fn create_room_conflict_keeps_original_room_and_desktop() {
        let manager = RoomManager::new();
        let (conn_orig, tx_orig) = make_conn(&manager);
        assert_eq!(
            manager.create_room("room-b", conn_orig, "dev-orig", "pk", tx_orig),
            CreateRoomOutcome::Created
        );

        let (conn_intruder, tx_intruder) = make_conn(&manager);
        assert_eq!(
            manager.create_room("room-b", conn_intruder, "dev-intruder", "pk", tx_intruder),
            CreateRoomOutcome::Conflict
        );

        // Original desktop untouched.
        let room = manager.rooms.get("room-b").unwrap();
        assert_eq!(room.desktop.as_ref().unwrap().conn_id, conn_orig);
        drop(room);
        // Intruder is not registered anywhere.
        assert!(!manager.conn_to_room.contains_key(&conn_intruder));
        assert!(manager.conn_to_room.contains_key(&conn_orig));
    }

    /// Disconnected desktop: after `on_disconnect` the room persists as a
    /// tombstone, and a new create with the same id takes it over.
    #[test]
    fn create_room_takes_over_after_disconnect() {
        let manager = RoomManager::new();
        let (conn_old, tx_old) = make_conn(&manager);
        assert_eq!(
            manager.create_room("room-c", conn_old, "dev-old", "pk", tx_old),
            CreateRoomOutcome::Created
        );

        manager.on_disconnect(conn_old);
        // Tombstone: room still exists, no desktop.
        assert!(manager.room_exists("room-c"));
        assert!(!manager.has_desktop("room-c"));
        assert_eq!(manager.connection_count(), 0);

        // Reconnect with the same id (desktop client reconnect path).
        let (conn_new, tx_new) = make_conn(&manager);
        assert_eq!(
            manager.create_room("room-c", conn_new, "dev-new", "pk", tx_new),
            CreateRoomOutcome::Created
        );
        assert_eq!(
            manager.rooms.get("room-c").unwrap().desktop.as_ref().unwrap().conn_id,
            conn_new
        );
        assert_eq!(manager.conn_to_room.get(&conn_new).unwrap().value(), "room-c");
    }

    /// Stale desktop (zombie connection, missed heartbeats): takeover is
    /// allowed without waiting for the idle timeout.
    #[test]
    fn create_room_takes_over_stale_heartbeat_connection() {
        let manager = RoomManager::new();
        let (conn_zombie, tx_zombie) = make_conn(&manager);
        assert_eq!(
            manager.create_room("room-d", conn_zombie, "dev-zombie", "pk", tx_zombie),
            CreateRoomOutcome::Created
        );
        // Simulate a zombie: no heartbeat for longer than the staleness window.
        if let Some(mut room) = manager.rooms.get_mut("room-d") {
            room.desktop.as_mut().unwrap().last_heartbeat = Utc::now().timestamp() - STALE_DESKTOP_AFTER_SECS - 10;
        }

        let (conn_new, tx_new) = make_conn(&manager);
        assert_eq!(
            manager.create_room("room-d", conn_new, "dev-new", "pk", tx_new),
            CreateRoomOutcome::Created
        );
        assert_eq!(
            manager.rooms.get("room-d").unwrap().desktop.as_ref().unwrap().conn_id,
            conn_new
        );
    }

    /// A fresh-heartbeat zombie (disconnected less than the staleness
    /// window ago) is still treated as active → conflict.
    #[test]
    fn create_room_conflicts_with_recently_active_desktop() {
        let manager = RoomManager::new();
        let (conn_a, tx_a) = make_conn(&manager);
        manager.create_room("room-e", conn_a, "dev-a", "pk", tx_a);
        let (conn_b, tx_b) = make_conn(&manager);
        assert_eq!(
            manager.create_room("room-e", conn_b, "dev-b", "pk", tx_b),
            CreateRoomOutcome::Conflict
        );
    }

    /// `send_to_desktop` must never block: a full bounded queue fails fast.
    #[test]
    fn send_to_desktop_fails_fast_when_queue_full() {
        let manager = RoomManager::new();
        let conn_id = manager.next_conn_id();
        let (tx, _rx) = mpsc::channel(1);
        // Fill the only slot; every subsequent try_send fails.
        tx.try_send(OutboundMessage { text: "queued".into() }).unwrap();
        manager.create_room("room-f", conn_id, "dev", "pk", tx);

        assert!(!manager.send_to_desktop("room-f", "message"));
        assert!(manager.has_desktop("room-f"), "queue-full must not kill the room");
    }

    /// Normal `send_to_desktop` delivery still works on a bounded queue.
    #[test]
    fn send_to_desktop_delivers_on_bounded_queue() {
        let manager = RoomManager::new();
        let conn_id = manager.next_conn_id();
        let (tx, mut rx) = mpsc::channel(256);
        manager.create_room("room-g", conn_id, "dev", "pk", tx);

        assert!(manager.send_to_desktop("room-g", "hello"));
        let delivered = rx.try_recv().expect("message should be queued");
        assert_eq!(delivered.text, "hello");
        assert!(manager.send_to_desktop("missing-room", "x") == false);
    }

    // ── Connection limit (H-2) ─────────────────────────────────────────

    /// The admit/release counter tracks exactly the admitted connections.
    #[test]
    fn connection_slot_counter_increments_and_decrements() {
        let manager = RoomManager::new();
        assert_eq!(manager.active_connection_count(), 0);

        assert!(manager.try_acquire_connection());
        assert!(manager.try_acquire_connection());
        assert_eq!(manager.active_connection_count(), 2);

        manager.release_connection();
        assert_eq!(manager.active_connection_count(), 1);
        manager.release_connection();
        assert_eq!(manager.active_connection_count(), 0);
    }

    /// Once MAX_CONNECTIONS slots are taken, further admits are rejected
    /// until a slot is released.
    #[test]
    fn connection_limit_rejects_admits_at_capacity() {
        let manager = RoomManager::new();
        for _ in 0..MAX_CONNECTIONS {
            assert!(manager.try_acquire_connection());
        }
        assert!(!manager.try_acquire_connection(), "over-capacity admit must fail");
        assert_eq!(manager.active_connection_count(), MAX_CONNECTIONS);

        manager.release_connection();
        assert!(manager.try_acquire_connection());
        assert_eq!(manager.active_connection_count(), MAX_CONNECTIONS);
    }
}
