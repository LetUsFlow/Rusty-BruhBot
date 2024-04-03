use std::{collections::HashSet, sync::Arc, time::Duration};

use reqwest::Client as HttpClient;
use serenity::{
    client::Context,
    model::id::{GuildId, UserId},
};
use songbird::{input::HttpRequest, tracks::Track, TrackEvent};
use tokio::{sync::Mutex, time::sleep};
use tracing::{error, warn};

use crate::events::*;

pub enum PlayStatus {
    AlreadyPlaying,
    SoundNotFound,
    StartedPlaying,
    VoiceChannelNotFound,
    GuildNotFound,
    JoinError,
}

pub async fn play_sound(
    ctx: &Context,
    guild_id: GuildId,
    author_id: UserId,
    sound_uri: Option<String>,
    connections: Arc<Mutex<HashSet<GuildId>>>,
) -> PlayStatus {
    let sound_uri = match sound_uri {
        Some(sound_uri) => sound_uri,
        None => return PlayStatus::SoundNotFound,
    };

    let channel_id = {
        let guild = match ctx.cache.guild(guild_id) {
            Some(guild) => guild,
            None => {
                warn!("Cannot find guild in cache: {}", guild_id);
                return PlayStatus::GuildNotFound;
            }
        };

        guild
            .voice_states
            .get(&author_id)
            .and_then(|voice_state| voice_state.channel_id)
    };

    let connect_to = match channel_id {
        Some(channel) => channel,
        None => {
            return PlayStatus::VoiceChannelNotFound;
        }
    };

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation")
        .clone();

    // Create audio source
    let source = HttpRequest::new(HttpClient::new(), sound_uri);

    // Create audio handler
    let audio = Track::from(source);

    if !connections.lock().await.insert(guild_id) {
        return PlayStatus::AlreadyPlaying;
    }

    match manager.join(guild_id, connect_to).await {
        Ok(handler_lock) => {
            let mut handler = handler_lock.lock().await;

            // Start playing audio
            let audio_handle = handler.play_only(audio);

            // Add disconnect eventlistener
            handler.add_global_event(
                songbird::Event::Core(songbird::CoreEvent::DriverDisconnect),
                DriverDisconnectNotifier::new(audio_handle.clone(), guild_id, connections),
            );

            // Add track end eventlistener
            audio_handle
                .add_event(
                    songbird::Event::Track(TrackEvent::End),
                    TrackEndNotifier::new(handler_lock.clone()),
                )
                .ok();

            PlayStatus::StartedPlaying
        }
        Err(why) => {
            error!("{}", why);
            sleep(Duration::from_secs(2)).await;
            manager.leave(guild_id).await.ok();
            connections.lock().await.remove(&guild_id);
            PlayStatus::JoinError
        }
    }
}
