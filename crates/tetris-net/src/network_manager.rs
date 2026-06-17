use std::collections::HashMap;
use std::io::ErrorKind;
use std::net::{SocketAddr, UdpSocket};
use std::time::Duration;

use renet::{ChannelConfig, ClientId, ConnectionConfig, RenetClient, RenetServer, SendType};
use serde::Serialize;
use tetris_protocol::newtypes::PlayerSlot;

use crate::error::NetError;

pub const RELIABLE_CHANNEL: u8 = 0;
pub const MODEL_B_CHANNEL: u8 = 1;
pub const UNRELIABLE_CHANNEL: u8 = 2;
const MAX_PACKET_SIZE: usize = 1500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    None,
    Host,
    Client,
}

pub struct NetworkManager {
    pub server: Option<RenetServer>,
    pub client: Option<RenetClient>,
    server_socket: Option<UdpSocket>,
    client_socket: Option<UdpSocket>,
    pub role: Role,
    pub local_player_id: u8,
    pub max_players: u8,
    server_addr: Option<SocketAddr>,
    client_addrs: HashMap<ClientId, SocketAddr>,
    client_by_addr: HashMap<SocketAddr, ClientId>,
    connected_clients: HashMap<ClientId, PlayerSlot>,
    next_client_id: ClientId,
    next_player_slot: u8,
}

fn channel_config() -> Vec<ChannelConfig> {
    vec![
        ChannelConfig {
            channel_id: 0,
            max_memory_usage_bytes: 5 * 1024 * 1024,
            send_type: SendType::ReliableOrdered {
                resend_time: Duration::from_millis(300),
            },
        },
        ChannelConfig {
            channel_id: 1,
            max_memory_usage_bytes: 5 * 1024 * 1024,
            send_type: SendType::ReliableOrdered {
                resend_time: Duration::from_millis(300),
            },
        },
        ChannelConfig {
            channel_id: 2,
            max_memory_usage_bytes: 5 * 1024 * 1024,
            send_type: SendType::Unreliable,
        },
    ]
}

fn connection_config() -> ConnectionConfig {
    let channels = channel_config();
    ConnectionConfig {
        available_bytes_per_tick: 1024 * 1024,
        client_channels_config: channels.clone(),
        server_channels_config: channels,
    }
}

impl NetworkManager {
    pub fn new() -> Self {
        NetworkManager {
            server: None,
            client: None,
            server_socket: None,
            client_socket: None,
            role: Role::None,
            local_player_id: 0,
            max_players: 2,
            server_addr: None,
            client_addrs: HashMap::new(),
            client_by_addr: HashMap::new(),
            connected_clients: HashMap::new(),
            next_client_id: 1,
            next_player_slot: 1,
        }
    }

    pub fn start_server(&mut self, ip: &str, port: u16, max_players: u8) -> Result<(), NetError> {
        let bind_addr = parse_socket_addr(ip, port)?;
        let socket = UdpSocket::bind(bind_addr).map_err(|e| NetError::Io(e.to_string()))?;
        socket
            .set_nonblocking(true)
            .map_err(|e| NetError::Io(e.to_string()))?;
        let public_addr = socket
            .local_addr()
            .map_err(|e| NetError::Io(e.to_string()))?;

        let server = RenetServer::new(connection_config());

        self.server = Some(server);
        self.server_socket = Some(socket);
        self.client = None;
        self.client_socket = None;
        self.role = Role::Host;
        self.local_player_id = 0;
        self.max_players = max_players;
        self.server_addr = Some(public_addr);
        self.client_addrs.clear();
        self.client_by_addr.clear();
        self.connected_clients.clear();
        self.next_client_id = 1;
        self.next_player_slot = 1;
        Ok(())
    }

