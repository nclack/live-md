use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;

/// Configuration for the live-md server
#[derive(Debug, Clone)]
pub struct Config {
    /// The directory containing markdown files
    pub content_dir: PathBuf,

    /// The directory where HTML files will be generated
    pub output_dir: PathBuf,

    /// The port to run the server on
    pub port: u16,

    /// The IP address to bind to
    pub host: IpAddr,

    /// Whether to automatically open the browser when starting
    pub open_browser: bool,

    /// The number of events to buffer in the broadcast channel
    pub broadcast_capacity: usize,
}

impl Config {
    /// Creates a new Config with custom settings
    pub fn new(
        content_dir: PathBuf,
        output_dir: PathBuf,
        port: u16,
        host: IpAddr,
        open_browser: bool,
        broadcast_capacity: usize,
    ) -> Self {
        Self {
            content_dir,
            output_dir,
            port,
            host,
            open_browser,
            broadcast_capacity,
        }
    }

    /// Gets the server's socket address
    pub fn socket_addr(&self) -> SocketAddr {
        SocketAddr::new(self.host, self.port)
    }

    /// Gets the server's URL
    pub fn server_url(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            content_dir: PathBuf::from("doc"),
            output_dir: PathBuf::from("_dist"),
            port: 3000,
            host: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            open_browser: true,
            broadcast_capacity: 16,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_config_new() {
        let content_dir = PathBuf::from("content");
        let output_dir = PathBuf::from("output");
        let port = 8080;
        let host = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let open_browser = false;
        let broadcast_capacity = 32;

        let config = Config::new(
            content_dir.clone(),
            output_dir.clone(),
            port,
            host,
            open_browser,
            broadcast_capacity,
        );

        assert_eq!(config.content_dir, content_dir);
        assert_eq!(config.output_dir, output_dir);
        assert_eq!(config.port, port);
        assert_eq!(config.host, host);
        assert_eq!(config.open_browser, open_browser);
        assert_eq!(config.broadcast_capacity, broadcast_capacity);
    }

    #[test]
    fn test_config_socket_addr() {
        let config = Config::new(
            PathBuf::from("content"),
            PathBuf::from("output"),
            8080,
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            true,
            16,
        );

        let addr = config.socket_addr();
        assert_eq!(addr.port(), 8080);
        assert_eq!(addr.ip(), IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
    }

    #[test]
    fn test_config_server_url() {
        let config = Config::new(
            PathBuf::from("content"),
            PathBuf::from("output"),
            8080,
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            true,
            16,
        );

        assert_eq!(config.server_url(), "http://127.0.0.1:8080");
    }

    #[test]
    fn test_default_config() {
        let config = Config::default();

        assert_eq!(config.port, 3000);
        assert_eq!(config.host, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
        assert_eq!(config.content_dir, PathBuf::from("doc"));
        assert_eq!(config.output_dir, PathBuf::from("_dist"));
        assert_eq!(config.broadcast_capacity, 16);
        assert!(config.open_browser);
    }
}
