use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io::{self, Write}, time::Duration};
use tokio::sync::mpsc;

mod app;
mod config;
mod ssh;
mod ui;

use app::{App, Action, ConnectState, ClusterStatus, StatusUpdate};
use config::{AppConfig, load_models};
use ssh::SshClient;

/// Messages for updating the log view in the TUI.
#[derive(Debug)]
pub enum LogMessage {
    /// Append a single line to the log buffer.
    Line(String),
    /// Replace the entire log buffer with a snapshot (e.g., from tmux capture-pane).
    Snapshot(String),
}

/// Entry point for the vLLM Cluster TUI.
///
/// Initializes configuration, terminal state, and starts the main event loop.
#[tokio::main]
async fn main() -> Result<()> {
    // Load server and node configuration
    let config = AppConfig::load_from_file("server.json").unwrap_or_else(|_| {
        AppConfig::default()
    });
    
    // Load model definitions
    let models = load_models("models.json").unwrap_or_else(|_| {
        vec![]
    });

    // Initialize the terminal for TUI use
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(config, models);

    // Communicate between background tasks and the UI loop
    let (tx, mut rx) = mpsc::channel::<Action>(32);
    let (log_tx, mut log_rx) = mpsc::channel::<LogMessage>(100);
    let (status_tx, mut status_rx) = mpsc::channel::<StatusUpdate>(10);
    
    // Spawn a periodic tick task for UI refreshes and status polling
    let tx_tick = tx.clone();
    tokio::spawn(async move {
        let mut count = 0;
        loop {
            tokio::time::sleep(Duration::from_millis(250)).await;
            if tx_tick.send(Action::Tick).await.is_err() {
                break;
            }
            count += 1;
            // Auto-refresh cluster status every 5 seconds
            if count >= 20 {
                let _ = tx_tick.send(Action::RefreshStatus).await;
                count = 0;
            }
        }
    });

    // Run the main application event loop
    let res = run_app(&mut terminal, &mut app, tx, &mut rx, log_tx, &mut log_rx, status_tx, &mut status_rx).await;

    // Restore terminal state on exit
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Application Error: {:?}", err);
    }

    Ok(())
}

