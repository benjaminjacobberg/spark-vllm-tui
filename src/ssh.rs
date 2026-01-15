use std::net::TcpStream;
use ssh2::Session;
use anyhow::{Context, Result, anyhow};
use std::io::{Read, BufReader, BufRead};

/// A wrapper around libssh2 to handle remote command execution and authentication.
pub struct SshClient {
    session: Session,
    _tcp: TcpStream, // Keep stream alive
}

impl SshClient {
    /// Connects to a remote host using SSH.
    ///
    /// The connection sequence is:
    /// 1. Try SSH agent identities.
    /// 2. Try default public key files in `~/.ssh/`.
    /// 3. Try password authentication if provided.
    pub fn connect(host: &str, user: &str, password: Option<&str>) -> Result<Self> {
        let tcp = TcpStream::connect(format!("{}:22", host))
            .with_context(|| format!("Failed to connect to {}:22", host))?;
        
        let mut session = Session::new()?;
        session.set_tcp_stream(tcp.try_clone()?);
        session.handshake()?;

        let mut authenticated = false;

        // Try agent
        {
            let mut agent = session.agent()?;
            if agent.connect().is_ok() {
                agent.list_identities()?;
                for identity in agent.identities()? {
                    if agent.userauth(user, &identity).is_ok() {
                        authenticated = true;
                        break;
                    }
                }
            }
        }

        // Try standard pubkey files
        if !authenticated {
            if let Some(user_dirs) = directories::UserDirs::new() {
                let ssh_dir = user_dirs.home_dir().join(".ssh");
                let keys = ["id_rsa", "id_ed25519", "id_ecdsa"];
                for key in keys {
                    let key_path = ssh_dir.join(key);
                    if key_path.exists() {
                        if session.userauth_pubkey_file(user, None, &key_path, None).is_ok() {
                            authenticated = true;
                            break;
                        }
                    }
                }
            }
        }

        // Try password
        if !authenticated {
            if let Some(pwd) = password {
                if session.userauth_password(user, pwd).is_ok() {
                    authenticated = true;
                }
            }
        }

        if !authenticated {
            return Err(anyhow!("Authentication failed. Password required."));
        }

        Ok(Self {
            session,
            _tcp: tcp,
        })
    }

    /// Executes a command on the remote host and returns (stdout, stderr, exit_code).
    pub fn run_command(&self, cmd: &str) -> Result<(String, String, i32)> {
        let mut channel = self.session.channel_session()?;
        channel.exec(cmd)?;
        
        let mut s_out = String::new();
        channel.read_to_string(&mut s_out)?;
        
        let mut s_err = String::new();
        channel.stderr().read_to_string(&mut s_err)?;
        
        channel.wait_close()?;
        let exit_status = channel.exit_status()?;
        
        Ok((s_out, s_err, exit_status))
    }

    /// Streams stdout from a remote command to a callback.
    #[allow(dead_code)]
    pub fn stream_stdout<F>(&self, cmd: &str, mut callback: F) -> Result<()> 
    where 
        F: FnMut(String) + Send + 'static,
    {
        let mut channel = self.session.channel_session()?;
        channel.exec(cmd)?;
        
        let stream = channel.stream(0);
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        
        while reader.read_line(&mut line)? > 0 {
             callback(line.trim_end().to_string());
             line.clear();
        }
        
        Ok(())
    }

    /// Checks if the SSH session is still authenticated.
    #[allow(dead_code)]
    pub fn is_connected(&self) -> bool {
        self.session.authenticated()
    }
}
