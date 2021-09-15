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
#![feature(async_closure)]

use anyhow::{anyhow, Context};
use counted_array::counted_array;

use once_cell::sync::Lazy;
use regexsoup::{
    bot::{Container, Msg, Tsx},
    notification::{Notification, SlashCommand, To},
};
use serenity::{
    async_trait,
    builder::CreateEmbed,
    client::{Client, EventHandler},
    http::Http,
    model::{
        gateway::Ready,
        interactions::{
            application_command::{ApplicationCommand, ApplicationCommandOptionType},
            Interaction, InteractionResponseType,
        },
    },
    utils::Colour,
};
use std::{
    collections::HashMap,
    fmt::Debug,
    sync::{Arc, Mutex},
};
use tokio::sync::mpsc::channel;

counted_array!(
    const COMMANDS: [&'static str; _] = [
        "start",
        "query",
        "guess",
        "summary",
        "join",
        "give-up",
    ]
);

#[allow(dead_code)]
pub static CONTAINER: Lazy<Arc<Container>> = Lazy::new(|| {
    let container = Container::default();
    Arc::new(container)
});

/// Handler for the BOT
#[derive(Debug)]
struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: serenity::client::Context, _ready: Ready) {
        let _ = create_slash_commands(&ctx.http).await;
        let interactions = ApplicationCommand::get_global_application_commands(&ctx.http).await;
        if let Ok(interactions) = interactions {
            for cmd in interactions.iter().filter(|cmd| {
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
        use regexsoup::parser::Parser;

        if let Some(command) = interaction.clone().application_command() {
            let flat_data = command.data.parse().unwrap();
            let (head, tail) = flat_data.split_first().unwrap();
            let dictionary = tail.iter().cloned().collect::<HashMap<_, _>>();

            match head {
                (_, Notification::SlashCommand(SlashCommand::Command(cmd))) if cmd.eq("start") => {
                    let tx = CENTRAL.sender();
                    tokio::task::spawn(async move {
                        let msg = "新しいREGガメのスープを開始しました".to_string();
                        match command
                            .create_interaction_response(&ctx.http, |response| {
                                response
                                    .kind(InteractionResponseType::ChannelMessageWithSource)
                                    .interaction_response_data(|message| message.content(&msg))
                            })
                            .await
                        {
                            Ok(_) => {
                                let _ = tx
                                    .send(Msg::Ok(
                                        "successfully started new regex-soup.".to_owned(),
                                    ))
                                    .await;
                            }
                            Err(err) => {
                                let _ = tx.send(Msg::Err(err.into())).await;
                            }
                        }
                    });
                }
                (_, Notification::SlashCommand(SlashCommand::Command(cmd))) if cmd.eq("query") => {
                    println!("cmd: query");
                    let tx = CENTRAL.sender();
                    tokio::task::spawn(async move {
                        let input = dictionary
                            .get(&"input".to_string())
                            .unwrap()
                            .to::<String>()
                            .unwrap();
                        let msg = format!("{input} => No");
                        match command
                            .create_interaction_response(&ctx.http, |response| {
                                response
                                    .kind(InteractionResponseType::ChannelMessageWithSource)
                                    .interaction_response_data(|message| message.content(&msg))
                            })
                            .await
                        {
                            Ok(_) => {
                                let _ = tx
                                    .send(Msg::Ok(
                                        "successfully finished query command.".to_owned(),
                                    ))
                                    .await;
                            }
                            Err(err) => {
                                let _ = tx.send(Msg::Err(err.into())).await;
                            }
                        }
                    });
                }
                (_, Notification::SlashCommand(SlashCommand::Command(cmd))) if cmd.eq("guess") => {
                    println!("cmd: guess");
                    let tx = CENTRAL.sender();
                    tokio::task::spawn(async move {
                        let regex = dictionary
                            .get(&"regex".to_string())
                            .unwrap()
                            .to::<String>()
                            .unwrap();
                        let msg = format!("{regex} => No");
                        match command
                            .create_interaction_response(&ctx.http, |response| {
                                response
                                    .kind(InteractionResponseType::ChannelMessageWithSource)
                                    .interaction_response_data(|message| message.content(&msg))
                            })
                            .await
                        {
                            Ok(_) => {
                                let _ = tx
                                    .send(Msg::Ok(
                                        "successfully finished guess command.".to_owned(),
                                    ))
                                    .await;
                            }
                            Err(err) => {
                                let _ = tx.send(Msg::Err(err.into())).await;
                            }
                        }
                    });
                }
                (_, Notification::SlashCommand(SlashCommand::Command(cmd)))
                    if cmd.eq("summary") =>
                {
                    println!("cmd: summary");
                    let tx = CENTRAL.sender();
                    tokio::task::spawn(async move {
                        let mut embed = CreateEmbed::default();
                        embed
                            .colour(Colour::DARK_BLUE)
                            .title("query history")
                            .field("dummy", "yes", false)
                            .field("dummy", "yes", false)
                            .field("dummy", "no", false);
                        match command
                            .create_interaction_response(&ctx.http, |response| {
                                response
                                    .kind(InteractionResponseType::ChannelMessageWithSource)
                                    .interaction_response_data(|message| message.add_embed(embed))
                            })
                            .await
                        {
                            Ok(_) => {
                                let _ = tx
                                    .send(Msg::Ok(
                                        "successfully finished summary command.".to_owned(),
                                    ))
                                    .await;
                            }
                            Err(err) => {
                                let _ = tx.send(Msg::Err(err.into())).await;
                            }
                        }
                    });
                }
                (_, Notification::SlashCommand(SlashCommand::Command(cmd))) if cmd.eq("join") => {
                    println!("cmd: join");
                    let tx = CENTRAL.sender();
                    tokio::task::spawn(async move {
                        let user_name = command.user.name.clone();
                        let msg = format!("{user_name} is added.");
                        match command
                            .create_interaction_response(&ctx.http, |response| {
                                response
                                    .kind(InteractionResponseType::ChannelMessageWithSource)
                                    .interaction_response_data(|message| message.content(&msg))
                            })
                            .await
                        {
                            Ok(_) => {
                                let _ = tx.send(Msg::Ok(format!("{user_name} is added."))).await;
                            }
                            Err(err) => {
                                let _ = tx.send(Msg::Err(err.into())).await;
                            }
                        }
                    });
                }
                (_, Notification::SlashCommand(SlashCommand::Command(cmd)))
                    if cmd.eq("give-up") =>
                {
                    println!("cmd: give-up");
                    let tx = CENTRAL.sender();
                    tokio::task::spawn(async move {
                        let user_name = command.user.name.clone();
                        let msg = format!("{user_name} is removed.");
                        match command
                            .create_interaction_response(&ctx.http, |response| {
                                response
                                    .kind(InteractionResponseType::ChannelMessageWithSource)
                                    .interaction_response_data(|message| message.content(&msg))
                            })
                            .await
                        {
                            Ok(_) => {
                                let _ = tx.send(Msg::Ok(format!("{user_name} is removed."))).await;
                            }
                            Err(err) => {
                                let _ = tx.send(Msg::Err(err.into())).await;
                            }
                        }
                    });
                }
                (_, unknown) => {
                    let _ = CENTRAL
                        .sender()
                        .send(Msg::Err(anyhow::anyhow!("unknown command: {:?}", unknown)))
                        .await;
                }
            }
        } else if let Some(component) = interaction.clone().message_component() {
            let _ = component.data.parse().unwrap();
            // TODO:
        }
    }
}

pub async fn bot_client() -> anyhow::Result<Client> {
    // Configure the client with your Discord bot token in the environment.
    let token = std::env::var("REGEX_SOUP_TOKEN").unwrap();

    // The Application Id is usually the Bot User Id.
    let application_id = std::env::var("REGEX_SOUP_ID")
        .unwrap()
        .parse::<u64>()
        .unwrap();

    // Build our client.
    Client::builder(token)
        .event_handler(Handler)
        .application_id(application_id)
        .await
        .with_context(|| anyhow!("ERROR: failed to build client"))
}

pub async fn create_slash_commands(http: impl AsRef<Http>) -> anyhow::Result<()> {
    // start [set] [level]: ゲームセッション開始コマンド
    // query: マッチクエリ
    // guess: 回答試行
    // summary: 今までのクエリのサマリ表示
    // join: 参加表明
    // give-up: 投了

    let _ = ApplicationCommand::create_global_application_command(&http, |a| {
        a.name("start")
            .description("Starting new regex-soup")
            .create_option(|o| {
                o.name("set")
                    .description("Please choice number of characters in the domain-set.")
                    .kind(ApplicationCommandOptionType::Integer)
                    .add_int_choice(1, 1)
                    .add_int_choice(2, 2)
                    .add_int_choice(3, 3)
                    .add_int_choice(4, 4)
                    .add_int_choice(5, 5)
                    .add_int_choice(6, 6)
                    .add_int_choice(7, 7)
                    .add_int_choice(8, 8)
                    .add_int_choice(9, 9)
                    .add_int_choice(10, 10)
                    .required(false)
            })
    })
    .await?;

    let _ = ApplicationCommand::create_global_application_command(&http, |a| {
        a.name("query")
            .description("Starting new regex-soup")
            .create_option(|o| {
                o.name("input")
                    .description("Please enter the input you wish to test for a match.")
                    .kind(ApplicationCommandOptionType::String)
                    .required(true)
            })
    })
    .await?;

    let _ = ApplicationCommand::create_global_application_command(&http, |a| {
        a.name("guess")
            .description("Starting new regex-soup")
            .create_option(|o| {
                o.name("regex")
                    .description("Please enter the regex you guess.")
                    .kind(ApplicationCommandOptionType::String)
                    .required(true)
            })
    })
    .await?;

    let _ = ApplicationCommand::create_global_application_command(&http, |a| {
        a.name("summary")
            .description("Dump the results of the query so far.")
    })
    .await?;

    let _ = ApplicationCommand::create_global_application_command(&http, |a| {
        a.name("join").description("Register your participation.")
    })
    .await?;

    let _ = ApplicationCommand::create_global_application_command(&http, |a| {
        a.name("give-up").description("Register your despair.")
    })
    .await?;

    Ok(())
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
    // spawn bot client
    tokio::spawn(async move {
        let mut client = bot_client().await.expect("client");
        if let Err(why) = client.start().await {
            println!("{why}");
        }
    });

    // lock receiver
    if let Ok(ref mut guardian) = CENTRAL.receiver().try_lock() {
        let rx = &mut *guardian;
        // streaming
        while let Some(msg) = rx.recv().await {
            match msg {
                Msg::Ok(log) => println!("{log}"),
                Msg::Err(why) => println!("{why}"),
            }
        }
    }
    Ok(())
}
