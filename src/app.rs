use crate::config::{AppConfig, ModelConfig};
use crate::ssh::SshClient;
use std::sync::{Arc, Mutex};

/// Represents the current SSH connection state.
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectState {
    Disconnected,
    Connecting,
    Connected,
    Failed(String),
}

use std::sync::atomic::AtomicBool;
/// Represents the operational status of the vLLM cluster.
#[derive(Debug, Clone, PartialEq)]
pub enum ClusterStatus {
    Unknown,
    #[allow(dead_code)]
    Checking,
    Running,
    Stopped,
}

/// Updates sent from background tasks to the main UI loop.
#[derive(Debug)]
pub enum StatusUpdate {
    ConnectState(ConnectState, String),
    ClusterStatus(ClusterStatus, String),
    Message(String),
}

/// User or internal actions processed by the application loop.
pub enum Action {
    Tick,
    Quit,
    Connect,
    Disconnect,
    SetPassword(String),
    SelectModel(usize),
    StartCluster,
    StopCluster,
    RefreshStatus,
    TailLogs,
    StopTail,
    AttachSession,
}

/// Core application state.
pub struct App {
    /// Server configuration (host, user, etc.).
    pub config: AppConfig,
    /// List of available models.
    pub models: Vec<ModelConfig>,
    /// Current SSH connection state.
    pub connect_state: ConnectState,
    /// Current cluster running status.
    pub cluster_status: ClusterStatus,
    /// Shared SSH client instance.
    pub ssh_client: Arc<Mutex<Option<SshClient>>>,
    /// Buffer for password input.
    pub password_input: String,
    /// Flag to show the password prompt modal.
    pub show_password_prompt: bool,
    /// Index of the currently selected model.
    pub selected_model_index: usize,
    /// Buffer of log lines to display in the UI.
    pub logs: Vec<String>,
    /// Current status message shown in the footer.
    pub status_message: String,
    /// Atomic flag to signal log tailing cancellation.
    pub is_tailing: Arc<AtomicBool>,
}

impl App {
    /// Creates a new App instance with the provided configuration.
    pub fn new(config: AppConfig, models: Vec<ModelConfig>) -> Self {
        Self {
            config,
            models,
            connect_state: ConnectState::Disconnected,
            cluster_status: ClusterStatus::Unknown,
            ssh_client: Arc::new(Mutex::new(None)),
            password_input: String::new(),
            show_password_prompt: false,
            selected_model_index: 0,
            logs: vec![],
            status_message: "Ready. Press 'c' to connect.".to_string(),
            is_tailing: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Returns the currently selected model if valid.
    pub fn selected_model(&self) -> Option<&ModelConfig> {
        self.models.get(self.selected_model_index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;

    #[test]
    fn test_app_init() {
        let config = AppConfig::default();
        let models = vec![
            ModelConfig {
                name: "model1".to_string(),
                model_id: "id1".to_string(),
                args: vec![],
            }
        ];
        let app = App::new(config, models);
        assert_eq!(app.selected_model_index, 0);
        assert_eq!(app.connect_state, ConnectState::Disconnected);
        assert!(app.selected_model().is_some());
    }

    #[test]
    fn test_app_selection() {
         let config = AppConfig::default();
        let models = vec![
            ModelConfig { name: "m1".into(), model_id: "i1".into(), args: vec![] },
            ModelConfig { name: "m2".into(), model_id: "i2".into(), args: vec![] },
        ];
        let mut app = App::new(config, models);
        app.selected_model_index = 1;
        assert_eq!(app.selected_model().unwrap().name, "m2");
    }
}
