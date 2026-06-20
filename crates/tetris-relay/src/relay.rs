use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use tetris_protocol::protocol::{
    PROTOCOL_VERSION, PacketHeader, PacketType, PktRoomSnapshot, RoomPlayerSnapshot,
};
use tokio::sync::{Mutex, RwLock, broadcast};

use crate::error::RelayError;
use crate::room_actor::RoomCommand;

pub type RoomCode = String;

const ROOM_CODE_ALPHABET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";
const ROOM_CODE_LEN: usize = 4;
pub const MAX_PLAYERS_PER_ROOM: usize = 4;
const BROADCAST_CAPACITY: usize = 256;

static NEXT_PEER_ID: AtomicU64 = AtomicU64::new(1);

pub struct RoomState {
    pub code: RoomCode,
    pub tx: broadcast::Sender<Vec<u8>>,
    /// `player_count` is used two ways: join/leave CAS for capacity gating,
    /// `add_peer`/`remove_peer` overwrite from `peers.len()` for consistency.
    pub player_count: AtomicUsize,
    pub host_peer_id: RwLock<Option<u64>>,
    pub peers: Mutex<Vec<PeerInfo>>,
    pub countdown_active: AtomicUsize,
    pub countdown_generation: AtomicU64,
    pub cancel_tx: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    pub actor_tx: Mutex<Option<tokio::sync::mpsc::Sender<RoomCommand>>>,
}

#[derive(Clone)]
pub struct PeerInfo {
    pub id: u64,
    pub player_id: u8,
    pub name: String,
    pub ready: bool,
    pub away: bool,
    pub is_bot: bool,
    pub temperature: f32,
}

pub struct RoomManager {
    rooms: RwLock<HashMap<RoomCode, Arc<RoomState>>>,
    max_rooms: usize,
}

impl RoomManager {
    pub fn new(max_rooms: usize) -> Self {
        Self {
            rooms: RwLock::new(HashMap::new()),
            max_rooms,
        }
    }

    pub fn alloc_peer_id() -> u64 {
        NEXT_PEER_ID.fetch_add(1, Ordering::Relaxed)
    }

    /// Add peer to room, assigning a sequential display name. Returns updated peer list.
    pub async fn add_peer(&self, code: &str, id: u64) -> Result<Vec<PeerInfo>, RelayError> {
        let rooms = self.rooms.read().await;
        let room = rooms
            .get(code)
            .ok_or_else(|| RelayError::RoomNotFound(code.into()))?;
        let mut peers = room.peers.lock().await;
        if peers.len() >= MAX_PLAYERS_PER_ROOM {
            return Err(RelayError::RoomFull("too many players".into()));
        }
        // Pick lowest unused player_id instead of peers.len() — avoids collision on leave+join
        let mut used = [false; MAX_PLAYERS_PER_ROOM];
        for p in peers.iter() {
            if (p.player_id as usize) < MAX_PLAYERS_PER_ROOM {
                used[p.player_id as usize] = true;
            }
        }
        // unwrap_or is dead code: len() check guarantees an unused slot exists.
        let player_id = used
            .iter()
            .position(|u| !u)
            .map_or(MAX_PLAYERS_PER_ROOM as u8 - 1, |i| i as u8);
        let name = format!("Player {}", player_id + 1);
        peers.push(PeerInfo {
            id,
            player_id,
            name,
            ready: false,
            away: false,
            is_bot: false,
            temperature: 0.0,
        });
        if room.host_peer_id.read().await.is_none() {
            *room.host_peer_id.write().await = Some(id);
        }
        room.player_count.store(peers.len(), Ordering::SeqCst);
        Ok(peers.clone())
    }

