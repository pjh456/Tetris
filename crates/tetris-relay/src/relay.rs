use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use tetris_protocol::protocol::{
    PacketHeader, PacketType, PktCountdownCancel, PktRoomSnapshot, RoomPlayerSnapshot,
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
    pub host_peer_id: RwLock<Option<u64>>,
    pub peers: Mutex<Vec<PeerInfo>>,
    pub countdown_active: AtomicUsize,
    pub countdown_generation: AtomicU64,
    pub cancel_tx: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    pub actor_tx: Mutex<Option<tokio::sync::mpsc::Sender<RoomCommand>>>,
}

/// First-class player session, decoupled from the WS connection. Survives a
/// disconnect (grace `away` window) so a reconnect can reclaim the same slot +
/// engine. Holds all player-scoped state; `PeerInfo` is the thin transport shell.
#[derive(Clone)]
pub struct Session {
    pub player_id: u8,
    pub name: String,
    pub is_bot: bool,
    pub temperature: f32,
    /// Server-issued unpredictable token authenticating reconnect/resume.
    /// Empty for bots (they never reconnect).
    pub resume_token: String,
    pub away: bool,
}

impl Session {
    fn new(
        player_id: u8,
        name: String,
        is_bot: bool,
        temperature: f32,
        resume_token: String,
    ) -> Self {
        Self {
            player_id,
            name,
            is_bot,
            temperature,
            resume_token,
            away: false,
        }
    }

    fn mark_away(&mut self) {
        self.away = true;
    }

    fn unmark_away(&mut self) {
        self.away = false;
    }

    /// A session is reclaimable by `token` only when it is away, not a bot, and
    /// the (non-empty) token matches exactly — a forged `player_id` cannot pass.
    fn reclaimable_by(&self, token: &str) -> bool {
        self.away && !self.is_bot && !token.is_empty() && self.resume_token == token
    }
}

/// Thin transport shell: a live WS connection (`id`) bound to a `Session`.
/// `ready` is connection-level lobby state (resets on reclaim).
#[derive(Clone)]
pub struct PeerInfo {
    pub id: u64,
    pub ready: bool,
    pub session: Session,
}

pub struct RoomManager {
    rooms: RwLock<HashMap<RoomCode, Arc<RoomState>>>,
    max_rooms: usize,
}

/// Pick the lowest unused `player_id` in the room (avoids collision on leave+join).
/// `map_or` fallback is dead code at the existing call sites: a `peers.len() < MAX`
/// check guarantees an unused slot exists before this is called.
fn alloc_player_slot(peers: &[PeerInfo]) -> u8 {
    let mut used = [false; MAX_PLAYERS_PER_ROOM];
    for p in peers {
        if (p.session.player_id as usize) < MAX_PLAYERS_PER_ROOM {
            used[p.session.player_id as usize] = true;
        }
    }
    used.iter()
        .position(|u| !u)
        .map_or(MAX_PLAYERS_PER_ROOM as u8 - 1, |i| i as u8)
}