/// The main application loop handling UI rendering, input events, and background updates.
async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    tx: mpsc::Sender<Action>,
    rx: &mut mpsc::Receiver<Action>,
    log_tx: mpsc::Sender<LogMessage>,
    log_rx: &mut mpsc::Receiver<LogMessage>,
    status_tx: mpsc::Sender<StatusUpdate>,
    status_rx: &mut mpsc::Receiver<StatusUpdate>,
) -> Result<()> 
where 
    <B as ratatui::backend::Backend>::Error: Send + Sync + 'static,
    B: std::io::Write
{
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        // Process status updates from background tasks
        while let Ok(update) = status_rx.try_recv() {
            match update {
                StatusUpdate::ConnectState(state, msg) => {
                    if let ConnectState::Failed(ref e) = state {
                        if e.contains("Password required") || e.contains("Authentication failed") {
                            app.show_password_prompt = true;
                            app.password_input.clear();
                        }
                    }
                    app.connect_state = state;
                    app.status_message = msg;
                },
                StatusUpdate::ClusterStatus(status, msg) => {
                    app.cluster_status = status;
                    app.status_message = msg;
                },
                StatusUpdate::Message(msg) => {
                    app.status_message = msg;
                }
            }
        }

        // Process log messages
        while let Ok(msg) = log_rx.try_recv() {
            match msg {
                LogMessage::Line(line) => {
                    app.logs.push(line);
                    if app.logs.len() > 1000 {
                        app.logs.remove(0);
                    }
                }
                LogMessage::Snapshot(content) => {
                    app.logs = content.lines().map(|s| s.to_string()).collect();
                }
            }
        }

        tokio::select! {
            // Process user and system actions
            action = rx.recv() => {
                if let Some(action) = action {
                    match action {
                        Action::Quit => {
                            app.ssh_client.lock().unwrap().take();
                            return Ok(());
                        },
                        Action::Tick => {}, 
                        Action::Connect => {
                             if app.connect_state == ConnectState::Connected {
                                 continue;
                             }
                             let config = app.config.clone();
                             let ssh_arc = app.ssh_client.clone();
                             let s_tx = status_tx.clone();
                             let pwd = if !app.password_input.is_empty() { Some(app.password_input.clone()) } else { None };
                             
                             app.connect_state = ConnectState::Connecting;
                             app.status_message = "Connecting...".into();

                             tokio::task::spawn_blocking(move || {
                                 match SshClient::connect(&config.remote_host, &config.remote_user, pwd.as_deref()) {
                                     Ok(client) => {
                                         let mut guard = ssh_arc.lock().unwrap();
                                         *guard = Some(client);
                                         let _ = s_tx.blocking_send(StatusUpdate::ConnectState(ConnectState::Connected, "Connected successfully. Checking status...".into()));
                                     }
                                     Err(e) => {
                                         let _ = s_tx.blocking_send(StatusUpdate::ConnectState(ConnectState::Failed(e.to_string()), format!("Error: {}", e)));
                                     }
                                 }
                             });
                        },
                        Action::Disconnect => {
                             app.ssh_client.lock().unwrap().take();
                             app.connect_state = ConnectState::Disconnected;
                             app.status_message = "Disconnected.".into();
                             app.cluster_status = ClusterStatus::Unknown;
                        },
                         Action::SelectModel(i) => {
                            if i < app.models.len() {
                                app.selected_model_index = i;
                                tx.send(Action::RefreshStatus).await?;
                            }
                        },
                        Action::SetPassword(p) => {
                            app.password_input = p;
                            app.show_password_prompt = false;
                            tx.send(Action::Connect).await?;
                        },
                        Action::StartCluster => {
                            handle_start_cluster(app, log_tx.clone(), status_tx.clone()).await;
                        },
                        Action::StopCluster => {
                             handle_stop_cluster(app, log_tx.clone(), status_tx.clone()).await;
                        },
                        Action::RefreshStatus => {
                            handle_refresh_status(app, status_tx.clone()).await;
                        },
                        Action::TailLogs => {
                            handle_tail_logs(app, log_tx.clone()).await;
                        },
                        Action::StopTail => {
                            app.is_tailing.store(false, std::sync::atomic::Ordering::Relaxed);
                            let _ = log_tx.send(LogMessage::Line("Stopping log watch...".into())).await;
                        },
                        Action::AttachSession => {
                             let _ = log_tx.send(LogMessage::Line("Attach requested (Suspending TUI)...".into())).await;
                             handle_attach_session(app, log_tx.clone()).await;
                        },
                    }
                }
            }
            // Poll for terminal input
            _ = tokio::time::sleep(Duration::from_millis(10)) => {
                 if event::poll(Duration::from_millis(10))? {
                    if let Event::Key(key) = event::read()? {
                         if key.kind == KeyEventKind::Press {
                             if app.show_password_prompt {
                                 match key.code {
                                     KeyCode::Enter => {
                                         tx.send(Action::SetPassword(app.password_input.clone())).await?;
                                     }
                                     KeyCode::Esc => {
                                         app.show_password_prompt = false;
                                         app.password_input.clear();
                                     }
                                     KeyCode::Backspace => {
                                         app.password_input.pop();
                                     }
                                     KeyCode::Char(c) => {
                                         app.password_input.push(c);
                                     }
                                     _ => {}
                                 }
                             } else {
                                 match key.code {
                                     KeyCode::Char('q') => tx.send(Action::Quit).await?,
                                     KeyCode::Char('c') => {
                                         if let ConnectState::Failed(ref e) = app.connect_state {
                                             if e.contains("Password required") || e.contains("Authentication failed") {
                                                 app.show_password_prompt = true;
                                                 app.password_input.clear();
                                                 continue;
                                             }
                                         }
                                         tx.send(Action::Connect).await?;
                                     }, 
                                     KeyCode::Char('d') => tx.send(Action::Disconnect).await?,
                                     KeyCode::Char('s') => tx.send(Action::StartCluster).await?,
                                     KeyCode::Char('x') => tx.send(Action::StopCluster).await?,
                                     KeyCode::Char('t') => tx.send(Action::TailLogs).await?,
                                     KeyCode::Char('T') => tx.send(Action::StopTail).await?,
                                     KeyCode::Char('a') => tx.send(Action::AttachSession).await?,
                                     KeyCode::Char('r') => tx.send(Action::RefreshStatus).await?, 
                                     KeyCode::Down | KeyCode::Char('j') => {
                                         if app.selected_model_index + 1 < app.models.len() {
                                            tx.send(Action::SelectModel(app.selected_model_index + 1)).await?;
                                         }
                                     },
                                     KeyCode::Up | KeyCode::Char('k') => {
                                         if app.selected_model_index > 0 {
                                            tx.send(Action::SelectModel(app.selected_model_index - 1)).await?;
                                         }
                                     },
                                     _ => {}
                                 }
                             }
                         }
                    }
                 }
            }
        }
    }
}

