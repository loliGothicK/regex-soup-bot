/*
 * ISC License
 *
 * Copyright (c) 2021 Mitama Lab
 *
 * Permission to use, copy, modify, and/or distribute this software for any
 * purpose with or without fee is hereby granted, provided that the above
 * copyright notice and this permission notice appear in all copies.
 *
 * THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
 * WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
 * MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
 * ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
 * WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
 * ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
 * OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
 *
 */

#![feature(format_args_capture)]
#![feature(never_type)]
#![feature(in_band_lifetimes)]
#![feature(result_flattening)]
#![feature(bool_to_option)]

use anyhow::{anyhow, Context};
use counted_array::counted_array;

use itertools::Either;
use once_cell::sync::Lazy;
use regexsoup::{
    bot::{Container, InspectionAcceptance, Msg, Quiz, Tsx},
    command_ext::CommandExt,
    commands,
    concepts::SameAs,
    notification::{Notification, SlashCommand, To},
    parser::{ComponentParser, CustomId},
    regex::Alphabet,
};
use serenity::{
    async_trait,
    builder::CreateEmbed,
    client::{Client, EventHandler},
    model::{
        gateway::Ready,
        id::{ChannelId, UserId},
        interactions::{application_command::ApplicationCommand, Interaction},
    },
    utils::Colour,
};
use std::{
    collections::{HashMap, HashSet},
    convert::TryInto,
    fmt::{Debug, Display},
    num::NonZeroU8,
    sync::{Arc, Mutex},
};
use strum::IntoEnumIterator;
use tokio::sync::mpsc::channel;

counted_array!(
    const COMMANDS: [&'static str; _] = [
        "start",
        "query",
        "guess",
        "summary",
        "join",
        "give-up",
        "help",
    ]
);

pub static CONTAINER: Lazy<Arc<Mutex<Container>>> = Lazy::new(|| {
    let container = Container::default();
    Arc::new(Mutex::new(container))
});

#[async_trait]
trait Containerized {
    async fn command<F, R>(&self, channel: ChannelId, cmd: F) -> anyhow::Result<R>
    where
        F: FnOnce(&mut Quiz) -> R + Send + Sync + 'async_trait;
    async fn checked_command<F, R>(
        &self,
        channel: ChannelId,
        user: UserId,
        cmd: F,
    ) -> anyhow::Result<R>
    where
        F: FnOnce(&mut Quiz) -> R + Send + Sync + 'async_trait;
    async fn fresh(&self, channel: ChannelId, difficulty: NonZeroU8)
        -> anyhow::Result<CreateEmbed>;
    async fn delete(&self, channel: ChannelId);
}

#[async_trait]
impl Containerized for Lazy<Arc<Mutex<Container>>> {
    async fn command<F, R>(&self, channel: ChannelId, cmd: F) -> anyhow::Result<R>
    where
        F: FnOnce(&mut Quiz) -> R + Send + Sync + 'async_trait,
    {
        loop {
            if let Ok(mut lock) = self.try_lock() {
                return lock
                    .channel_map
                    .get_mut(&channel)
                    .ok_or_else(|| anyhow!("ゲームが開始していません"))?
                    .as_mut()
                    .map(cmd)
                    .ok_or_else(|| anyhow!("not started"));
            }
        }
    }

    async fn checked_command<F, R>(
        &self,
        channel: ChannelId,
        user: UserId,
        cmd: F,
    ) -> anyhow::Result<R>
    where
        F: FnOnce(&mut Quiz) -> R + Send + Sync + 'async_trait,
    {
        loop {
            if let Ok(mut lock) = self.try_lock() {
                return lock
                    .channel_map
                    .get_mut(&channel)
                    .ok_or_else(|| anyhow!("ゲームが開始していません"))?
                    .as_mut()
                    .ok_or_else(|| anyhow!("ゲームが開始していません"))
                    .and_then(|quiz: &mut Quiz| {
                        quiz.is_participant(&user).then_some(quiz).ok_or_else(|| {
                            anyhow!("まずは`start`コマンドでゲームを開始してください")
                        })
                    })
                    .map(cmd);
            }
        }
    }

