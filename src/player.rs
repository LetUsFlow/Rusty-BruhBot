use std::{collections::HashSet, sync::Arc};

use parking_lot::Mutex;
use reqwest::Client as HttpClient;
use serenity::{all::UserId, model::prelude::GuildId, prelude::Context};
use songbird::{input::HttpRequest, tracks::Track, TrackEvent};
use tracing::warn;

use crate::events::*;

pub async fn play_sound(
    ctx: &Context,
    handler: &DiscordHandler,
    guild_id: GuildId,
    author_id: UserId,
    sound: String,
    connections: Arc<Mutex<HashSet<GuildId>>>,
) -> bool {
    let sound_uri = handler
        .commands
        .get_sound_uri(&sound.trim().to_lowercase())
        .await;
    let sound_uri = match sound_uri {
        Some(sound_uri) => sound_uri,
        None => return false,
    };

    let channel_id = {
        let guild = match ctx.cache.guild(guild_id) {
            Some(guild) => guild,
            None => {
                warn!("Cannot find guild in cache: {}", guild_id);
                return false;
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
            warn!("Cannot find channel: {channel_id:?}");
            return false;
        }
    };

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation")
        .clone();

    if !connections.lock().insert(guild_id) {
        return false;
    }

    // Create audio source
    let source = HttpRequest::new(HttpClient::new(), sound_uri);

    // Create audio handler
    let audio = Track::from(source);

    if let Ok(handler_lock) = manager.join(guild_id, connect_to).await {
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

        true
    } else {
        connections.lock().remove(&guild_id);
        false
    }
}
