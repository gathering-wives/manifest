use std::{path::PathBuf, sync::Arc};

use serde::{Deserialize, Serialize};
use tokio::{io::AsyncWriteExt, task::JoinSet};
use tracing::{Level, error, info};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

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
pub async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    let content = tokio::fs::read_to_string("versions.json").await?;
    let manifest = serde_json::from_str::<Vec<ManifestVersion>>(&content)?;

    let client = reqwest::Client::new();
    let mut tasks = JoinSet::new();

    for version in manifest {
        let version = Arc::new(version);

        let launcher_client = client.clone();
        let launcher_version = version.clone();
        tasks.spawn(async move { download_launcher(launcher_client, launcher_version).await });

        let game_client = client.clone();
        let game_version = version.clone();
        tasks.spawn(async move { download_game(game_client, game_version).await });
    }

    let result = tasks.join_all().await;
    for err in result.into_iter().filter_map(|result| result.err()) {
        error!("Error occurred in one of the tasks: {:?}", err);
    }

    Ok(())
}

pub async fn download_launcher(
    client: reqwest::Client,
    version: Arc<ManifestVersion>,
) -> Result<()> {
    let url = format!(
        "https://{}-{}-gamestarter.kurogame.com/launcher/launcher/{}_{}/{}/index.json",
        version.branch, version.cdn[0], version.unk_id, version.hash, version.game_id
    );
    let path = format!("launcher/{}/index.json", version.name);

    download_json_file(client, &url, &path).await?;

    Ok(())
}

pub async fn download_game(client: reqwest::Client, version: Arc<ManifestVersion>) -> Result<()> {
    let url = format!(
        "https://{}-{}-gamestarter.kurogame.com/launcher/game/{}/{}_{}/index.json",
        version.branch, version.cdn[0], version.game_id, version.unk_id, version.hash
    );
    let path = format!("game/{}/index.json", version.name);

    let json = download_json_file(client.clone(), &url, &path).await?;
    let cdn = &json["default"]["cdnList"][0]["url"].as_str();
    let resources = &json["default"]["resources"].as_str();

    if let Some(resources) = resources {
        if let Some(cdn) = cdn {
            let url = format!("{}/{}", cdn, resources);
            let path = format!("game/{}/resources.json", version.name);

            download_json_file(client, &url, &path).await?;
        }
    }

    Ok(())
}

pub async fn download_json_file(
    client: reqwest::Client,
    url: &String,
    path: &String,
) -> Result<serde_json::Value> {
    let path = PathBuf::from(path);
    tokio::fs::create_dir_all(path.parent().unwrap()).await?;

    info!(r#"Downloading "{}""#, url);
    let response = client.get(url).send().await?;

    match response.error_for_status() {
        Ok(response) => {
            let mut text = response.text().await?;
            let json: serde_json::Value = serde_json::from_str(&text)?;
            text = serde_json::to_string_pretty(&json)?;

            let mut file = tokio::fs::File::create(path).await?;
            file.write_all(text.as_bytes()).await?;

            Ok(json)
        }
        Err(error) => Err(error.into()),
    }
}