    pub fn connect_to_server(&mut self, ip: &str, port: u16) -> Result<(), NetError> {
        let server_addr = parse_socket_addr(ip, port)?;
        let socket = UdpSocket::bind("0.0.0.0:0").map_err(|e| NetError::Io(e.to_string()))?;
        socket
            .set_nonblocking(true)
            .map_err(|e| NetError::Io(e.to_string()))?;
        let mut client = RenetClient::new(connection_config());
        client.set_connected();

        self.client = Some(client);
        self.client_socket = Some(socket);
        self.server = None;
        self.server_socket = None;
        self.role = Role::Client;
        self.server_addr = Some(server_addr);
        Ok(())
    }

    pub fn disconnect(&mut self) {
        self.server = None;
        self.client = None;
        self.server_socket = None;
        self.client_socket = None;
        self.role = Role::None;
        self.client_addrs.clear();
        self.client_by_addr.clear();
        self.connected_clients.clear();
    }

    pub fn send_packet<T: Serialize>(&mut self, packet: &T, channel: u8) -> Result<(), NetError> {
        let data = bincode::serialize(packet).map_err(|e| NetError::Encode(e.to_string()))?;

        if let Some(ref mut server) = self.server {
            for client_id in self.connected_clients.keys() {
                server.send_message(*client_id, channel, data.clone());
            }
        } else if let Some(ref mut client) = self.client {
            client.send_message(channel, data);
        }
        Ok(())
    }

    pub fn tick(&mut self, duration: Duration) -> Result<(), NetError> {
        // renet 2.0 exposes connection packet pumps but leaves socket transport to callers.
        if self.server.is_some() && self.server_socket.is_some() {
            self.pump_server_incoming()?;
            if let Some(server) = self.server.as_mut() {
                server.update(duration);
            }
            self.pump_server_outgoing()?;
        } else if self.client.is_some() && self.client_socket.is_some() {
            self.pump_client_incoming()?;
            if let Some(client) = self.client.as_mut() {
                client.update(duration);
            }
            self.pump_client_outgoing()?;
        }
        Ok(())
    }

    pub fn receive_messages(&mut self, channel: u8) -> Vec<Vec<u8>> {
        let mut messages = Vec::new();
        if let Some(ref mut client) = self.client {
            while let Some(msg) = client.receive_message(channel) {
                messages.push(msg.to_vec());
            }
        }
        messages
    }

    pub fn receive_server_messages(&mut self, client_id: ClientId, channel: u8) -> Vec<Vec<u8>> {
        let mut messages = Vec::new();
        if let Some(ref mut server) = self.server {
            while let Some(msg) = server.receive_message(client_id, channel) {
                messages.push(msg.to_vec());
            }
        }
        messages
    }

    pub fn send_to_client(&mut self, client_id: ClientId, channel: u8, data: Vec<u8>) {
        if let Some(ref mut server) = self.server {
            server.send_message(client_id, channel, data);
        }
    }

    pub fn broadcast(&mut self, channel: u8, data: Vec<u8>) {
        if let Some(ref mut server) = self.server {
            server.broadcast_message(channel, data);
        } else if let Some(ref mut client) = self.client {
            client.send_message(channel, data);
        }
    }

    pub fn connected_client_ids(&self) -> Vec<ClientId> {
        self.connected_clients.keys().copied().collect()
    }

    pub fn slot_for_client(&self, client_id: ClientId) -> Option<PlayerSlot> {
        self.connected_clients.get(&client_id).copied()
    }

    pub fn server_addr(&self) -> Option<SocketAddr> {
        self.server_addr
    }

    pub fn role(&self) -> Role {
        self.role
    }

    pub fn is_connected(&self) -> bool {
        self.server.is_some() || self.client.is_some()
    }

    pub fn local_player_id(&self) -> u8 {
        self.local_player_id
    }

    pub fn max_players(&self) -> u8 {
        self.max_players
    }

    pub fn connected_count(&self) -> usize {
        self.connected_clients.len()
    }

