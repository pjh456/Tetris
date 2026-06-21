use std::collections::HashSet;
use std::time::Duration;

use bincode::Options;
use renet::ClientId;
use tetris_protocol::newtypes::{PlayerSlot, Seed};
use tetris_protocol::protocol::{
    PROTOCOL_VERSION, PacketHeader, PacketType, PktReplay, PktServerAccept,
};
use tetris_sim::{AuthoritativeSim, RoomMode, SimOutbound};

use crate::error::NetError;
use crate::network_manager::{MODEL_B_CHANNEL, NetworkManager, RELIABLE_CHANNEL};

const MAX_PACKET_BYTES: u64 = 65536;

pub struct RenetHostAdapter {
    pub sim: AuthoritativeSim,
    known_clients: HashSet<ClientId>,
    max_players: u8,
}

impl RenetHostAdapter {
    pub fn new(seed: Seed, max_players: u8) -> Self {
        let mut sim = AuthoritativeSim::new(seed);
        sim.add_player(PlayerSlot(0));
        Self {
            sim,
            known_clients: HashSet::new(),
            max_players,
        }
    }

    pub fn tick(&mut self, net: &mut NetworkManager, delta: Duration) -> Result<(), NetError> {
        net.tick(delta)?;
        self.accept_new_clients(net)?;
        self.collect_replay_packets(net)?;
        let outbound = self
            .sim
            .tick()
            .map_err(|e| NetError::Protocol(e.to_string()))?;
        Self::dispatch_outbound(net, outbound);
        Ok(())
    }

    pub fn start_playing(&mut self) {
        self.sim.set_room_mode(RoomMode::Playing);
    }

    fn accept_new_clients(&mut self, net: &mut NetworkManager) -> Result<(), NetError> {
        for client_id in net.connected_client_ids() {
            if self.known_clients.contains(&client_id) {
                continue;
            }
            let Some(slot) = net.slot_for_client(client_id) else {
                continue;
            };
            if slot.0 >= self.max_players {
                continue;
            }
            self.sim.add_player(slot);
            self.known_clients.insert(client_id);

            let accept = PktServerAccept {
                header: PacketHeader::new(PacketType::ServerAccept, 0),
                assigned_player_id: slot.0,
                max_players: self.max_players,
                resume_token: String::new(),
            };
            let data = bincode::serialize(&accept).map_err(|e| NetError::Encode(e.to_string()))?;
            net.send_to_client(client_id, RELIABLE_CHANNEL, data);
        }
        Ok(())
    }

    fn collect_replay_packets(&mut self, net: &mut NetworkManager) -> Result<(), NetError> {
        for client_id in net.connected_client_ids() {
            let Some(slot) = net.slot_for_client(client_id) else {
                continue;
            };
            for data in net.receive_server_messages(client_id, MODEL_B_CHANNEL) {
                let header: PacketHeader = deser(&data)?;
                if header.version != PROTOCOL_VERSION || header.packet_type != PacketType::Replay {
                    continue;
                }
                let pkt: PktReplay = deser(&data)?;
                for event in pkt.events {
                    self.sim.enqueue_input(slot, event);
                }
            }
        }
        Ok(())
    }

    fn dispatch_outbound(net: &mut NetworkManager, outbound: Vec<SimOutbound>) {
        for message in outbound {
            match message {
                SimOutbound::ToPlayer(slot, data) => {
                    if slot == PlayerSlot(0) {
                        continue;
                    }
                    if let Some(client_id) = Self::client_for_slot(net, slot) {
                        net.send_to_client(client_id, MODEL_B_CHANNEL, data);
                    }
                }
                SimOutbound::Broadcast(data) => net.broadcast(MODEL_B_CHANNEL, data),
            }
        }
    }

    fn client_for_slot(net: &NetworkManager, slot: PlayerSlot) -> Option<ClientId> {
        net.connected_client_ids()
            .into_iter()
            .find(|client_id| net.slot_for_client(*client_id) == Some(slot))
    }
}

fn deser<'de, T: serde::Deserialize<'de>>(data: &'de [u8]) -> Result<T, NetError> {
    bincode::DefaultOptions::new()
        .with_fixint_encoding()
        .allow_trailing_bytes()
        .with_limit(MAX_PACKET_BYTES)
        .deserialize::<T>(data)
        .map_err(|e| NetError::Decode(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tetris_protocol::newtypes::{KeyAction, TickNumber};
    use tetris_protocol::protocol::InputEvent;

    fn input_event() -> InputEvent {
        InputEvent {
            key: KeyAction::KeyHardDrop,
            pressed: true,
            tick: TickNumber(0),
            subframe: 0.0,
        }
    }

    #[test]
    fn host_adapter_reuses_authoritative_sim_for_replay() {
        let mut adapter = RenetHostAdapter::new(Seed(42), 2);
        adapter.sim.add_player(PlayerSlot(1));
        let before = adapter.sim.engine(PlayerSlot(1)).unwrap().state.piece;

        adapter.sim.enqueue_input(PlayerSlot(1), input_event());
        adapter.sim.tick().unwrap();

        assert_ne!(
            adapter.sim.engine(PlayerSlot(1)).unwrap().state.piece,
            before
        );
    }

    #[test]
    fn host_adapter_serializes_model_b_outbound() {
        let mut adapter = RenetHostAdapter::new(Seed(42), 2);
        adapter.sim.add_player(PlayerSlot(1));
        adapter.start_playing();

        let outbound = adapter
            .sim
            .replay_broadcast(PlayerSlot(1), &[input_event()])
            .unwrap();

        assert!(outbound.iter().any(|message| {
            let SimOutbound::ToPlayer(PlayerSlot(0), data) = message else {
                return false;
            };
            bincode::deserialize::<tetris_protocol::protocol::PktServerReplay>(data).is_ok()
        }));
    }
}
