
pub mod config;
pub mod service;
pub mod frpc;
pub mod msg;
pub mod crypto;
pub mod control;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const FRP_VERSION: &str = "0.44.0";

pub const PAYLOAD_SIZE: usize = 128 * 1024;
