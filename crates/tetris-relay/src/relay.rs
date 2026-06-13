use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use tokio::sync::{RwLock, broadcast};

use crate::error::RelayError;

pub type RoomCode = String;

const ROOM_CODE_ALPHABET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";
const ROOM_CODE_LEN: usize = 4;
const MAX_PLAYERS_PER_ROOM: usize = 4;
const BROADCAST_CAPACITY: usize = 256;

pub struct RoomState {
    pub code: RoomCode,
    pub tx: broadcast::Sender<Vec<u8>>,
    pub player_count: AtomicUsize,
    pub host_id: RwLock<Option<String>>,
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
