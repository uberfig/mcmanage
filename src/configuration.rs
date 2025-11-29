use std::sync::Arc;

use argon2::{Argon2, PasswordHash, PasswordVerifier};
use serde::{Deserialize, Serialize};

use tokio::fs;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

use crate::downloader::Downloader;
use crate::server_runner::ServerRunnerHandle;
use crate::versions::VersionInfo;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub username: String,
    pub hashed_password: String,
    pub is_admin: bool,
}

// nice support for fabric planned
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum ServerType {
    Vanilla,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Server {
    pub id: usize,
    pub name: String,
    pub domain: Option<String>,
    pub port: u16,
    pub server_type: ServerType,
    pub mc_version_id: String,
    pub enabled: bool,
    /// #By changing the setting below to TRUE you are indicating your agreement to our EULA (https://aka.ms/MinecraftEULA).
    pub eula: bool,
}

impl Server {
    pub fn new(name: String, version_id: String, eula: bool) -> Self {
        Self {
            id: 0,
            name,
            domain: None,
            port: 25565,
            server_type: ServerType::Vanilla,
            mc_version_id: version_id,
            enabled: true,
            eula,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Configuration {
    pub next_server_id: usize,
    pub max_concurrent_servers: usize,
    pub max_total_ram_mb: usize,
    pub users: Vec<User>,
    pub servers: Vec<Server>,
}

const FILE_PATH: &str = "mcmanager.toml";

impl Configuration {
    async fn load_config() -> Configuration {
        match fs::read_to_string(FILE_PATH).await {
            Ok(contents) => toml::from_str(contents.as_str()).expect("invalid config on disk"),
            Err(_) => {
                let new = Self::new();
                let mut file = fs::File::create(FILE_PATH)
                    .await
                    .expect("failed to init file");
                file.write_all(toml::to_string_pretty(&new).unwrap().as_bytes())
                    .await
                    .expect("failed to write to new file");
                file.flush().await.expect("failed to flush");
                new
            }
        }
    }
    fn new() -> Self {
        Self {
            next_server_id: 0,
            max_concurrent_servers: 1,
            max_total_ram_mb: 3000,
            users: vec![],
            servers: vec![],
        }
    }
    async fn write(&self) -> std::io::Result<()> {
        let mut file = File::create(FILE_PATH).await?;
        file.write_all(toml::to_string_pretty(&self).unwrap().as_bytes())
            .await?;
        file.flush().await?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ConfigurationManager {
    manager: Arc<Mutex<Configuration>>,
}

impl ConfigurationManager {
    pub async fn new() -> Self {
        Self {
            manager: Arc::new(Mutex::new(Configuration::load_config().await)),
        }
    }
    pub async fn reload(&self) {
        let mut lock = self.manager.lock().await;
        *lock = Configuration::load_config().await;
    }
    pub async fn server_count(&self) -> usize {
        let lock = self.manager.lock().await;
        lock.servers.len()
    }
    pub async fn add_user(&self, new_user: User) {
        let mut lock = self.manager.lock().await;
        lock.users.push(new_user);
        lock.write().await.expect("failed to write updated config");
    }
    pub async fn modify_user(&self, updated_user: User) {
        let mut lock = self.manager.lock().await;
        let user = lock
            .users
            .iter_mut()
            .find(|x| x.username == updated_user.username)
            .expect("updating nonexistent user");
        *user = updated_user;
        lock.write().await.expect("failed to write updated config");
    }
    pub async fn get_user(&self, username: String) -> Option<User> {
        let lock = self.manager.lock().await;
        lock.users.iter().find(|x| x.username == username).cloned()
    }
    pub async fn has_users(&self) -> bool {
        let lock = self.manager.lock().await;
        !lock.users.is_empty()
    }
    pub async fn validate_password(&self, username: String, password: String) -> Result<(), ()> {
        if let Some(user) = self.get_user(username).await {
            let parsed_hash =
                PasswordHash::new(&user.hashed_password).expect("invalid password hash stored");
            if let Ok(_) = Argon2::default().verify_password(password.as_bytes(), &parsed_hash) {
                return Ok(());
            }
        }
        Err(())
    }
    /// add a new server and return it, ignores whatever id or port is provided
    pub async fn add_server(&self, mut new_server: Server) -> Server {
        let mut lock = self.manager.lock().await;
        let new_id = lock.next_server_id;
        new_server.id = new_id;
        new_server.port = 25565 + new_id as u16;
        lock.next_server_id += 1;
        lock.servers.push(new_server.clone());
        lock.write().await.expect("failed to write updated config");
        new_server
    }
    pub async fn modify_server(&self, updated_server: Server) {
        let mut lock = self.manager.lock().await;
        let server = lock
            .servers
            .iter_mut()
            .find(|x| x.id == updated_server.id)
            .expect("updating nonexistent server");
        *server = updated_server;
        lock.write().await.expect("failed to write updated config");
    }
    pub async fn get_servers(&self) -> Vec<Server> {
        let lock = self.manager.lock().await;
        lock.servers.clone()
    }
    pub async fn get_server(&self, server_id: usize) -> Option<Server> {
        let lock = self.manager.lock().await;
        lock.servers.iter().find(|x| x.id == server_id).cloned()
    }
    pub async fn set_server_enabled(&self, server: usize, enabled: bool) {
        let mut lock = self.manager.lock().await;
        let server = lock.servers.iter_mut().find(|x| x.id == server);
        if let Some(server) = server {
            server.enabled = enabled;
        }
        lock.write().await.expect("failed to write updated config");
    }
    pub async fn create_new_server(
        &self,
        name: String,
        version: VersionInfo,
        eula: bool,
    ) -> Server {
        let server = Server::new(name, version.id.clone(), eula);
        let server = self.add_server(server).await;
        Downloader::ensure_available(version).await;
        return server;
    }
    pub async fn start_all(&self, handle: ServerRunnerHandle) {
        let lock = self.manager.lock().await;
        handle.start_all(
            lock.servers
                .clone()
                .into_iter()
                .filter(|x| x.enabled)
                .collect(),
        );
    }
    pub async fn disable_all(&self) {
        let mut lock = self.manager.lock().await;
        for server in &mut lock.servers {
            server.enabled = false;
        }
        lock.write().await.expect("failed to write config to disk")
    }
}