    async fn fresh(
        &self,
        channel: ChannelId,
        difficulty: NonZeroU8,
    ) -> anyhow::Result<CreateEmbed> {
        let quiz = commands::generate_regex(difficulty).await?;

        loop {
            if let Ok(mut lock) = self.try_lock() {
                let domain = Alphabet::iter()
                    .take(difficulty.get().into())
                    .collect::<HashSet<_>>();

                let mut embed = CreateEmbed::default();
                embed
                    .colour(Colour::BLITZ_BLUE)
                    .title("Starts a fresh REGEX-SOUP")
                    .field("domain", format!("Σ = {domain:?}"), false);

                return Ok(lock
                    .channel_map
                    .insert(channel, Some(quiz))
                    .map(|_| embed.clone())
                    .unwrap_or_else(move || {
                        embed.field("ATTENTION:", "An old REGEX-SOUP is expired.", false);
                        embed
                    }));
            }
        }
    }

    async fn delete(&self, channel: ChannelId) {
        loop {
            if let Ok(mut lock) = self.try_lock() {
                lock.channel_map
                    .entry(channel)
                    .and_modify(|quiz| *quiz = None);
                break;
            }
        }
    }
}

#[async_trait]
pub trait Logger<T: Debug> {
    async fn logging(self) -> anyhow::Result<(), !>
    where
        Self: SameAs<anyhow::Result<T>>;

    async fn logging_with<F: Send + Sync + 'static, Log: Display>(
        self,
        f: F,
    ) -> anyhow::Result<(), !>
    where
        Self: SameAs<anyhow::Result<T>>,
        F: FnOnce(T) -> Log;
}

#[async_trait]
impl<T: Debug + Send + Sync + 'static> Logger<T> for anyhow::Result<T> {
    async fn logging(self) -> anyhow::Result<(), !>
    where
        Self: SameAs<anyhow::Result<T>>,
    {
        let tx = CENTRAL.sender();
        tokio::task::spawn(async move {
            match self {
                Ok(msg) => {
                    let _ = tx.send(Msg::Ok(format!("{msg:?}"))).await;
                }
                Err(err) => {
                    let _ = tx.send(Msg::Err(err)).await;
                }
            }
        });
        Ok(())
    }

    async fn logging_with<F: Send + Sync + 'static, Log: Display>(
        self,
        f: F,
    ) -> anyhow::Result<(), !>
    where
        Self: SameAs<anyhow::Result<T>>,
        F: FnOnce(T) -> Log,
    {
        let tx = CENTRAL.sender();
        tokio::task::spawn(async move {
            match self {
                Ok(value) => {
                    let _ = tx.send(Msg::Ok(format!("{}", f(value)))).await;
                }
                Err(err) => {
                    let _ = tx.send(Msg::Err(err)).await;
                }
            }
        });
        Ok(())
    }
}

trait AsEmbed {
    fn as_embed(&self) -> CreateEmbed;
}

impl AsEmbed for anyhow::Error {
    fn as_embed(&self) -> CreateEmbed {
        let mut embed = CreateEmbed::default();
        embed
            .colour(Colour::RED)
            .title("ERROR")
            .field("description:", format!("{self:#?}"), false);
        embed
    }
}

