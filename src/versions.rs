use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::{
    fs::{File, remove_file},
    io::{self, AsyncReadExt, AsyncWriteExt},
    join,
};
use tokio_stream::StreamExt;
pub const PACKAGES_ENDPOINT: &str = "https://launchermeta.mojang.com/mc/game/version_manifest.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Latest {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseType {
    Snapshot,
    Release,
    OldBeta,
    OldAlpha,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McVersion {
    pub id: String,
    #[serde(rename = "type")]
    pub release_type: ReleaseType,
    pub url: String,
    pub time: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
}

impl McVersion {
    pub async fn get_version_info(&self) -> VersionInfo {
        let res = reqwest::get(&self.url)
            .await
            .expect("failed to communicate with mojang")
            .json::<VersionInfo>()
            .await
            .expect("invalid version info format from mojang");
        res
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackagesList {
    pub latest: Latest,
    pub versions: Vec<McVersion>,
}

impl PackagesList {
    pub fn get_version(&self, version_id: &str) -> McVersion {
        let version = self
            .versions
            .iter()
            .find(|&x| x.id.eq(version_id))
            .expect("unable to find version");
        version.clone()
    }
    pub fn get_latest_release(&self) -> McVersion {
        self.get_version(&self.latest.release)
    }
    pub async fn new() -> Self {
        let res = reqwest::get(PACKAGES_ENDPOINT)
            .await
            .expect("failed to communicate with mojang")
            .json::<PackagesList>()
            .await
            .expect("invalid packages list format from mojang");
        res
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadInfo {
    pub sha1: String,
    pub size: usize,
    pub url: String,
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
impl DownloadInfo {
    pub async fn download(&self, path: &str) -> Result<()> {
        let mut file = File::create(path).await?;
        println!("Downloading {}...", &self.url);

        let mut stream = reqwest::get(&self.url).await?.bytes_stream();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            file.write_all(&chunk).await?;
        }

        file.flush().await?;

        println!("Downloaded {}", &self.url);
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionDownloads {
    pub client: DownloadInfo,
    pub client_mappings: DownloadInfo,
    pub server: DownloadInfo,
    pub server_mappings: DownloadInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub downloads: VersionDownloads,
    pub id: String,
}
