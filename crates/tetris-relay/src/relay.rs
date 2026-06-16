use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use tetris_protocol::protocol::{
    PROTOCOL_VERSION, PacketHeader, PacketType, PktRoomSnapshot, RoomPlayerSnapshot,
};
use tokio::sync::{Mutex, RwLock, broadcast};

use crate::error::RelayError;

pub type RoomCode = String;

const ROOM_CODE_ALPHABET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";
const ROOM_CODE_LEN: usize = 4;
const MAX_PLAYERS_PER_ROOM: usize = 4;
const BROADCAST_CAPACITY: usize = 256;

static NEXT_PEER_ID: AtomicU64 = AtomicU64::new(1);

pub struct RoomState {
    pub code: RoomCode,
    pub tx: broadcast::Sender<Vec<u8>>,
    pub player_count: AtomicUsize,
    pub host_peer_id: RwLock<Option<u64>>,
    pub peers: Mutex<Vec<PeerInfo>>,
    pub countdown_active: AtomicUsize,
    pub cancel_tx: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
}

#[derive(Clone)]
pub struct PeerInfo {
    pub id: u64,
    pub player_id: u8,
    pub name: String,
    pub ready: bool,
    pub away: bool,
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
        let player_id = used.iter().position(|u| !u).map(|i| i as u8)
            .unwrap_or(MAX_PLAYERS_PER_ROOM as u8 - 1);
        let name = format!("Player {}", player_id + 1);
        peers.push(PeerInfo {
            id,
            player_id,
            name,
            ready: false,
            away: false,
        });
        if room.host_peer_id.read().await.is_none() {
            *room.host_peer_id.write().await = Some(id);
        }
        Ok(peers.clone())
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
    pub async fn try_start_countdown(&self, code: &str) -> Result<bool, RelayError> {
        let rooms = self.rooms.read().await;
        let room = rooms
            .get(code)
            .ok_or_else(|| RelayError::RoomNotFound(code.into()))?;
        Ok(room
            .countdown_active
            .compare_exchange(0, 1, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok())
    }

    pub async fn set_countdown_active(&self, code: &str, active: bool) -> Result<(), RelayError> {
        let rooms = self.rooms.read().await;
        let room = rooms
            .get(code)
            .ok_or_else(|| RelayError::RoomNotFound(code.into()))?;
        room.countdown_active
            .store(usize::from(active), Ordering::SeqCst);
        Ok(())
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
            cancel_tx: Mutex::new(None),
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
                let prev = room.player_count.fetch_update(
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                    |p| if p > 0 { Some(p - 1) } else { Some(0) },
                );
                prev.map_or(true, |p| p <= 1)
            } else {
                false
            }
        };
        if should_remove {
            let mut rooms = self.rooms.write().await;
            if rooms
                .get(code)
                .is_some_and(|r| r.player_count.load(Ordering::SeqCst) == 0)
            {
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
            cancel_tx: Mutex::new(None),
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
