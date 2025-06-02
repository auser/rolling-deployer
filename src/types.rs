use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct Image {
    #[serde(rename = "Containers")]
    pub containers: i64,
    #[serde(rename = "Created")]
    pub created: i64,
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "Labels")]
    pub labels: Option<HashMap<String, String>>,
    #[serde(rename = "ParentId")]
    pub parent_id: String,
    #[serde(rename = "RepoDigests")]
    pub repo_digests: Vec<String>,
    #[serde(rename = "RepoTags")]
    pub repo_tags: Vec<String>,
    #[serde(rename = "SharedSize")]
    pub shared_size: i64,
    #[serde(rename = "Size")]
    pub size: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Container {
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "Names")]
    pub names: Vec<String>,
    #[serde(rename = "Image")]
    pub image: String,
    #[serde(rename = "ImageID")]
    pub image_id: String,
    #[serde(rename = "Command")]
    pub command: String,
    #[serde(rename = "Created")]
    pub created: i64,
    #[serde(rename = "Ports")]
    pub ports: Vec<Port>,
    #[serde(rename = "Labels")]
    pub labels: Option<HashMap<String, String>>,
    #[serde(rename = "State")]
    pub state: String,
    #[serde(rename = "Status")]
    pub status: String,
    #[serde(rename = "HostConfig")]
    pub host_config: HostConfig,
    #[serde(rename = "NetworkSettings")]
    pub network_settings: NetworkSettings,
    #[serde(rename = "Mounts")]
    pub mounts: Vec<Mount>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Port {
    #[serde(rename = "IP")]
    pub ip: Option<String>,
    #[serde(rename = "PrivatePort")]
    pub private_port: u16,
    #[serde(rename = "PublicPort")]
    pub public_port: Option<u16>,
    #[serde(rename = "Type")]
    pub port_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HostConfig {
    #[serde(rename = "NetworkMode")]
    pub network_mode: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkSettings {
    #[serde(rename = "Networks")]
    pub networks: HashMap<String, Network>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Network {
    #[serde(rename = "IPAMConfig")]
    pub ipam_config: Option<serde_json::Value>,
    #[serde(rename = "Links")]
    pub links: Option<Vec<String>>,
    #[serde(rename = "Aliases")]
    pub aliases: Option<Vec<String>>,
    #[serde(rename = "NetworkID")]
    pub network_id: String,
    #[serde(rename = "EndpointID")]
    pub endpoint_id: String,
    #[serde(rename = "Gateway")]
    pub gateway: String,
    #[serde(rename = "IPAddress")]
    pub ip_address: String,
    #[serde(rename = "IPPrefixLen")]
    pub ip_prefix_len: u8,
    #[serde(rename = "IPv6Gateway")]
    pub ipv6_gateway: String,
    #[serde(rename = "GlobalIPv6Address")]
    pub global_ipv6_address: String,
    #[serde(rename = "GlobalIPv6PrefixLen")]
    pub global_ipv6_prefix_len: u8,
    #[serde(rename = "MacAddress")]
    pub mac_address: String,
    #[serde(rename = "DriverOpts")]
    pub driver_opts: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Mount {
    #[serde(rename = "Target")]
    pub target: String,
    #[serde(rename = "Source")]
    pub source: String,
    #[serde(rename = "Type")]
    pub mount_type: String,
    #[serde(rename = "Mode")]
    pub mode: String,
    #[serde(rename = "RW")]
    pub rw: bool,
    #[serde(rename = "Propagation")]
    pub propagation: String,
}
