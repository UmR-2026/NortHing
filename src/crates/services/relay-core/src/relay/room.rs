//! Room management for the relay server.
//!
//! NOTE: This is one of two divergent implementations. The other is at
//! `src/apps/relay-server/src/relay/room.rs`. This surface is FROZEN
//! (v0.1.0). When unfreezing, the two implementations MUST be deduped
//! before any feature work.
//!
//! Each room holds a single desktop participant connected via WebSocket.
//! Mobile clients interact through HTTP requests that the relay bridges
//! to the desktop via the WebSocket connection. The relay stores no
//! business data — it only routes messages.

use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, info, warn};

pub type ConnId = u64;

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
    pub tx: mpsc::UnboundedSender<OutboundMessage>,
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

    pub fn send_to_desktop(&self, message: &str) -> bool {
        if let Some(ref desktop) = self.desktop {
            let _ = desktop.tx.send(OutboundMessage {
                text: message.to_string(),
            });
            true
        } else {
            false
        }
    }
}

pub struct RoomManager {
    rooms: DashMap<String, RelayRoom>,
    conn_to_room: DashMap<ConnId, String>,
    next_conn_id: std::sync::atomic::AtomicU64,
    pending_requests: DashMap<String, oneshot::Sender<ResponsePayload>>,
}

impl RoomManager {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            rooms: DashMap::new(),
            conn_to_room: DashMap::new(),
            next_conn_id: std::sync::atomic::AtomicU64::new(1),
            pending_requests: DashMap::new(),
        })
    }

    pub fn next_conn_id(&self) -> ConnId {
        self.next_conn_id.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    pub fn create_room(
        &self,
        room_id: &str,
        conn_id: ConnId,
        device_id: &str,
        public_key: &str,
        tx: mpsc::UnboundedSender<OutboundMessage>,
    ) -> bool {
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

        self.rooms.remove(room_id);

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

        self.rooms.insert(room_id.to_string(), room);
        self.conn_to_room.insert(conn_id, room_id.to_string());

        info!("Room {room_id} created by desktop {device_id}");
        true
    }

    pub fn send_to_desktop(&self, room_id: &str, message: &str) -> bool {
        if let Some(mut room) = self.rooms.get_mut(room_id) {
            room.touch();
            room.send_to_desktop(message)
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

    pub fn on_disconnect(&self, conn_id: ConnId) {
        if let Some((_, room_id)) = self.conn_to_room.remove(&conn_id) {
            let should_remove = if let Some(mut room) = self.rooms.get_mut(&room_id) {
                if room.desktop.as_ref().is_some_and(|d| d.conn_id == conn_id) {
                    info!("Desktop disconnected from room {room_id}");
                    room.desktop = None;
                }
                room.is_empty()
            } else {
                false
            };
            if should_remove {
                self.rooms.remove(&room_id);
                debug!("Empty room {room_id} removed");
            }
        }
    }

    pub fn heartbeat(&self, conn_id: ConnId) -> bool {
        if let Some(room_id) = self.conn_to_room.get(&conn_id) {
            if let Some(mut room) = self.rooms.get_mut(room_id.value()) {
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
        let conn_id_stale = manager.next_conn_id();
        let (tx_stale, _rx_stale) = mpsc::unbounded_channel();
        manager.create_room("stale-room", conn_id_stale, "device-stale", "pk-stale", tx_stale);
        if let Some(mut room) = manager.rooms.get_mut("stale-room") {
            room.last_activity = Utc::now().timestamp() - 10_000;
        }

        let conn_id_fresh = manager.next_conn_id();
        let (tx_fresh, _rx_fresh) = mpsc::unbounded_channel();
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
}