    pub async fn add_bot_peer(&self, code: &str, temperature: f32) -> Result<PeerInfo, RelayError> {
        let rooms = self.rooms.read().await;
        let room = rooms
            .get(code)
            .ok_or_else(|| RelayError::RoomNotFound(code.into()))?;
        let mut peers = room.peers.lock().await;
        if peers.len() >= MAX_PLAYERS_PER_ROOM {
            return Err(RelayError::RoomFull("too many players".into()));
        }
        let mut used = [false; MAX_PLAYERS_PER_ROOM];
        for p in peers.iter() {
            if (p.player_id as usize) < MAX_PLAYERS_PER_ROOM {
                used[p.player_id as usize] = true;
            }
        }
        let player_id = used
            .iter()
            .position(|u| !u)
            .map_or(MAX_PLAYERS_PER_ROOM as u8 - 1, |i| i as u8);
        let id = Self::alloc_peer_id();
        let bot_count = peers.iter().filter(|peer| peer.is_bot).count() + 1;
        let peer = PeerInfo {
            id,
            player_id,
            name: format!("AI {bot_count}"),
            ready: true,
            away: false,
            is_bot: true,
            temperature,
        };
        peers.push(peer.clone());
        room.player_count.store(peers.len(), Ordering::SeqCst);
        Ok(peer)
    }

    pub async fn rename_peer(
        &self,
        code: &str,
        id: u64,
        mut name: String,
    ) -> Result<Vec<PeerInfo>, RelayError> {
        const MAX_NAME_LEN: usize = 32;
        name.truncate(MAX_NAME_LEN);
        let rooms = self.rooms.read().await;
        let room = rooms
            .get(code)
            .ok_or_else(|| RelayError::RoomNotFound(code.into()))?;
        let mut peers = room.peers.lock().await;
        let peer = peers
            .iter_mut()
            .find(|peer| peer.id == id)
            .ok_or(RelayError::PeerNotFound)?;
        peer.name = name;
        Ok(peers.clone())
    }

    pub async fn set_peer_ready(
        &self,
        code: &str,
        id: u64,
        ready: bool,
    ) -> Result<Vec<PeerInfo>, RelayError> {
        let rooms = self.rooms.read().await;
        let room = rooms
            .get(code)
            .ok_or_else(|| RelayError::RoomNotFound(code.into()))?;
        let mut peers = room.peers.lock().await;
        let peer = peers
            .iter_mut()
            .find(|peer| peer.id == id)
            .ok_or(RelayError::PeerNotFound)?;
        peer.ready = ready;
        if !ready {
            room.countdown_active.store(0, Ordering::SeqCst);
            room.countdown_generation.fetch_add(1, Ordering::SeqCst);
        }
        Ok(peers.clone())
    }

    pub async fn reset_ready_states(&self, code: &str) -> Result<Vec<PeerInfo>, RelayError> {
        let rooms = self.rooms.read().await;
        let room = rooms
            .get(code)
            .ok_or_else(|| RelayError::RoomNotFound(code.into()))?;
        let mut peers = room.peers.lock().await;
        for peer in &mut *peers {
            peer.ready = false;
        }
        Ok(peers.clone())
    }

    pub async fn all_peers_ready(&self, code: &str) -> Result<bool, RelayError> {
        let rooms = self.rooms.read().await;
        let room = rooms
            .get(code)
            .ok_or_else(|| RelayError::RoomNotFound(code.into()))?;
        let peers = room.peers.lock().await;
        Ok(peers.len() >= 2 && peers.iter().all(|peer| peer.ready))
    }

    /// Remove peer from room. Returns updated peer list (may be empty if room gone).
    pub async fn remove_peer(&self, code: &str, id: u64) -> Vec<PeerInfo> {
        let rooms = self.rooms.read().await;
        if let Some(room) = rooms.get(code) {
            let mut peers = room.peers.lock().await;
            peers.retain(|p| p.id != id);
            let mut host_peer_id = room.host_peer_id.write().await;
            if host_peer_id.as_ref().is_some_and(|host_id| *host_id == id) {
                *host_peer_id = peers.first().map(|peer| peer.id);
            }
            room.player_count.store(peers.len(), Ordering::SeqCst);
            room.countdown_active.store(0, Ordering::SeqCst);
            room.countdown_generation.fetch_add(1, Ordering::SeqCst);
            peers.clone()
        } else {
            vec![]
        }
    }