/// Handler for the BOT
#[derive(Debug)]
struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: serenity::client::Context, _ready: Ready) {
        let commands = commands::create_slash_commands(&ctx.http).await;
        if let Ok(command) = commands {
            for cmd in command.iter().filter(|cmd| {
                !COMMANDS
                    .iter()
                    .any(|expected| cmd.name.starts_with(expected))
            }) {
                let _ =
                    ApplicationCommand::delete_global_application_command(&ctx.http, cmd.id).await;
            }
        }
        println!("successfully connected!!");
        let commands = ApplicationCommand::get_global_application_commands(&ctx.http).await;
        println!("I now have the following global slash commands: {commands:#?}");
    }

    async fn interaction_create(&self, ctx: serenity::client::Context, interaction: Interaction) {
        use regexsoup::parser::CommandParser;

        if let Some(command) = interaction.clone().application_command() {
            let flat_data = command.data.parse().unwrap();
            let (head, tail) = flat_data.split_first().unwrap();
            let dictionary = tail.iter().cloned().collect::<HashMap<_, _>>();

            match head {
                (_, Notification::SlashCommand(SlashCommand::Command(cmd))) if cmd.eq("start") => {
                    println!("cmd: start");
                    let difficulty: NonZeroU8 = (dictionary
                        .get("size")
                        .map_or_else(|| Ok(3i64), |size| size.to::<i64>())
                        .unwrap() as u8)
                        .try_into()
                        .unwrap();
                    let res = CONTAINER.fresh(command.channel_id, difficulty).await;
                    let _ = command
                        .embed(&ctx.http, res.unwrap_or_else(|why| why.as_embed()))
                        .await
                        .with_context(|| anyhow!("ERROR: fail to interaction"))
                        .logging_with(|_| {
                            "parse error: successfully finished to send error message."
                        })
                        .await;
                }
                (_, Notification::SlashCommand(SlashCommand::Command(cmd))) if cmd.eq("query") => {
                    println!("cmd: query");
                    tokio::task::spawn(async move {
                        let input = dictionary.get("input").unwrap().to::<String>().unwrap();
                        let is_match = CONTAINER
                            .checked_command(command.channel_id, command.user.id, |quiz| {
                                quiz.query(&input)
                            })
                            .await
                            .flatten();

                        match is_match {
                            Ok(is_match) => {
                                let _ = command
                                    .message(&ctx.http, is_match)
                                    .await
                                    .with_context(|| anyhow!("ERROR: fail to interaction"))
                                    .logging_with(|_| "successfully finished query command.")
                                    .await;
                            }
                            Err(why) => {
                                let _ = command
                                    .embed(&ctx.http, why.as_embed())
                                    .await
                                    .with_context(|| anyhow!("ERROR: fail to interaction"))
                                    .logging_with(move |_| format!("{why:#?}"))
                                    .await;
                            }
                        }
                    });
                }
                (_, Notification::SlashCommand(SlashCommand::Command(cmd))) if cmd.eq("guess") => {
                    println!("cmd: guess");
                    tokio::task::spawn(async move {
                        let input = dictionary.get("regex").unwrap().to::<String>().unwrap();

                        let inspection = CONTAINER
                            .checked_command(command.channel_id, command.user.id, |quiz| {
                                quiz.inspect(&input)
                            })
                            .await
                            .flatten();

                        match inspection {
                            Ok(res) => {
                                if let InspectionAcceptance::Accepted(_) = res {
                                    CONTAINER.delete(command.channel_id).await;
                                }
                                let _ = command
                                    .message(&ctx.http, res)
                                    .await
                                    .with_context(|| anyhow!("ERROR: fail to interaction"))
                                    .logging_with(|_| "successfully finished guess command.")
                                    .await;
                            }
                            Err(why) => {
                                let _ = command
                                    .embed(&ctx.http, why.as_embed())
                                    .await
                                    .with_context(|| anyhow!("ERROR: fail to interaction"))
                                    .logging_with(move |_| format!("{why:#?}"))
                                    .await;
                            }
                        }
                    });
                }
                (_, Notification::SlashCommand(SlashCommand::Command(cmd)))
                    if cmd.eq("summary") =>
                {
                    println!("cmd: summary");
                    tokio::task::spawn(async move {
                        let summary = CONTAINER
                            .checked_command(command.channel_id, command.user.id, |quiz| {
                                quiz.get_query_history()
                            })
                            .await;
                        match summary {
                            Ok(summary) => {
                                let _ = command
                                    .embed(&ctx.http, summary)
                                    .await
                                    .with_context(|| anyhow!("ERROR: fail to interaction"))
                                    .logging_with(|_| "successfully finished summary command.")
                                    .await;
                            }
                            Err(why) => {
                                let _ = command
                                    .message(&ctx.http, format!("{why}"))
                                    .await
                                    .with_context(|| anyhow!("ERROR: fail to interaction"))
                                    .logging_with(move |_| format!("{why}"))
                                    .await;
                            }
                        }
                    });
                }
                (_, Notification::SlashCommand(SlashCommand::Command(cmd))) if cmd.eq("join") => {
                    println!("cmd: join");
                    tokio::task::spawn(async move {
                        let res = CONTAINER
                            .checked_command(command.channel_id, command.user.id, |quiz| {
                                quiz.register(command.user.id)
                            })
                            .await
                            .flatten()
                            .map(|_| format!("{} is added.", command.user.name));

                        match res {
                            Ok(msg) => {
                                let _ = command
                                    .message(&ctx.http, &msg)
                                    .await
                                    .with_context(|| anyhow!("ERROR: fail to interaction"))
                                    .logging_with(|_| "successfully finished join command.")
                                    .await;
                            }
                            Err(why) => {
                                let _ = command
                                    .message(&ctx.http, format!("{why}"))
                                    .await
                                    .with_context(|| anyhow!("ERROR: fail to interaction"))
                                    .logging_with(move |_| format!("{why}"))
                                    .await;
                            }
                        }
                    });
                }
                (_, Notification::SlashCommand(SlashCommand::Command(cmd)))
                    if cmd.eq("give-up") =>
                {
                    println!("cmd: give-up");
                    tokio::task::spawn(async move {
                        let res = CONTAINER
                            .checked_command(command.channel_id, command.user.id, |quiz| {
                                quiz.accepts_give_up(&command.user)
                            })
                            .await
                            .flatten();

                        match res {
                            Ok(either) => match either {
                                Either::Right((content, buttons)) => {
                                    CONTAINER.delete(command.channel_id).await;
                                    let _ = command
                                        .button(&ctx.http, content, buttons)
                                        .await
                                        .with_context(|| anyhow!("ERROR: fail to interaction"))
                                        .logging_with(|_| "successfully finished give-up command.")
                                        .await;
                                }
                                Either::Left(msg) => {
                                    let _ = command
                                        .message(&ctx.http, &msg)
                                        .await
                                        .with_context(|| anyhow!("ERROR: fail to interaction"))
                                        .logging_with(|_| "successfully finished give-up command.")
                                        .await;
                                }
                            },
                            Err(why) => {
                                let _ = command
                                    .message(&ctx.http, format!("{why}"))
                                    .await
                                    .with_context(|| anyhow!("ERROR: fail to interaction"))
                                    .logging_with(move |_| format!("{why}"))
                                    .await;
                            }
                        }
                    });
                }
                (_, Notification::SlashCommand(SlashCommand::Command(cmd))) if cmd.eq("help") => {
                    let _ = command
                        .embed(&ctx.http, commands::help())
                        .await
                        .with_context(|| anyhow!("ERROR: fail to interaction"))
                        .logging_with(|_| "successfully finished help command.")
                        .await;
                }
                (_, unknown) => {
                    let _ = CENTRAL
                        .sender()
                        .send(Msg::Err(anyhow::anyhow!("unknown command: {:?}", unknown)))
                        .await;
                }
            }
        } else if let Some(component) = interaction.clone().message_component() {
            let data = component.data.parse().unwrap();
            match data {
                CustomId::Feedback { label, regex } => {
                    println!("{regex} => {label}");
                    let _ = component
                        .message(&ctx.http, "ありがとうございました")
                        .await
                        .with_context(|| anyhow!("ERROR: fail to interaction"))
                        .logging_with(|_| "successfully finished feedback.")
                        .await;
                }
            }
        }
    }
}

