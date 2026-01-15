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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_app_config_default() {
        let config = AppConfig::default();
        assert_eq!(config.remote_user, "root");
        assert_eq!(config.nodes.len(), 1);
    }

    #[test]
    fn test_load_app_config() -> Result<()> {
        let mut file = NamedTempFile::new()?;
        let json = r#"{
            "remote_user": "test_user",
            "remote_host": "test_host",
            "tmux_session_name": "test_session",
            "vllm_docker_dir": "test_dir",
            "nodes": ["1.1.1.1"]
        }"#;
        writeln!(file, "{}", json)?;

        let config = AppConfig::load_from_file(file.path())?;
        assert_eq!(config.remote_user, "test_user");
        assert_eq!(config.nodes, vec!["1.1.1.1"]);
        Ok(())
    }

    #[test]
    fn test_load_models() -> Result<()> {
        let mut file = NamedTempFile::new()?;
        let json = r#"[
            {
                "name": "test_model",
                "model_id": "test_id",
                "args": ["--arg1"]
            }
        ]"#;
        writeln!(file, "{}", json)?;

        let models = load_models(file.path())?;
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].name, "test_model");
        assert_eq!(models[0].args, vec!["--arg1"]);
        Ok(())
    }
}
