/// Local Unix Domain Socket used by the daemon.
pub const SOCKET_PATH: &str = "/tmp/rex.sock";

/// Timeout budget for establishing a daemon connection.
pub const CONNECT_TIMEOUT_SECONDS: u64 = 2;

/// Timeout budget for unary RPC operations.
pub const REQUEST_TIMEOUT_SECONDS: u64 = 5;

/// Timeout budget for receiving each stream item from daemon.
pub const STREAM_ITEM_TIMEOUT_SECONDS: u64 = 15;
