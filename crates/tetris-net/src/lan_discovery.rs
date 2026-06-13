use std::net::SocketAddr;
use std::time::Duration;

use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};

use crate::error::NetError;

const SERVICE_TYPE: &str = "_tetris._udp.local.";

#[derive(Debug, Clone)]
pub struct DiscoveredHost {
    pub name: String,
    pub address: SocketAddr,
    pub player_name: String,
}

pub struct LanDiscovery {
    daemon: ServiceDaemon,
    service_fullname: Option<String>,
}

impl LanDiscovery {
    pub fn new() -> Result<Self, NetError> {
        let daemon = ServiceDaemon::new().map_err(|e| NetError::MdnsError(e.to_string()))?;
        Ok(Self {
            daemon,
            service_fullname: None,
        })
    }

    pub fn publish(&mut self, port: u16, player_name: &str) -> Result<(), NetError> {
        let host = "tetris-host";
        let instance_name = format!("{player_name}@{host}");

        let service = ServiceInfo::new(
            SERVICE_TYPE,
            &instance_name,
            &format!("{host}.local."),
            "",
            port,
            [
                ("name", player_name),
                ("version", env!("CARGO_PKG_VERSION")),
            ]
            .as_slice(),
        )
        .map_err(|e| NetError::MdnsError(e.to_string()))?;

        self.service_fullname = Some(service.get_fullname().to_string());
        self.daemon
            .register(service)
            .map_err(|e| NetError::MdnsError(e.to_string()))?;
        Ok(())
    }

    pub fn browse(&self, timeout: Duration) -> Result<Vec<DiscoveredHost>, NetError> {
        let receiver = self
            .daemon
            .browse(SERVICE_TYPE)
            .map_err(|e| NetError::MdnsError(e.to_string()))?;

        let mut hosts = Vec::new();
        let deadline = std::time::Instant::now() + timeout;

        while std::time::Instant::now() < deadline {
            match receiver.recv_timeout(Duration::from_millis(100)) {
                Ok(ServiceEvent::ServiceResolved(info)) => {
                    let player_name = info
                        .get_properties()
                        .get("name")
                        .map(|v| v.val_str().to_string())
                        .unwrap_or_default();

                    for addr in info.get_addresses() {
                        hosts.push(DiscoveredHost {
                            name: info.get_fullname().to_string(),
                            address: SocketAddr::new(*addr, info.get_port()),
                            player_name: player_name.clone(),
                        });
                    }
                }
                Ok(_) => {}
                Err(_) => {}
            }
        }

        Ok(hosts)
    }

    pub fn unpublish(&mut self) -> Result<(), NetError> {
        if let Some(fullname) = self.service_fullname.take() {
            self.daemon
                .unregister(&fullname)
                .map_err(|e| NetError::MdnsError(e.to_string()))?;
        }
        Ok(())
    }
}

impl Drop for LanDiscovery {
    fn drop(&mut self) {
        let _ = self.unpublish();
        let _ = self.daemon.shutdown();
    }
}
