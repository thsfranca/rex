/// Local Unix Domain Socket used by the daemon.
pub const SOCKET_PATH: &str = "/tmp/rex.sock";

/// Timeout budget for establishing a daemon connection.
pub const CONNECT_TIMEOUT_SECONDS: u64 = 2;

/// Timeout budget for unary RPC operations (status, etc.).
pub const REQUEST_TIMEOUT_SECONDS: u64 = 5;