/// Generates a tmux session name based on the model name.
fn get_session_name(model_name: &str) -> String {
    let mut sanitized = model_name
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>();
    
    while sanitized.contains("--") {
        sanitized = sanitized.replace("--", "-");
    }
    let sanitized = sanitized.trim_matches('-');
    
    format!("vllm-{}", sanitized)
}

/// Periodically checks the status of the remote tmux session for the selected model.
async fn handle_refresh_status(app: &mut App, status_tx: mpsc::Sender<StatusUpdate>) {
    if app.connect_state != ConnectState::Connected {
        return;
    }
    
    let ssh_arc = app.ssh_client.clone();
    
    let session_name = if let Some(model) = app.selected_model() {
        get_session_name(&model.name)
    } else {
        app.config.tmux_session_name.clone()
    };
    
    tokio::task::spawn_blocking(move || {
        let guard = ssh_arc.lock().unwrap();
        if let Some(client) = guard.as_ref() {
            let cmd = format!("tmux has-session -t {}", session_name);
            match client.run_command(&cmd) {
                Ok((_, _, code)) => {
                    if code == 0 {
                        let _ = status_tx.blocking_send(StatusUpdate::ClusterStatus(ClusterStatus::Running, format!("Session {} is active.", session_name)));
                    } else {
                        let _ = status_tx.blocking_send(StatusUpdate::ClusterStatus(ClusterStatus::Stopped, format!("Session {} not found.", session_name)));
                    }
                },
                Err(e) => {
                     let _ = status_tx.blocking_send(StatusUpdate::Message(format!("Status check error: {}", e)));
                }
            }
        }
    });
}

/// Initiates the cluster shutdown sequence on the remote host.
async fn handle_stop_cluster(app: &mut App, log_tx: mpsc::Sender<LogMessage>, status_tx: mpsc::Sender<StatusUpdate>) {
     if app.connect_state != ConnectState::Connected {
        app.status_message = "Not connected!".into();
        return;
    }
    
    let ssh_arc = app.ssh_client.clone();
    let config = app.config.clone();
    let is_tailing = app.is_tailing.clone();
    
    let session_name = if let Some(model) = app.selected_model() {
        get_session_name(&model.name)
    } else {
        config.tmux_session_name.clone()
    };

    let nodes_arg = config.nodes.join(",");

    app.status_message = format!("Stopping cluster ({})...", session_name);

    tokio::task::spawn_blocking(move || {
         let guard = ssh_arc.lock().unwrap();
         if let Some(client) = guard.as_ref() {
             let stop_cmd = format!("cd {} && ./launch-cluster.sh --nodes \"{}\" stop", config.vllm_docker_dir, nodes_arg);
             let _ = log_tx.blocking_send(LogMessage::Line(format!("CMD: {}", stop_cmd)));
             let _ = client.run_command(&stop_cmd);
             
             let kill_cmd = format!("tmux kill-session -t {}", session_name);
             let _ = log_tx.blocking_send(LogMessage::Line(format!("CMD: {}", kill_cmd)));
             let res = client.run_command(&kill_cmd);
              match res {
                 Ok((_, _, _)) => { 
                     let _ = log_tx.blocking_send(LogMessage::Line("Cluster stopped.".into()));
                     let _ = status_tx.blocking_send(StatusUpdate::ClusterStatus(ClusterStatus::Stopped, "Cluster Stopped.".into()));
                     is_tailing.store(false, std::sync::atomic::Ordering::Relaxed);
                 },
                 Err(e) => { 
                     let _ = log_tx.blocking_send(LogMessage::Line(format!("Error stopping: {}", e))); 
                     let _ = status_tx.blocking_send(StatusUpdate::Message("Stop Error.".into()));
                     is_tailing.store(false, std::sync::atomic::Ordering::Relaxed);
                 }
             }
         }
    });
}

