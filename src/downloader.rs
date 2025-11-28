use std::{fs::create_dir_all, path::Path};

use crate::versions::{DownloadInfo, VersionInfo};

pub struct Downloader;

impl Downloader {
    pub async fn ensure_available(version: VersionInfo) {
        let path = format!("./server_versions/{}", &version.id);
        if Path::new(&path).is_dir() {
            return;
        }
        create_dir_all(&path).expect("could not create server directory");
        version
            .downloads
            .server
            .download(&format!("{}/{}.jar", path, &version.id))
            .await
            .expect("failed to download new server");
    }
}
