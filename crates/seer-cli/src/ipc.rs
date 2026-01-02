//! IPC between CLI and daemon

use anyhow::Result;

/// IPC client for communicating with daemon
pub struct IpcClient {
    // TODO: Add unix socket or similar
}

impl IpcClient {
    /// Connect to daemon
    pub async fn connect() -> Result<Self> {
        // TODO: Connect to .snap/state/daemon.sock
        todo!("Implement IpcClient::connect")
    }

    /// Send a message to daemon
    pub async fn send(&mut self, message: &str) -> Result<String> {
        // TODO: Send message and receive response
        todo!("Implement IpcClient::send")
    }
}

/// IPC server for daemon
pub struct IpcServer {
    // TODO: Add unix socket server
}

impl IpcServer {
    /// Start IPC server
    pub async fn start() -> Result<Self> {
        // TODO: Listen on .snap/state/daemon.sock
        todo!("Implement IpcServer::start")
    }

    /// Handle incoming messages
    pub async fn handle_message(&self, message: &str) -> Result<String> {
        // TODO: Process message and return response
        todo!("Implement IpcServer::handle_message")
    }
}
