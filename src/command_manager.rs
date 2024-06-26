use std::{collections::HashMap, sync::Arc, time::Duration};

use async_recursion::async_recursion;
use rand::seq::SliceRandom;
use serde::Deserialize;
use tokio::{sync::Mutex, time::sleep};
use tracing::{info, warn};

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

impl Default for CommandManager {
    fn default() -> Self {
        let manager = CommandManager {
            commands: Arc::default(),
        };

        tokio::spawn(Self::command_updater(manager.commands.clone()));
        info!("Started command data updater task");
        manager
    }
}

impl CommandManager {
    #[async_recursion]
    pub async fn get_sound_uri(&self, sound: String) -> (String, Option<String>) {
        let uri = self
            .commands
            .lock()
            .await
            .get(&sound)
            .and_then(|choices| choices.choose(&mut rand::thread_rng()).cloned());
        match uri {
            Some(uri) => (sound, Some(uri)),
            None => {
                let similars = self
                    .get_commands()
                    .await
                    .into_iter()
                    .filter(|key| key.starts_with(&sound))
                    .collect::<Vec<String>>();
                if similars.len() == 1 {
                    self.get_sound_uri(similars[0].clone()).await
                } else {
                    (sound, None)
                }
            }
        }
    }

    pub async fn get_commands(&self) -> Vec<String> {
        self.commands.lock().await.keys().cloned().collect()
    }

    pub async fn get_human_readable_command_list(&self) -> String {
        let mut v = self.get_commands().await;
        v.sort();
        v.join(", ")
    }

    async fn get_list(
        api: &String,
        collection: &str,
        page: usize,
        per_page: usize,
    ) -> Result<SupabaseCommandsList, reqwest::Error> {
        reqwest::get(format!(
            "{api}/api/collections/{collection}/records?page={page}&perPage={per_page}"
        ))
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

        let api =
            dotenvy::var("POCKETBASE_API").expect("Expected POCKETBASE_API in the environment");
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
            sleep(Duration::from_secs(20)).await;
        }
    }
}