/// Launches the vLLM cluster inside a remote tmux session using the selected model configuration.
async fn handle_start_cluster(app: &mut App, log_tx: mpsc::Sender<LogMessage>, status_tx: mpsc::Sender<StatusUpdate>) {
    if app.connect_state != ConnectState::Connected {
        app.status_message = "Not connected!".into();
        return;
    }
    
    let ssh_arc = app.ssh_client.clone();
    let config = app.config.clone();
    let nodes_arg = config.nodes.join(",");
    
    if let Some(model) = app.selected_model() {
         let model_id = model.model_id.clone();
         let args = model.args.join(" ");
         let remote_cmd_inner = format!(
             "cd {} && ./launch-cluster.sh --nodes \"{}\" exec vllm serve {} {}",
             config.vllm_docker_dir, nodes_arg, model_id, args
         );
         
         let session_name = get_session_name(&model.name);
         
         let tmux_cmd = format!(
            "tmux new-session -d -s {} '{}'",
            session_name, remote_cmd_inner
         );

         app.status_message = format!("Starting cluster with model {} (Session: {})...", model.name, session_name);
         let _ = log_tx.send(LogMessage::Line(format!("CMD: {}", tmux_cmd))).await;

         tokio::task::spawn_blocking(move || {
             let guard = ssh_arc.lock().unwrap();
             if let Some(client) = guard.as_ref() {
                 match client.run_command(&tmux_cmd) {
                     Ok((out, err, code)) => {
                         let _ = log_tx.blocking_send(LogMessage::Line(format!("Exited with {}. Out: {}. Err: {}", code, out, err)));
                         let _ = status_tx.blocking_send(StatusUpdate::ClusterStatus(ClusterStatus::Running, "Cluster Started (Tmux session created).".into()));
                     }
                     Err(e) => {
                         let _ = log_tx.blocking_send(LogMessage::Line(format!("Execution Error: {}", e)));
                         let _ = status_tx.blocking_send(StatusUpdate::Message("Start Error.".into()));
                     }
                 }
             }
         });
    }
}

/// Begins polling the remote tmux session output to display logs in the TUI logs panel.
async fn handle_tail_logs(app: &mut App, log_tx: mpsc::Sender<LogMessage>) {
    if app.connect_state != ConnectState::Connected { return; }
    
    app.is_tailing.store(true, std::sync::atomic::Ordering::Relaxed);
    
    let ssh_arc = app.ssh_client.clone();
    let config = app.config.clone();
    let is_tailing = app.is_tailing.clone();
    
     let session_name = if let Some(model) = app.selected_model() {
        get_session_name(&model.name)
    } else {
        config.tmux_session_name.clone()
    };
    
    app.status_message = format!("Watching logs ({}) ...", session_name);
    
    tokio::task::spawn_blocking(move || {
        let cmd = format!("tmux capture-pane -pt {} -S -200", session_name);
        
        loop {
            if !is_tailing.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }
            
            let guard = ssh_arc.lock().unwrap();
            if let Some(client) = guard.as_ref() {
                 match client.run_command(&cmd) {
                     Ok((out, _, _)) => {
                         if !out.trim().is_empty() {
                            let _ = log_tx.blocking_send(LogMessage::Snapshot(out));
                         } else {
                            let _ = log_tx.blocking_send(LogMessage::Line(format!("... waiting for session {} ...", session_name)));
                         }
                     }
                     Err(e) => {
                         let _ = log_tx.blocking_send(LogMessage::Line(format!("Watch error: {}", e)));
                     }
                 }
            } else {
                break; 
            }
            drop(guard); 
            std::thread::sleep(Duration::from_millis(1000));
        }
    });
}

/// Suspends the TUI and shells out to `ssh` to attach to the remote tmux session.
async fn handle_attach_session(app: &mut App, log_tx: mpsc::Sender<LogMessage>) {
    // Suspend TUI
    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
    let _ = io::stdout().flush(); 
    let _ = io::stdout().write_all(b"\x1b[?25h"); 
    
    let config = app.config.clone();
    let session_name = if let Some(model) = app.selected_model() {
        get_session_name(&model.name)
    } else {
        config.tmux_session_name.clone()
    };

    let mut cmd = std::process::Command::new("ssh");
    cmd.arg("-t").arg(format!("{}@{}", config.remote_user, config.remote_host))
       .arg("tmux").arg("attach").arg("-t").arg(&session_name);
    
    let status = cmd.status(); 
    
    if let Err(e) = status {
        let _ = log_tx.send(LogMessage::Line(format!("Attach failed: {}", e))).await;
    }

    // Resume TUI
    let _ = enable_raw_mode();
    let _ = execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture);
    let _ = io::stdout().write_all(b"\x1b[?25l"); 
    let _ = io::stdout().flush(); 
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_session_name() {
        assert_eq!(get_session_name("MiniMax-M2.1"), "vllm-minimax-m2-1");
        assert_eq!(get_session_name("llama3-70b"), "vllm-llama3-70b");
        assert_eq!(get_session_name("DeepSeek-V3 (AWQ)"), "vllm-deepseek-v3-awq");
    }
}