/// Generate an unpredictable 128-bit resume token (hex). The client must echo
/// this exact value to be authenticated on reconnect — a forged `player_id`
/// (0/1/2) cannot match it.
fn generate_resume_token() -> String {
    format!("{:032x}", rand::random::<u128>())
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
        let player_id = alloc_player_slot(&peers);
        let name = format!("Player {}", player_id + 1);
        peers.push(PeerInfo {
            id,
            ready: false,
            session: Session::new(player_id, name, false, 0.0, generate_resume_token()),
        });
        if room.host_peer_id.read().await.is_none() {
            *room.host_peer_id.write().await = Some(id);
        }
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
        let player_id = alloc_player_slot(&peers);
        let id = Self::alloc_peer_id();
        let bot_count = peers.iter().filter(|peer| peer.session.is_bot).count() + 1;
        let peer = PeerInfo {
            id,
            ready: true,
            session: Session::new(
                player_id,
                format!("AI {bot_count}"),
                true,
                temperature,
                String::new(),
            ),
        };
        peers.push(peer.clone());
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
        peer.session.name = name;
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
            room.countdown_active.store(0, Ordering::SeqCst);
            room.countdown_generation.fetch_add(1, Ordering::SeqCst);
            peers.clone()
        } else {
            vec![]
        }
    }

    /// Mark a peer as away (disconnected, in grace window) without removing it.
    /// Keeps its slot, engine, and `resume_token` so a reconnect can reclaim them.
    /// Returns the peer clone if found.
    pub async fn mark_peer_away(&self, code: &str, id: u64) -> Option<PeerInfo> {
        let rooms = self.rooms.read().await;
        let room = rooms.get(code)?;
        let mut peers = room.peers.lock().await;
        let peer = peers.iter_mut().find(|p| p.id == id)?;
        peer.session.mark_away();
        Some(peer.clone())
    }

    /// Returns true if a peer with this id exists and is still marked away.
    /// Used by the grace timer to decide whether to truly remove the slot.
    pub async fn peer_is_away(&self, code: &str, id: u64) -> bool {
        let rooms = self.rooms.read().await;
        let Some(room) = rooms.get(code) else {
            return false;
        };
        room.peers
            .lock()
            .await
            .iter()
            .any(|p| p.id == id && p.session.away)
    }

    /// Reclaim an away peer whose `resume_token` matches `token` (non-empty),
    /// rebinding it to the new connection `new_id` and clearing the away flag.
    /// Returns the reclaimed peer (with its original `slot`/`player_id`) or None when
    /// no away peer matches — e.g. a forged or stale token.
    pub async fn reclaim_away_peer(
        &self,
        code: &str,
        new_id: u64,
        token: &str,
    ) -> Option<PeerInfo> {
        if token.is_empty() {
            return None;
        }
        let rooms = self.rooms.read().await;
        let room = rooms.get(code)?;
        let mut peers = room.peers.lock().await;
        let peer = peers.iter_mut().find(|p| p.session.reclaimable_by(token))?;
        peer.session.unmark_away();
        peer.id = new_id;
        if room.host_peer_id.read().await.is_none() {
            *room.host_peer_id.write().await = Some(new_id);
        }
        Some(peer.clone())
    }

    pub async fn broadcast_snapshot(&self, code: &str, peers: &[PeerInfo]) {
        let rooms = self.rooms.read().await;
        let Some(room) = rooms.get(code) else {
            return;
        };
        let host_peer_id = *room.host_peer_id.read().await;
        let pkt = PktRoomSnapshot {
            header: PacketHeader::new(PacketType::RoomSnapshot, 0),
            room_code: code.to_string(),
            players: peers
                .iter()
                .map(|peer| RoomPlayerSnapshot {
                    player_id: peer.session.player_id,
                    name: peer.session.name.clone(),
                    ready: peer.ready,
                    alive: true,
                    away: peer.session.away,
                    is_host: host_peer_id.is_some_and(|host_id| host_id == peer.id),
                    is_bot: peer.session.is_bot,
                })
                .collect(),
        };
        if let Ok(data) = bincode::serialize(&pkt) {
            let _ = room.tx.send(data);
        }
    }

    /// 取房间 peers 并广播 `RoomSnapshot`，调用方无需先 `room_peers`。
    pub async fn broadcast_room_snapshot(&self, code: &str) {
        if let Ok(peers) = self.room_peers(code).await {
            self.broadcast_snapshot(code, &peers).await;
        }
    }

    /// 取消倒计时并广播 `CountdownCancel`，成对封装防漏。
    pub async fn cancel_and_broadcast_countdown(&self, code: &str) {
        let _ = self.cancel_countdown(code).await;
        let pkt = PktCountdownCancel {
            header: PacketHeader::new(PacketType::CountdownCancel, 0),
        };
        if let Ok(data) = bincode::serialize(&pkt) {
            let _ = self.broadcast(code, data).await;
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

    /// Find a peer by its in-game `player_id` (slot). Used for host-issued
    /// kick / remove-bot, where the target is addressed by slot, not connection.
    pub async fn peer_by_player_id(
        &self,
        code: &str,
        player_id: u8,
    ) -> Result<PeerInfo, RelayError> {
        let rooms = self.rooms.read().await;
        let room = rooms
            .get(code)
            .ok_or_else(|| RelayError::RoomNotFound(code.into()))?;
        room.peers
            .lock()
            .await
            .iter()
            .find(|peer| peer.session.player_id == player_id)
            .cloned()
            .ok_or(RelayError::PeerNotFound)
    }

    /// True iff connection `id` is the current host of room `code`.
    pub async fn is_host(&self, code: &str, id: u64) -> bool {
        let rooms = self.rooms.read().await;
        let Some(room) = rooms.get(code) else {
            return false;
        };
        room.host_peer_id
            .read()
            .await
            .is_some_and(|host_id| host_id == id)
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
        // 容量闸以 peers.len() 为唯一记账源（add_peer 同样校验，双重防护）。
        if room.peers.lock().await.len() >= MAX_PLAYERS_PER_ROOM {
            return Err(RelayError::RoomFull(code.into()));
        }
        Ok(room.tx.subscribe())
    }

    pub async fn leave_room(&self, code: &str) {
        // 房间删除仅依据 peers 是否为空（单一记账源），不再用独立 counter，
        // 避免与 remove_peer 双轨叠加导致 2 人房一人离开误删整间房。
        let should_remove = {
            let rooms = self.rooms.read().await;
            match rooms.get(code) {
                Some(room) => room.peers.lock().await.is_empty(),
                None => false,
            }
        };
        if should_remove {
            let mut rooms = self.rooms.write().await;
            if let Some(room) = rooms.get(code)
                && room.peers.lock().await.is_empty()
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
    async fn two_player_room_survives_one_leave() {
        let manager = RoomManager::new(4);
        manager.get_or_create_room("ABCD").await.unwrap();
        let id1 = RoomManager::alloc_peer_id();
        let id2 = RoomManager::alloc_peer_id();
        manager.join_room("ABCD").await.unwrap();
        manager.add_peer("ABCD", id1).await.unwrap();
        manager.join_room("ABCD").await.unwrap();
        manager.add_peer("ABCD", id2).await.unwrap();

        // 一人离开：remove_peer 后 leave_room 不应误删整间房。
        let remaining = manager.remove_peer("ABCD", id1).await;
        manager.leave_room("ABCD").await;

        assert_eq!(remaining.len(), 1);
        let peers = manager.room_peers("ABCD").await.unwrap();
        assert_eq!(peers.len(), 1);
    }

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
        assert_eq!(remaining[0].session.player_id, 1);

        manager.join_room("ABCD").await.unwrap();
        let peers = manager.add_peer("ABCD", third_id).await.unwrap();

        let mut slots: Vec<_> = peers.iter().map(|peer| peer.session.player_id).collect();
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

        assert_eq!(bot.session.player_id, 1);
        assert!(bot.session.is_bot);
        assert!(bot.ready);
        assert_eq!(bot.session.temperature, 0.5);
        assert_eq!(peers.len(), 2);
    }

    #[tokio::test]
    async fn reclaim_away_peer_restores_same_slot_on_matching_token() {
        let manager = RoomManager::new(4);
        manager.get_or_create_room("ABCD").await.unwrap();
        let first_id = RoomManager::alloc_peer_id();
        manager.join_room("ABCD").await.unwrap();
        let peers = manager.add_peer("ABCD", first_id).await.unwrap();
        let slot = peers[0].session.player_id;
        let token = peers[0].session.resume_token.clone();
        assert!(!token.is_empty());

        // Disconnect: mark away (slot preserved during grace), then reconnect.
        manager.mark_peer_away("ABCD", first_id).await.unwrap();
        assert!(manager.peer_is_away("ABCD", first_id).await);

        let new_id = RoomManager::alloc_peer_id();
        let reclaimed = manager
            .reclaim_away_peer("ABCD", new_id, &token)
            .await
            .expect("matching token must reclaim the away slot");

        assert_eq!(
            reclaimed.session.player_id, slot,
            "reclaim keeps original slot"
        );
        assert!(!reclaimed.session.away);
        assert_eq!(reclaimed.id, new_id, "peer rebound to new connection");
    }

    #[tokio::test]
    async fn reclaim_away_peer_rejects_forged_player_id_token() {
        let manager = RoomManager::new(4);
        manager.get_or_create_room("ABCD").await.unwrap();
        let first_id = RoomManager::alloc_peer_id();
        manager.join_room("ABCD").await.unwrap();
        let peers = manager.add_peer("ABCD", first_id).await.unwrap();
        let slot = peers[0].session.player_id;
        manager.mark_peer_away("ABCD", first_id).await.unwrap();

        // Attacker guesses the resume token is the player_id (the old behavior).
        let forged = slot.to_string();
        let new_id = RoomManager::alloc_peer_id();

        assert!(
            manager
                .reclaim_away_peer("ABCD", new_id, &forged)
                .await
                .is_none(),
            "forged player_id token must not reclaim the slot"
        );
        // Empty token is also rejected outright.
        assert!(
            manager
                .reclaim_away_peer("ABCD", new_id, "")
                .await
                .is_none()
        );
    }
}
