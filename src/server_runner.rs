use std::collections::HashMap;

use actix_web::rt::spawn;
use tokio::fs::{self, create_dir_all};
use tokio::io;
use tokio::process::Command;
use tokio::sync::mpsc;

use crate::configuration::Server;

#[derive(Debug, Clone)]
pub struct ServerRunnerHandle {
    cmd_tx: mpsc::UnboundedSender<RunnerCommand>,
}

impl ServerRunnerHandle {
    pub fn start_server(&self, server: Server) {
        self.cmd_tx
            .send(RunnerCommand::StartServer { server })
            .unwrap();
    }
    pub fn start_all(&self, servers: Vec<Server>) {
        self.cmd_tx
            .send(RunnerCommand::StartAll { servers })
            .unwrap();
    }
    pub fn stop_all(&self) {
        self.cmd_tx.send(RunnerCommand::StopAll).unwrap();
    }
}

pub enum RunnerCommand {
    StartServer { server: Server },
    StartAll { servers: Vec<Server> },
    StopServer { id: usize },
    Terminated { id: usize, message: String },
    StopAll,
}

pub struct ServerRunner {
    cmd_reciever: mpsc::UnboundedReceiver<RunnerCommand>,
    active_servers: HashMap<usize, tokio::task::JoinHandle<()>>,
}

impl ServerRunner {
    pub fn new() -> (Self, ServerRunnerHandle) {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let new = Self {
            cmd_reciever: cmd_rx,
            active_servers: HashMap::new(),
        };
        (new, ServerRunnerHandle { cmd_tx })
    }
    async fn start_server(&mut self, server: Server) {
        if let Some(existing) = self.active_servers.get(&server.id) {
            existing.abort();
        }
        create_dir_all(&format!("./servers/{}/game", server.id))
            .await
            .expect("could not create server directory");
        if server.eula {
            fs::write(
                &format!("./servers/{}/game/eula.txt", server.id),
                "eula=true",
            )
            .await
            .expect("failed to write to eula");
        }
        let handle = spawn(async move {
            let mut command = Command::new("java");
            command.current_dir(&format!("./servers/{}/game", server.id));
            command
                .arg("-jar")
                .arg(&format!(
                    "../../../server_versions/{}/{}.jar",
                    &server.mc_version_id, &server.mc_version_id
                ))
                .arg("nogui");
            let status = command.status().await;
            // todo send Terminated
        });
        self.active_servers.insert(server.id, handle);
    }
    pub async fn run(mut self) -> io::Result<()> {
        while let Some(cmd) = self.cmd_reciever.recv().await {
            match cmd {
                RunnerCommand::StartServer { server } => {
                    self.start_server(server).await;
                }

                RunnerCommand::StartAll { servers } => {
                    for server in servers {
                        self.start_server(server).await;
                    }
                }

                RunnerCommand::StopAll => {
                    for (_, server) in &self.active_servers {
                        server.abort();
                    }
                }

                _ => {
                    todo!()
                }
            }
        }

        Ok(())
    }
    pub async fn begin() -> ServerRunnerHandle {
        let (runner, handler) = Self::new();
        spawn(runner.run());
        handler
    }
}
