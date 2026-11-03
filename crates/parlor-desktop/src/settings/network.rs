// SPDX-License-Identifier: GPL-3.0-or-later
//! Where the receiver listens, and whether it announces itself.

use serde::{Deserialize, Serialize};

use super::value::Problem;

/// How the media ports are chosen.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PortsMode {
    /// Every port chosen by the operating system, which is what discovery needs.
    #[default]
    Automatic,
    /// The layout the protocol's own receiver binds, for a firewall or a port forward.
    Legacy,
    /// Four ports the user names.
    Custom,
}

/// Where the receiver listens.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct NetworkSettings {
    /// The TCP port the control service listens on.
    pub port: u16,
    /// The address the control and media sockets bind to.
    pub bind: String,
    /// Whether the receiver appears in a client's menu.
    pub advertise: bool,
    /// How the four media ports are chosen.
    pub ports: PortsMode,
    /// The UDP timing port, when [ports](Self::ports) is custom.
    pub timing: u16,
    /// The UDP control port, when [ports](Self::ports) is custom.
    pub control: u16,
    /// The UDP data port, when [ports](Self::ports) is custom.
    pub data: u16,
    /// The TCP mirror port, when [ports](Self::ports) is custom.
    pub mirror: u16,
}

impl Default for NetworkSettings {
    fn default() -> Self {
        Self {
            port: 7000,
            bind: "0.0.0.0".to_owned(),
            advertise: true,
            ports: PortsMode::Automatic,
            timing: 7011,
            control: 6001,
            data: 6000,
            mirror: 7100,
        }
    }
}

impl NetworkSettings {
    /// The address to bind, or None when the text is not an address.
    pub fn bind_address(&self) -> Option<std::net::IpAddr> {
        self.bind.trim().parse().ok()
    }

    /// The media ports the receiver binds.
    pub fn media_ports(&self) -> rust_airplay::media::ports::MediaPorts {
        match self.ports {
            PortsMode::Automatic => rust_airplay::media::ports::MediaPorts::automatic(),
            PortsMode::Legacy => rust_airplay::media::ports::MediaPorts::legacy(),
            PortsMode::Custom => rust_airplay::media::ports::MediaPorts {
                timing: self.timing,
                control: self.control,
                data: self.data,
                mirror: self.mirror,
            },
        }
    }

    /// The problems with these values, if any.
    pub fn problems(&self) -> Vec<Problem> {
        let mut problems = Vec::new();
        if self.port == 0 {
            problems.push(Problem::new("network", "port", "must be between 1 and 65535"));
        }
        if self.bind_address().is_none() {
            problems.push(Problem::new("network", "bind", "must be an IPv4 or IPv6 address"));
        }
        if self.ports == PortsMode::Custom {
            for (field, port) in [
                ("timing", self.timing),
                ("control", self.control),
                ("data", self.data),
                ("mirror", self.mirror),
            ] {
                if port == 0 {
                    problems.push(Problem::new("network", field, "must be between 1 and 65535"));
                }
            }
        }
        problems
    }
}