pub async fn build_bot_client(
    token: impl AsRef<str>,
    application_id: u64,
) -> anyhow::Result<Client> {
    // Build our client.
    Client::builder(token)
        .event_handler(Handler)
        .application_id(application_id)
        .await
        .with_context(|| anyhow!("ERROR: failed to build client"))
}

/// Sender/Receiver
pub static CENTRAL: Lazy<Tsx<Msg>> = Lazy::new(|| {
    let (sender, receiver) = channel(8);
    Tsx {
        sender: Arc::new(sender),
        receiver: Arc::new(Mutex::new(receiver)),
    }
});

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Configure the client with your Discord bot token in the environment.
    let token = std::env::var("REGEX_SOUP_TOKEN").expect("`REGEX_SOUP_TOKEN` is not found");

    // The Application Id is usually the Bot User Id.
    let application_id = std::env::var("REGEX_SOUP_ID")
        .expect("`REGEX_SOUP_ID` is not found")
        .parse::<u64>()
        .unwrap();

    // spawn bot client
    tokio::spawn(async move {
        let mut client = build_bot_client(token, application_id)
            .await
            .expect("client");
        if let Err(why) = client.start().await {
            println!("{why:#?}");
        }
    });

    // lock receiver
    if let Ok(ref mut guardian) = CENTRAL.receiver().try_lock() {
        let rx = &mut *guardian;
        // streaming
        while let Some(msg) = rx.recv().await {
            match msg {
                Msg::Ok(log) => println!("{log}"),
                Msg::Err(why) => println!("{why:#?}"),
            }
        }
    }
    Ok(())
}
