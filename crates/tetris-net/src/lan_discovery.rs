use std::net::SocketAddr;
use std::time::Duration;

use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};

use crate::error::NetError;

const SERVICE_TYPE: &str = "_tetris._udp.local.";

#[derive(Debug, Clone)]
pub struct DiscoveredHost {
    /// Friendly label advertised by the host (mDNS TXT `name`).
    pub label: String,
    pub address: SocketAddr,
    /// Advertised crate version (mDNS TXT `version`).
    pub version: String,
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

    /// Publish a relay server so LAN clients can discover it. Broadcasts the
    /// `label` (TXT `name`) + crate `version`; the port lives in the SRV record.
    pub fn publish_relay(&mut self, port: u16, label: &str) -> Result<(), NetError> {
        let host = "tetris-relay";

        let service = ServiceInfo::new(
            SERVICE_TYPE,
            label,
            &format!("{host}.local."),
            "",
            port,
            [("name", label), ("version", env!("CARGO_PKG_VERSION"))].as_slice(),
        )
        .map_err(|e| NetError::MdnsError(e.to_string()))?;

        self.service_fullname = Some(service.get_fullname().to_string());
        self.daemon
            .register(service)
            .map_err(|e| NetError::MdnsError(e.to_string()))?;
        Ok(())
    }

    /// Blocking — call from `tokio::task::spawn_blocking` in async contexts.
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
                    let props = info.get_properties();
                    let label = props
                        .get("name")
                        .map(|v| v.val_str().to_string())
                        .filter(|s| !s.is_empty())
                        .unwrap_or_else(|| info.get_fullname().to_string());
                    let version = props
                        .get("version")
                        .map(|v| v.val_str().to_string())
                        .unwrap_or_default();

                    // NOTE: mDNS data from LAN is inherently untrusted — display after sanitization.
                    for addr in info.get_addresses() {
                        hosts.push(DiscoveredHost {
                            label: label.clone(),
                            address: SocketAddr::new(*addr, info.get_port()),
                            version: version.clone(),
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
