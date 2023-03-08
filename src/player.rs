use std::collections::HashSet;

use std::sync::Arc;

use serenity::model::prelude::{GuildId, Member};
use serenity::prelude::*;
use songbird::{create_player, TrackEvent};
use tracing::warn;

use crate::events::*;

pub async fn play_sound(
    ctx: &Context,
    handler: &Handler,
    author: Member,
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

    let guild = match ctx.cache.guild(author.guild_id) {
        Some(guild) => guild,
        None => {
            warn!("Cannot find guild in cache: {}", author.guild_id);
            return false;
        }
    };
    let guild_id = author.guild_id;

    let channel_id = guild
        .voice_states
        .get(&author.user.id)
        .and_then(|voice_state| voice_state.channel_id);

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

    if !connections.lock().await.insert(guild_id) {
        return false;
    }

    // Create audio source
    let source = match songbird::ffmpeg(sound_uri).await {
        Ok(source) => source,
        Err(err) => {
            warn!("Err starting source: {err:?}");
            connections.lock().await.remove(&guild_id);
            return false;
        }
    };

    // Create audio handler
    let (audio, audio_handle) = create_player(source);

    let (handler_lock, _join_result) = manager.join(guild_id, connect_to).await;
    let mut handler = handler_lock.lock().await;

    // Add disconnect eventlistener
    handler.add_global_event(
        songbird::Event::Core(songbird::CoreEvent::DriverDisconnect),
        DriverDisconnectNotifier::new(audio_handle.clone(), guild_id, connections),
    );

    // Start playing audio
    handler.play_only(audio);

    // Add track end eventlistener
    audio_handle
        .add_event(
            songbird::Event::Track(TrackEvent::End),
            TrackEndNotifier::new(handler_lock.clone()),
        )
        .ok();

    true
}