    fn pump_server_incoming(&mut self) -> Result<(), NetError> {
        let mut buffer = [0u8; MAX_PACKET_SIZE];
        loop {
            let result = self
                .server_socket
                .as_ref()
                .ok_or(NetError::Disconnected)?
                .recv_from(&mut buffer);
            let (len, addr) = match result {
                Ok(result) => result,
                Err(err) if err.kind() == ErrorKind::WouldBlock => return Ok(()),
                Err(err) => return Err(NetError::Io(err.to_string())),
            };
            let client_id = self.ensure_client(addr);
            if let Some(server) = self.server.as_mut() {
                server
                    .process_packet_from(&buffer[..len], client_id)
                    .map_err(|e| NetError::Connection(e.to_string()))?;
            }
        }
    }

    fn pump_server_outgoing(&mut self) -> Result<(), NetError> {
        let Some(server) = self.server.as_mut() else {
            return Ok(());
        };
        let socket = self.server_socket.as_ref().ok_or(NetError::Disconnected)?;
        for (client_id, addr) in &self.client_addrs {
            for packet in server
                .get_packets_to_send(*client_id)
                .map_err(|e| NetError::Connection(e.to_string()))?
            {
                socket
                    .send_to(&packet, addr)
                    .map_err(|e| NetError::Io(e.to_string()))?;
            }
        }
        Ok(())
    }

    fn pump_client_incoming(&mut self) -> Result<(), NetError> {
        let mut buffer = [0u8; MAX_PACKET_SIZE];
        loop {
            let result = self
                .client_socket
                .as_ref()
                .ok_or(NetError::Disconnected)?
                .recv_from(&mut buffer);
            let (len, _addr) = match result {
                Ok(result) => result,
                Err(err) if err.kind() == ErrorKind::WouldBlock => return Ok(()),
                Err(err) => return Err(NetError::Io(err.to_string())),
            };
            if let Some(client) = self.client.as_mut() {
                client.process_packet(&buffer[..len]);
            }
        }
    }

    fn pump_client_outgoing(&mut self) -> Result<(), NetError> {
        let server_addr = self.server_addr.ok_or(NetError::Disconnected)?;
        let socket = self.client_socket.as_ref().ok_or(NetError::Disconnected)?;
        let Some(client) = self.client.as_mut() else {
            return Ok(());
        };
        for packet in client.get_packets_to_send() {
            socket
                .send_to(&packet, server_addr)
                .map_err(|e| NetError::Io(e.to_string()))?;
        }
        Ok(())
    }

    fn ensure_client(&mut self, addr: SocketAddr) -> ClientId {
        if let Some(client_id) = self.client_by_addr.get(&addr) {
            return *client_id;
        }
        let client_id = self.next_client_id;
        self.next_client_id = self.next_client_id.wrapping_add(1);
        self.client_by_addr.insert(addr, client_id);
        self.client_addrs.insert(client_id, addr);
        if let Some(server) = self.server.as_mut() {
            server.add_connection(client_id);
        }
        if self.connected_clients.len() < usize::from(self.max_players) {
            let slot = PlayerSlot(self.next_player_slot);
            self.connected_clients.insert(client_id, slot);
            self.next_player_slot = self.next_player_slot.wrapping_add(1);
        }
        client_id
    }
}

impl Default for NetworkManager {
    fn default() -> Self {
        Self::new()
    }
}

fn parse_socket_addr(ip: &str, port: u16) -> Result<SocketAddr, NetError> {
    format!("{ip}:{port}")
        .parse()
        .map_err(|e| NetError::Connection(format!("invalid socket address {ip}:{port}: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn network_manager_bind_connect_uses_real_socket_transport() {
        let mut server = NetworkManager::new();
        server.start_server("127.0.0.1", 0, 2).unwrap();
        let addr = server.server_addr().unwrap();

        let mut client = NetworkManager::new();
        client
            .connect_to_server(&addr.ip().to_string(), addr.port())
            .unwrap();

        assert_eq!(server.role(), Role::Host);
        assert_eq!(client.role(), Role::Client);
        assert!(server.server_socket.is_some());
        assert!(client.client_socket.is_some());
    }
}