    pub async fn broadcast_snapshot(&self, code: &str, peers: &[PeerInfo]) {
        let rooms = self.rooms.read().await;
        let Some(room) = rooms.get(code) else {
            return;
        };
        let host_peer_id = *room.host_peer_id.read().await;
        let pkt = PktRoomSnapshot {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::RoomSnapshot,
                player_id: 0,
            },
            room_code: code.to_string(),
            players: peers
                .iter()
                .map(|peer| RoomPlayerSnapshot {
                    player_id: peer.player_id,
                    name: peer.name.clone(),
                    ready: peer.ready,
                    alive: true,
                    away: peer.away,
                    is_host: host_peer_id.is_some_and(|host_id| host_id == peer.id),
                })
                .collect(),
        };
        if let Ok(data) = bincode::serialize(&pkt) {
            let _ = room.tx.send(data);
        }
    }

    pub async fn room_peers(&self, code: &str) -> Result<Vec<PeerInfo>, RelayError> {
        let rooms = self.rooms.read().await;
        let room = rooms
            .get(code)
            .ok_or_else(|| RelayError::RoomNotFound(code.into()))?;
        Ok(room.peers.lock().await.clone())
    }

    pub async fn peer_by_id(&self, code: &str, id: u64) -> Result<PeerInfo, RelayError> {
        let rooms = self.rooms.read().await;
        let room = rooms
            .get(code)
            .ok_or_else(|| RelayError::RoomNotFound(code.into()))?;
        room.peers
            .lock()
            .await
            .iter()
            .find(|peer| peer.id == id)
            .cloned()
            .ok_or(RelayError::PeerNotFound)
    }

    pub async fn countdown_active(&self, code: &str) -> Result<bool, RelayError> {
        let rooms = self.rooms.read().await;
        let room = rooms
            .get(code)
            .ok_or_else(|| RelayError::RoomNotFound(code.into()))?;
        Ok(room.countdown_active.load(Ordering::SeqCst) != 0)
    }

    /// Atomically try to start the countdown. Returns Ok(true) if countdown was started,
    /// Ok(false) if it was already active (caller should not start a duplicate).
    pub async fn try_start_countdown(&self, code: &str) -> Result<Option<u64>, RelayError> {
        let rooms = self.rooms.read().await;
        let room = rooms
            .get(code)
            .ok_or_else(|| RelayError::RoomNotFound(code.into()))?;
        if room
            .countdown_active
            .compare_exchange(0, 1, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            let generation = room.countdown_generation.fetch_add(1, Ordering::SeqCst) + 1;
            Ok(Some(generation))
        } else {
            Ok(None)
        }
    }

    pub async fn cancel_countdown(&self, code: &str) -> Result<(), RelayError> {
        let rooms = self.rooms.read().await;
        let room = rooms
            .get(code)
            .ok_or_else(|| RelayError::RoomNotFound(code.into()))?;
        room.countdown_active.store(0, Ordering::SeqCst);
        room.countdown_generation.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    pub async fn countdown_generation_matches(
        &self,
        code: &str,
        generation: u64,
    ) -> Result<bool, RelayError> {
        let rooms = self.rooms.read().await;
        let room = rooms
            .get(code)
            .ok_or_else(|| RelayError::RoomNotFound(code.into()))?;
        Ok(room.countdown_active.load(Ordering::SeqCst) != 0
            && room.countdown_generation.load(Ordering::SeqCst) == generation)
    }

    pub async fn create_room(&self) -> Result<RoomCode, RelayError> {
        let mut rooms = self.rooms.write().await;
        if rooms.len() >= self.max_rooms {
            return Err(RelayError::RoomFull("server at max room capacity".into()));
        }
        let code = generate_room_code(&rooms);
        let (tx, _) = broadcast::channel(BROADCAST_CAPACITY);
        let state = Arc::new(RoomState {
            code: code.clone(),
            tx,
            player_count: AtomicUsize::new(0),
            host_peer_id: RwLock::new(None),
            peers: Mutex::new(vec![]),
            countdown_active: AtomicUsize::new(0),
            countdown_generation: AtomicU64::new(0),
            cancel_tx: Mutex::new(None),
            actor_tx: Mutex::new(None),
        });
        rooms.insert(code.clone(), state);
        Ok(code)
    }

    pub async fn join_room(&self, code: &str) -> Result<broadcast::Receiver<Vec<u8>>, RelayError> {
        let rooms = self.rooms.read().await;
        let room = rooms
            .get(code)
            .ok_or_else(|| RelayError::RoomNotFound(code.into()))?;
        room.player_count
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |count| {
                if count >= MAX_PLAYERS_PER_ROOM {
                    None
                } else {
                    Some(count + 1)
                }
            })
            .map_err(|_| RelayError::RoomFull(code.into()))?;
        Ok(room.tx.subscribe())
    }

    pub async fn leave_room(&self, code: &str) {
        let should_remove = {
            let rooms = self.rooms.read().await;
            if let Some(room) = rooms.get(code) {
                let prev =
                    room.player_count
                        .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |p| {
                            if p > 0 { Some(p - 1) } else { Some(0) }
                        });
                prev.map_or(true, |p| p <= 1)
            } else {
                false
            }
        };
        if should_remove {
            let mut rooms = self.rooms.write().await;
            if let Some(room) = rooms.get(code)
                && room.player_count.load(Ordering::SeqCst) == 0
            {
                if let Some(cancel_tx) = room.cancel_tx.lock().await.take() {
                    let _ = cancel_tx.send(());
                }
                room.countdown_active.store(0, Ordering::SeqCst);
                room.countdown_generation.fetch_add(1, Ordering::SeqCst);
                rooms.remove(code);
            }
        }
    }

    pub async fn broadcast(&self, code: &str, data: Vec<u8>) -> Result<(), RelayError> {
        let rooms = self.rooms.read().await;
        let room = rooms
            .get(code)
            .ok_or_else(|| RelayError::RoomNotFound(code.into()))?;
        let _ = room.tx.send(data);
        Ok(())
    }

    pub async fn get_or_create_room(&self, code: &str) -> Result<RoomCode, RelayError> {
        {
            let rooms = self.rooms.read().await;
            if rooms.contains_key(code) {
                return Ok(code.to_string());
            }
        }
        let mut rooms = self.rooms.write().await;
        if rooms.contains_key(code) {
            return Ok(code.to_string());
        }
        if rooms.len() >= self.max_rooms {
            return Err(RelayError::RoomFull("server at max room capacity".into()));
        }
        let (tx, _) = broadcast::channel(BROADCAST_CAPACITY);
        let state = Arc::new(RoomState {
            code: code.to_string(),
            tx,
            player_count: AtomicUsize::new(0),
            host_peer_id: RwLock::new(None),
            peers: Mutex::new(vec![]),
            countdown_active: AtomicUsize::new(0),
            countdown_generation: AtomicU64::new(0),
            cancel_tx: Mutex::new(None),
            actor_tx: Mutex::new(None),
        });
        rooms.insert(code.to_string(), state);
        Ok(code.to_string())
    }

    pub async fn store_cancel_tx(
        &self,
        code: &str,
        tx: tokio::sync::oneshot::Sender<()>,
    ) -> Result<(), RelayError> {
        let rooms = self.rooms.read().await;
        let room = rooms
            .get(code)
            .ok_or_else(|| RelayError::RoomNotFound(code.into()))?;
        *room.cancel_tx.lock().await = Some(tx);
        Ok(())
    }

    pub async fn store_actor_tx(
        &self,
        code: &str,
        tx: tokio::sync::mpsc::Sender<RoomCommand>,
    ) -> Result<(), RelayError> {
        let rooms = self.rooms.read().await;
        let room = rooms
            .get(code)
            .ok_or_else(|| RelayError::RoomNotFound(code.into()))?;
        *room.actor_tx.lock().await = Some(tx);
        Ok(())
    }

    pub async fn actor_tx(
        &self,
        code: &str,
    ) -> Result<Option<tokio::sync::mpsc::Sender<RoomCommand>>, RelayError> {
        let rooms = self.rooms.read().await;
        let room = rooms
            .get(code)
            .ok_or_else(|| RelayError::RoomNotFound(code.into()))?;
        Ok(room.actor_tx.lock().await.clone())
    }
}

