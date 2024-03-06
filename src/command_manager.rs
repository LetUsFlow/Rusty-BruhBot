use std::{collections::HashMap, env, sync::Arc, time::Duration};

use async_recursion::async_recursion;
use rand::seq::SliceRandom;
use serde::Deserialize;
use serenity::prelude::Mutex;
use tokio::time::sleep;
use tracing::{info, warn};

#[derive(Default)]
pub struct CommandManager {
    commands: Arc<Mutex<HashMap<String, Vec<String>>>>,
}

#[allow(non_snake_case)]
#[derive(Deserialize, Debug, Default)]
struct SupabaseCommandsList {
    perPage: usize,
    items: Vec<SupabaseCommandItem>,
}

#[allow(non_snake_case)]
#[derive(Deserialize, Debug, Clone)]
struct SupabaseCommandItem {
    collectionId: String,
    command: String,
    id: String,
    audio: String,
}

impl CommandManager {
    pub async fn new() -> Self {
        let manager = CommandManager::default();

        let command_data = Self::get_command_data()
            .await
            .expect("Could not load command data from database");
        manager.commands.lock().await.extend(command_data);
        info!("Initially updated command data");
        tokio::spawn(Self::command_updater(manager.commands.clone()));
        manager
    }

    pub async fn get_sound_uri(&self, sound: &str) -> Option<String> {
        self.commands
            .lock()
            .await
            .get(sound)
            .and_then(|choices| choices.choose(&mut rand::thread_rng()).cloned())
    }

    pub async fn list_commands(&self) -> String {
        let mut v = self
            .commands
            .lock()
            .await
            .keys()
            .cloned()
            .collect::<Vec<String>>();
        v.sort();
        v.join(", ")
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
        let mut res = SupabaseCommandsList::default();
        Self::request(api, collection, &mut res, 1).await?;
        Ok(res)
    }

    #[async_recursion]
    async fn request(
        api: &String,
        collection: &str,
        res: &mut SupabaseCommandsList,
        page: usize,
    ) -> Result<(), reqwest::Error> {
        let mut current = Self::get_list(api, collection, page, 100).await?;

        res.items.append(&mut current.items);

        if res.items.len() == current.perPage {
            return Self::request(api, collection, res, page + 1).await;
        }

        Ok(())
    }

    async fn get_command_data() -> Result<HashMap<String, Vec<String>>, reqwest::Error> {
        let mut res: HashMap<String, Vec<String>> = HashMap::new();

        let api = env::var("POCKETBASE_API").expect("Expected POCKETBASE_API in the environment");
        let source = Self::get_full_list(&api, "sounds").await?;

        for item in source.items {
            match res.get_mut(&item.command) {
                Some(urls) => {
                    urls.push(Self::format_api_url(&item, &api));
                }
                None => {
                    res.insert(
                        item.clone().command,
                        vec![Self::format_api_url(&item, &api)],
                    );
                }
            }
        }

        Ok(res)
    }

    fn format_api_url(item: &SupabaseCommandItem, api: &String) -> String {
        format!(
            "{api}/api/files/{}/{}/{}",
            item.collectionId, item.id, item.audio
        )
    }

    async fn command_updater(commands: Arc<Mutex<HashMap<String, Vec<String>>>>) {
        loop {
            sleep(Duration::from_secs(20)).await;
            let command_data: Result<HashMap<String, Vec<String>>, reqwest::Error> =
                Self::get_command_data().await;

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
