use std::collections::HashSet;
use std::sync::Arc;

use parking_lot::Mutex;
use serenity::async_trait;
use serenity::model::application::command::Command;
use serenity::model::application::interaction::{Interaction, InteractionResponseType};
use serenity::model::gateway::Ready;
use serenity::model::prelude::command::CommandOptionType;
use serenity::model::prelude::interaction::application_command::CommandDataOption;
use serenity::model::prelude::{GuildId, Message};
use serenity::prelude::{Context, EventHandler};
use songbird::tracks::TrackHandle;
use songbird::{Call, Event, EventContext};
use tracing::{info, warn};

use crate::command_manager::CommandManager;
use crate::player::play_sound;

pub struct Handler {
    pub connections: Arc<Mutex<HashSet<GuildId>>>,
    pub commands: CommandManager,
}

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            let content = match command.data.name.as_str() {
                "bruh" => {
                    let cdo = command.data.options.get(0);
                    let author = command.member.clone();

                    match (cdo, author) {
                        (
                            Some(CommandDataOption {
                                value: Some(ref sound),
                                ..
                            }),
                            Some(author),
                        ) => {
                            let play_status = play_sound(
                                &ctx,
                                self,
                                author,
                                sound.as_str().unwrap_or("").into(),
                                self.connections.clone(),
                            )
                            .await;

                            match play_status {
                                true => ":sunglasses:".into(),
                                false => ":skull:".into(),
                            }
                        }
                        (_, None) => {
                            "You shouldn't be here, I shouldn't be here, we both know it...".into()
                        }
                        _ => self.commands.list_commands().await,
                    }
                }
                _ => "i donbt know dis command uwu :(".into(),
            };

            if let Err(why) = command
                .create_interaction_response(&ctx.http, |response| {
                    response
                        .kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|message| message.content(content))
                })
                .await
            {
                warn!("Cannot respond to slash command: {why}");
            }
        }
    }

    async fn message(&self, ctx: Context, msg: Message) {
        let content = msg.content.trim().to_lowercase();
        if content == "brelp" || content == "bruhelp" {
            if let Err(why) = msg
                .channel_id
                .say(&ctx.http, self.commands.list_commands().await)
                .await
            {
                warn!("Error sending message: {why:?}");
            }
        } else if let Some(guild_id) = msg.guild_id {
            if let Some(author) = ctx.cache.member(guild_id, msg.author.id) {
                play_sound(&ctx, self, author, content, self.connections.clone()).await;
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);

        Command::set_global_application_commands(&ctx.http, |commands| {
            commands.create_application_command(|command| {
                // Enable autocomplete for all sounds
                command
                    .name("bruh")
                    .description("Play a sound")
                    .create_option(|option| {
                        option
                            .name("sound")
                            .description("Name of sound")
                            .kind(CommandOptionType::String)
                    })
            })
        })
        .await
        .ok();
    }
}

pub struct TrackEndNotifier {
    handler_lock: Arc<serenity::prelude::Mutex<Call>>,
}

impl TrackEndNotifier {
    pub fn new(handler_lock: Arc<serenity::prelude::Mutex<Call>>) -> Self {
        Self { handler_lock }
    }
}

#[async_trait]
impl songbird::events::EventHandler for TrackEndNotifier {
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        let mut handler = self.handler_lock.lock().await;
        handler.leave().await.ok();
        None
    }
}

pub struct DriverDisconnectNotifier {
    audio_handle: TrackHandle,
    guild_id: GuildId,
    connections: Arc<Mutex<HashSet<GuildId>>>,
}

impl DriverDisconnectNotifier {
    pub fn new(
        audio_handle: TrackHandle,
        guild_id: GuildId,
        connections: Arc<Mutex<HashSet<GuildId>>>,
    ) -> Self {
        Self {
            audio_handle,
            guild_id,
            connections,
        }
    }
}

#[async_trait]
impl songbird::events::EventHandler for DriverDisconnectNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::DriverDisconnect(_data) = ctx {
            self.audio_handle.stop().ok();
            self.connections.lock().remove(&self.guild_id);
        }
        None
    }
}
