use std::{path::PathBuf, sync::Arc};

use futures::future::join_all;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;

#[derive(Deserialize, Serialize)]
pub struct ManifestVersion {
    name: String,
    branch: String,
    cdn: Vec<String>,
    game_id: String,
    unk_id: String,
    hash: String,
}

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let content = tokio::fs::read_to_string("versions.json").await?;
    let manifest = serde_json::from_str::<Vec<ManifestVersion>>(&content)?;

    _ = tokio::fs::remove_dir_all("game").await;
    _ = tokio::fs::remove_dir_all("launcher").await;

    let client = reqwest::Client::new();
    let mut tasks = vec![];

    for version in manifest {
        let version = Arc::new(version);

        let launcher_client = client.clone();
        let launcher_version = version.clone();
        tasks.push(tokio::spawn(async move {
            download_launcher(launcher_client, launcher_version).await
        }));

        let game_client = client.clone();
        let game_version = version.clone();
        tasks.push(tokio::spawn(async move {
            download_game(game_client, game_version).await
        }));
    }

    join_all(tasks).await;

    Ok(())
}

pub async fn download_launcher(
    client: reqwest::Client,
    version: Arc<ManifestVersion>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let url = format!(
        "https://{}-{}-gamestarter.kurogame.com/launcher/launcher/{}_{}/{}/index.json",
        version.branch, version.cdn[0], version.unk_id, version.hash, version.game_id
    );
    let path = format!("launcher/{}/index.json", version.name);

    download_file(client, &url, &path).await
}

pub async fn download_game(
    client: reqwest::Client,
    version: Arc<ManifestVersion>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let url = format!(
        "https://{}-{}-gamestarter.kurogame.com/launcher/game/{}/{}_{}/index.json",
        version.branch, version.cdn[0], version.game_id, version.unk_id, version.hash
    );
    let path = format!("game/{}/index.json", version.name);

    download_file(client, &url, &path).await
}

pub async fn download_file(
    client: reqwest::Client,
    url: &String,
    path: &String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path = PathBuf::from(path);
    tokio::fs::create_dir_all(path.parent().unwrap()).await?;

    let response = client.get(url).send().await?;

    let mut text = response.text().await?;
    let json: serde_json::Value = serde_json::from_str(&text)?;
    text = serde_json::to_string_pretty(&json)?;

    let mut file = tokio::fs::File::create(path).await?;
    file.write_all(text.as_bytes()).await?;

    Ok(())
}
