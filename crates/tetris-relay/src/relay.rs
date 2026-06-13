use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

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
    pub host_id: RwLock<Option<String>>,
    pub peers: Mutex<Vec<PeerInfo>>,
}

#[derive(Clone)]
pub struct PeerInfo {
    pub id: u64,
    pub name: String,
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
        let name = format!("Player {}", peers.len() + 1);
        peers.push(PeerInfo { id, name });
        Ok(peers.clone())
    }

    /// Remove peer from room. Returns updated peer list (may be empty if room gone).
    pub async fn remove_peer(&self, code: &str, id: u64) -> Vec<PeerInfo> {
        let rooms = self.rooms.read().await;
        if let Some(room) = rooms.get(code) {
            let mut peers = room.peers.lock().await;
            peers.retain(|p| p.id != id);
            peers.clone()
        } else {
            vec![]
        }
    }

    /// Broadcast presence list as JSON-over-binary to all room clients.
    pub async fn broadcast_presence(&self, code: &str, peers: &[PeerInfo]) {
        let names_json = peers
            .iter()
            .map(|p| {
                let escaped = p.name.replace('\\', "\\\\").replace('"', "\\\"");
                format!("\"{escaped}\"")
            })
            .collect::<Vec<_>>()
            .join(",");
        let msg = format!("{{\"type\":\"presence\",\"peers\":[{names_json}]}}");
        let _ = self.broadcast(code, msg.into_bytes()).await;
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
            host_id: RwLock::new(None),
            peers: Mutex::new(vec![]),
        });
        rooms.insert(code.clone(), state);
        Ok(code)
    }

    pub async fn join_room(&self, code: &str) -> Result<broadcast::Receiver<Vec<u8>>, RelayError> {
        let rooms = self.rooms.read().await;
        let room = rooms
            .get(code)
            .ok_or_else(|| RelayError::RoomNotFound(code.into()))?;
        let count = room.player_count.load(Ordering::SeqCst);
        if count >= MAX_PLAYERS_PER_ROOM {
            return Err(RelayError::RoomFull(code.into()));
        }
        room.player_count.fetch_add(1, Ordering::SeqCst);
        Ok(room.tx.subscribe())
    }

    pub async fn leave_room(&self, code: &str) {
        let should_remove = {
            let rooms = self.rooms.read().await;
            if let Some(room) = rooms.get(code) {
                let prev = room.player_count.fetch_sub(1, Ordering::SeqCst);
                prev <= 1
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
            host_id: RwLock::new(None),
            peers: Mutex::new(vec![]),
        });
        rooms.insert(code.to_string(), state);
        Ok(code.to_string())
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
