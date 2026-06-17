use std::collections::HashMap;
use std::time::Duration;

use renet::{ChannelConfig, ConnectionConfig, RenetClient, RenetServer, SendType};
use serde::Serialize;

use crate::error::NetError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    None,
    Host,
    Client,
}

pub struct NetworkManager {
    pub server: Option<RenetServer>,
    pub client: Option<RenetClient>,
    pub role: Role,
    pub local_player_id: u8,
    pub max_players: u8,
    connected_clients: HashMap<u64, u8>,
    next_client_id: u64,
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
            role: Role::None,
            local_player_id: 0,
            max_players: 2,
            connected_clients: HashMap::new(),
            next_client_id: 1,
        }
    }

    pub fn start_server(&mut self, _port: u16, max_players: u8) -> Result<(), NetError> {
        let server = RenetServer::new(connection_config());
        self.server = Some(server);
        self.role = Role::Host;
        self.local_player_id = 0;
        self.max_players = max_players;
        Ok(())
    }

    /// NOTE: _ip/_port are unused — transport layer is established externally.
    pub fn connect_to_server(&mut self, _ip: &str, _port: u16) -> Result<(), NetError> {
        let client = RenetClient::new(connection_config());
        self.client = Some(client);
        self.role = Role::Client;
        Ok(())
    }

    pub fn disconnect(&mut self) {
        self.server = None;
        self.client = None;
        self.role = Role::None;
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

    pub fn tick(&mut self, duration: Duration) {
        if let Some(ref mut server) = self.server {
            server.update(duration);
            while let Some(event) = server.get_event() {
                use renet::ServerEvent;
                match event {
                    ServerEvent::ClientConnected { client_id } => {
                        if self.connected_clients.len() >= 8 {
                            continue;
                        }
                        let player_id = (self.next_client_id % 256) as u8;
                        self.connected_clients.insert(client_id, player_id);
                        self.next_client_id = self.next_client_id.wrapping_add(1);
                    }
                    ServerEvent::ClientDisconnected { client_id, .. } => {
                        self.connected_clients.remove(&client_id);
                    }
                }
            }
        } else if let Some(ref mut client) = self.client {
            client.update(duration);
        }
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
}

impl Default for NetworkManager {
    fn default() -> Self {
        Self::new()
    }
}
