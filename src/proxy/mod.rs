pub mod macos;
pub mod ssh;

pub use macos::{disable_socks_proxy, enable_socks_proxy, is_socks_proxy_enabled};
pub use ssh::{find_ssh_pid, start_ssh_tunnel, stop_ssh_tunnel, stop_ssh_tunnel_by_port, wait_for_tunnel};
