use serde::Deserialize;

use crate::{
    configuration::{ConfigurationManager, Server},
    server_runner::ServerRunnerHandle,
    versions::PackagesList,
};

#[derive(Debug, Clone)]
pub struct WebState {
    pub config: ConfigurationManager,
    pub runner_handle: ServerRunnerHandle,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Checked {
    On,
    Off,
}

#[derive(Deserialize)]
pub struct NewServer {
    pub servername: String,
    pub version: String,
    pub eula: Option<Checked>,
}

impl From<Checked> for bool {
    fn from(value: Checked) -> Self {
        match value {
            Checked::On => true,
            Checked::Off => false,
        }
    }
}

impl WebState {
    pub async fn create_new_server(&self, new_server: NewServer) -> Server {
        let eula: bool = new_server.eula.unwrap_or(Checked::Off).into();
        print!("creating new server with eula {}", eula);
        self.runner_handle.stop_all();
        self.config.disable_all().await;
        let packages = PackagesList::new().await;
        let selected = packages
            .get_version(&new_server.version)
            .expect("invalid server version provided");
        let info = selected.get_version_info().await;
        let server = self
            .config
            .create_new_server(new_server.servername, info, eula)
            .await;
        self.config.start_all(self.runner_handle.clone()).await;
        server
    }
    pub async fn disable_server(&self, server_id: usize) {
        self.runner_handle.stop_specific(server_id);
        self.config.set_server_enabled(server_id, false).await;
    }
    pub async fn enable_server(&self, server_id: usize) {
        self.runner_handle.stop_all();
        self.config.disable_all().await;
        self.config.set_server_enabled(server_id, true).await;
        self.config.start_all(self.runner_handle.clone()).await;
    }
}
