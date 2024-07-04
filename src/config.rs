use std::net::{SocketAddr, ToSocketAddrs};

/// Configuration of the cluster client
#[derive(Debug)]
pub struct ClusterClientConfig {
    pub coord_addr: SocketAddr, // Cluster Coordinator adddress
    pub bypass_pk_check: bool,  // Bypass Server public key check
}

impl ClusterClientConfig {
    /// Creates a new ClusterClientConfig instance with default values
    pub fn new(coord_addr: impl ToSocketAddrs) -> Self {
        Self {
            coord_addr: coord_addr.to_socket_addrs().unwrap().next().unwrap(), // TODO: Add error handling
            bypass_pk_check: false,
        }
    }

    pub fn bypass_pk_check(mut self, val: bool) -> Self {
        self.bypass_pk_check = val;
        self
    }
}
