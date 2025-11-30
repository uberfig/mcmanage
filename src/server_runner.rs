use std::collections::{HashMap, VecDeque};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use actix_web::rt::spawn;
use tokio::fs::{self, create_dir_all};
use tokio::io::{self, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::process::{Child, Command};
use tokio::sync::{RwLock, mpsc, oneshot};
use tokio::time::timeout;

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
    pub fn stop_specific(&self, server: usize) {
        self.cmd_tx
            .send(RunnerCommand::StopServer { id: server })
            .unwrap();
    }
    pub fn issue_command(&self, server: usize, command: String) {
        println!("issuing command {}", &command);
        self.cmd_tx
            .send(RunnerCommand::IssueCommand {
                id: server,
                command,
            })
            .unwrap();
    }
    pub async fn get_output(&self, server: usize) -> Option<String> {
        let (res_tx, res_rx) = oneshot::channel();
        // unwraps used as the server should run until all chat server handles are dropped
        // and then nicely shutdown itself. the server should always be shutting down after
        // us here, never before
        self.cmd_tx
            .send(RunnerCommand::GetOutput {
                id: server,
                response_handle: res_tx,
            })
            .unwrap();
        res_rx.await.unwrap()
    }
}

pub enum RunnerCommand {
    StartServer {
        server: Server,
    },
    StartAll {
        servers: Vec<Server>,
    },
    StopServer {
        id: usize,
    },
    StopAll,
    IssueCommand {
        id: usize,
        command: String,
    },
    GetOutput {
        id: usize,
        response_handle: oneshot::Sender<Option<String>>,
    },
}

pub struct ServerRunner {
    cmd_reciever: mpsc::UnboundedReceiver<RunnerCommand>,
    active_servers: HashMap<usize, ServerProcess>,
}

pub struct ServerProcess {
    child_process: tokio::process::Child,
    text_log: Arc<RwLock<VecDeque<String>>>,
    stdin: tokio::io::BufWriter<tokio::process::ChildStdin>,
}

async fn append_to_log(text_log: Arc<RwLock<VecDeque<String>>>, line: String) {
    println!("writing to log");
    let mut writing = text_log.write().await;
    writing.push_back(line);
    if writing.len() > 200 {
        let _ = writing.pop_front();
    }
    println!("wrote to log");
}

async fn append_many_to_log(text_log: Arc<RwLock<VecDeque<String>>>, lines: Vec<String>) {
    let mut writing = text_log.write().await;
    for line in lines{
        writing.push_back(line);
    if writing.len() > 200 {
        let _ = writing.pop_front();
    }
    }
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
        if let Some(mut existing) = self.active_servers.remove(&server.id) {
            existing
                .child_process
                .kill()
                .await
                .expect("failed to kill server");
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
        let mut command = Command::new("java");
        command.current_dir(&format!("./servers/{}/game", server.id));
        command
            .arg("-jar")
            .arg(&format!(
                "../../../server_versions/{}/{}.jar",
                &server.mc_version_id, &server.mc_version_id
            ))
            .arg("nogui")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped());
        let mut status = command
            .kill_on_drop(true)
            .spawn()
            .expect("could not spawn server");
        let text_log = Arc::new(RwLock::new(VecDeque::new()));        
        
        let stdout = BufReader::new(status.stdout.take().unwrap());
        let stdin = BufWriter::new(status.stdin.take().unwrap());

        self.active_servers.insert(
            server.id,
            ServerProcess {
                child_process: status,
                text_log: text_log.clone(),
                stdin: stdin,
            },
        );
        spawn(async move {
            let mut reader = stdout.lines();
            while let Ok(Some(line)) = reader.next_line().await {
                append_to_log(text_log.clone(), line).await;
            }
            println!("reader dead");
        });
    }
    pub async fn run(mut self) -> io::Result<()> {
        'meow: while let Some(cmd) = self.cmd_reciever.recv().await {
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
                    for (_, server) in &mut self.active_servers {
                        server
                            .child_process
                            .kill()
                            .await
                            .expect("failed to kill server");
                    }
                }
                RunnerCommand::StopServer { id } => {
                    if let Some(server) = &mut self.active_servers.remove(&id) {
                        server
                            .child_process
                            .kill()
                            .await
                            .expect("failed to kill server");
                    }
                }
                RunnerCommand::IssueCommand { id, command } => {
                    println!("command recieved: {}", &command);
                    if let Some(server) = self.active_servers.get_mut(&id) {
                        server
                            .stdin
                            .write_all(format!("{}\n", command).as_bytes())
                            .await
                            .expect("could not issue command");
                        server.stdin.flush().await.expect("failed to flush stdin");
                        append_to_log(server.text_log.clone(), command).await;
                    }
                }
                RunnerCommand::GetOutput {
                    id,
                    response_handle,
                } => {
                    if let Some(server) = self.active_servers.get_mut(&id) {
                        let data = server.text_log.read().await;
                        let data: Vec<String> = data.clone().into();
                        let _ = response_handle.send(Some(data.join("\n")));
                        continue 'meow;
                    }
                    let _ = response_handle.send(None);
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
