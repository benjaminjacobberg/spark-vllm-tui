use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use anyhow::Result;

/// Configuration for a specific vLLM model.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ModelConfig {
    /// Friendly name for the UI.
    pub name: String,
    /// The model identifier (e.g., HuggingFace ID).
    pub model_id: String,
    /// Arguments passed directly to vllm serve.
    pub args: Vec<String>,
}

/// Global application and server configuration.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppConfig {
    /// SSH username for the remote host.
    pub remote_user: String,
    /// SSH hostname or IP.
    pub remote_host: String,
    /// The base name for tmux sessions.
    pub tmux_session_name: String,
    /// Directory on the remote host containing the launch scripts.
    pub vllm_docker_dir: String,
    /// List of worker node IPs for the cluster.
    pub nodes: Vec<String>,
}

impl AppConfig {
    /// Loads the server configuration from a JSON file.
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config = serde_json::from_str(&content)?;
        Ok(config)
    }
}

/// Loads the list of available models from a JSON file.
pub fn load_models<P: AsRef<Path>>(path: P) -> Result<Vec<ModelConfig>> {
    let content = fs::read_to_string(path)?;
    let models = serde_json::from_str(&content)?;
    Ok(models)
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            remote_user: "root".to_string(),
            remote_host: "localhost".to_string(),
            tmux_session_name: "vllm-cluster".to_string(),
            vllm_docker_dir: "spark-vllm-docker".to_string(),
            nodes: vec!["127.0.0.1".to_string()],
        }
    }
}
