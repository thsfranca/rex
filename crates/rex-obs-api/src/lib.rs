mod error;
mod server;

pub use error::ReadApiError;
pub use server::{serve, ReadApiState};
