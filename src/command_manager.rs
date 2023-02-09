use std::env;
use std::time::Duration;
use std::collections::HashMap;

use async_recursion::async_recursion;
use once_cell::sync::Lazy;
use serde::Deserialize;
use serenity::prelude::Mutex;
use tokio::time::sleep;
use tracing::{info, warn};

static COMMANDS: Lazy<Mutex<HashMap<String, String>>> = Lazy::new(|| Mutex::new(HashMap::new()));

#[allow(non_snake_case)]
#[derive(Deserialize, Debug, Default)]
struct SupabaseCommandsList {
    page: usize,
    perPage: usize,
    totalItems: usize,
    items: Vec<SupabaseCommandItem>,
}

#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
struct SupabaseCommandItem {
    collectionId: String,
    command: String,
    id: String,
    audio: String,
}

pub async fn setup() {
    let command_data = get_command_data()
        .await
        .unwrap_or_else(|_| panic!("Could not load command data from database"));
    COMMANDS.lock().await.extend(command_data);
    info!("Initially updated command data");
    tokio::spawn(command_updater());
}

pub async fn get_sound_uri(sound: &String) -> Option<String> {
    let commands = COMMANDS.lock().await;

    commands.get(sound).cloned()
}

pub async fn list_commands() -> String {
    COMMANDS.lock().await.iter().map(|c| c.0.clone()).collect::<Vec<String>>().join(", ")
}

async fn get_list(
    api: &String,
    collection: &str,
    page: usize,
    per_page: usize,
) -> Result<SupabaseCommandsList, reqwest::Error> {
    reqwest::get(
        api.to_owned()
            + &format!("/api/collections/{collection}/records?page={page}&perPage={per_page}"),
    )
    .await?
    .json::<SupabaseCommandsList>()
    .await
}

async fn get_full_list(
    api: &String,
    collection: &str,
) -> Result<SupabaseCommandsList, reqwest::Error> {
    let mut res = request(api, collection, SupabaseCommandsList::default(), 1).await?;

    res.page = 1;
    res.totalItems = res.items.len();
    res.perPage = res.items.len();

    Ok(res)
}

#[async_recursion]
async fn request(
    api: &String,
    collection: &str,
    mut res: SupabaseCommandsList,
    page: usize,
) -> Result<SupabaseCommandsList, reqwest::Error> {
    let mut current = get_list(api, collection, page, 100).await?;

    res.items.append(&mut current.items);

    if res.items.len() < res.totalItems {
        return request(api, collection, res, page + 1).await;
    }

    Ok(res)
}

async fn get_command_data() -> Result<HashMap<String, String>, reqwest::Error> {
    let mut res = HashMap::new();

    let api = env::var("POCKETBASE_API").unwrap();
    let source = get_full_list(&api, "sounds").await?;

    for item in source.items {
        res.insert(
            item.command,
            format!(
                "{api}/api/files/{}/{}/{}",
                item.collectionId, item.id, item.audio
            ),
        );
    }
    Ok(res)
}

async fn command_updater() {
    loop {
        sleep(Duration::from_secs(60)).await;
        let command_data = get_command_data().await;

        match command_data {
            Ok(data) => {
                let mut commands = COMMANDS.lock().await;

                commands.clear();
                commands.extend(data);
                info!("Successfully updated command data");
            }
            Err(err) => {
                warn!("Failed updating command data: {err}");
            }
        }
    }
}
