use std::env;
use std::time::Duration;
use std::{collections::HashMap, sync::Arc};

use async_recursion::async_recursion;
use serde::Deserialize;
use serenity::prelude::Mutex;
use tokio::time::sleep;
use tracing::{info, warn};

pub struct CommandManager {
    commands: Arc<Mutex<HashMap<String, String>>>,
}

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

impl CommandManager {
    pub async fn new() -> Self {
        let manager = CommandManager {
            commands: Arc::default(),
        };

        let command_data = Self::get_command_data()
            .await
            .expect("Could not load command data from database");
        manager.commands.lock().await.extend(command_data);
        info!("Initially updated command data");
        tokio::spawn(Self::command_updater(manager.commands.clone()));
        manager
    }

    pub async fn get_sound_uri(&self, sound: &String) -> Option<String> {
        self.commands.lock().await.get(sound).cloned()
    }

    pub async fn list_commands(&self) -> String {
        self.commands
            .lock()
            .await
            .iter()
            .map(|c| c.0.clone())
            .collect::<Vec<String>>()
            .join(", ")
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
        let mut res = Self::request(api, collection, SupabaseCommandsList::default(), 1).await?;

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
        let mut current = Self::get_list(api, collection, page, 100).await?;

        res.items.append(&mut current.items);

        if res.items.len() < res.totalItems {
            return Self::request(api, collection, res, page + 1).await;
        }

        Ok(res)
    }

    async fn get_command_data() -> Result<HashMap<String, String>, reqwest::Error> {
        let mut res = HashMap::new();

        let api = env::var("POCKETBASE_API").expect("Expected POCKETBASE_API in the environment");
        let source = Self::get_full_list(&api, "sounds").await?;

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

    async fn command_updater(commands: Arc<Mutex<HashMap<String, String>>>) {
        loop {
            sleep(Duration::from_secs(600)).await;
            let command_data = Self::get_command_data().await;

            match command_data {
                Ok(data) => {
                    *commands.lock().await = data;
                    info!("Successfully updated command data");
                }
                Err(err) => {
                    warn!("Failed updating command data: {err}");
                }
            }
        }
    }
}