fn generate_room_code(existing: &HashMap<RoomCode, Arc<RoomState>>) -> RoomCode {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    loop {
        let code: String = (0..ROOM_CODE_LEN)
            .map(|_| {
                let idx = rng.gen_range(0..ROOM_CODE_ALPHABET.len());
                ROOM_CODE_ALPHABET[idx] as char
            })
            .collect();
        if !existing.contains_key(&code) {
            return code;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{Duration, timeout};

    #[tokio::test]
    async fn lifecycle_game_over_and_cancel() {
        let manager = RoomManager::new(4);
        manager.get_or_create_room("ABCD").await.unwrap();
        let _rx = manager.join_room("ABCD").await.unwrap();
        let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();
        manager.store_cancel_tx("ABCD", cancel_tx).await.unwrap();

        manager.leave_room("ABCD").await;

        assert!(timeout(Duration::from_millis(100), cancel_rx).await.is_ok());
    }

    #[tokio::test]
    async fn all_peers_ready_returns_false_after_ready_revoked() {
        let manager = RoomManager::new(4);
        manager.get_or_create_room("ABCD").await.unwrap();
        let first_id = RoomManager::alloc_peer_id();
        let second_id = RoomManager::alloc_peer_id();
        manager.join_room("ABCD").await.unwrap();
        manager.add_peer("ABCD", first_id).await.unwrap();
        manager.join_room("ABCD").await.unwrap();
        manager.add_peer("ABCD", second_id).await.unwrap();
        manager
            .set_peer_ready("ABCD", first_id, true)
            .await
            .unwrap();
        manager
            .set_peer_ready("ABCD", second_id, true)
            .await
            .unwrap();
        manager
            .set_peer_ready("ABCD", second_id, false)
            .await
            .unwrap();

        assert!(!manager.all_peers_ready("ABCD").await.unwrap());
    }

    #[tokio::test]
    async fn player_slot_allocation_reuses_lowest_free_slot() {
        let manager = RoomManager::new(4);
        manager.get_or_create_room("ABCD").await.unwrap();
        let first_id = RoomManager::alloc_peer_id();
        let second_id = RoomManager::alloc_peer_id();
        let third_id = RoomManager::alloc_peer_id();
        manager.join_room("ABCD").await.unwrap();
        manager.add_peer("ABCD", first_id).await.unwrap();
        manager.join_room("ABCD").await.unwrap();
        manager.add_peer("ABCD", second_id).await.unwrap();

        let remaining = manager.remove_peer("ABCD", first_id).await;
        assert_eq!(remaining[0].player_id, 1);

        manager.join_room("ABCD").await.unwrap();
        let peers = manager.add_peer("ABCD", third_id).await.unwrap();

        let mut slots: Vec<_> = peers.iter().map(|peer| peer.player_id).collect();
        slots.sort_unstable();
        assert_eq!(slots, vec![0, 1]);
    }

    #[tokio::test]
    async fn add_bot_peer_uses_normal_room_slot() {
        let manager = RoomManager::new(4);
        manager.get_or_create_room("ABCD").await.unwrap();
        manager.join_room("ABCD").await.unwrap();
        let human_id = RoomManager::alloc_peer_id();
        manager.add_peer("ABCD", human_id).await.unwrap();

        let bot = manager.add_bot_peer("ABCD", 0.5).await.unwrap();
        let peers = manager.room_peers("ABCD").await.unwrap();

        assert_eq!(bot.player_id, 1);
        assert!(bot.is_bot);
        assert!(bot.ready);
        assert_eq!(bot.temperature, 0.5);
        assert_eq!(peers.len(), 2);
    }
}
