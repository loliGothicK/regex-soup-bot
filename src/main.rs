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

use anyhow::{anyhow, Context};
use counted_array::counted_array;
use regexsoup::{
    concepts::SameAs,
    notification::{Notification, SlashCommand},
    response::{self, Message, Response},
};
use once_cell::sync::Lazy;
use serenity::{
    async_trait,
    builder::{CreateEmbed, CreateInteractionResponse, EditInteractionResponse},
    client::{Client, EventHandler},
    http::Http,
    model::{
        gateway::Ready,
        interactions::{
            application_command::{
                ApplicationCommand, ApplicationCommandInteraction, ApplicationCommandOptionType,
            },
            message_component::MessageComponentInteraction,
            Interaction, InteractionResponseType,
        },
    },
    utils::Colour,
};
use std::{
    fmt::{Debug, Display},
    sync::{Arc, Mutex},
};
use tokio::sync::mpsc::{channel, Receiver, Sender};

pub trait MsgSender<Msg> {
    fn send_msg(self)
    where
        Self: SameAs<Msg>;
}

impl<T: Display + Send + Sync + 'static> MsgSender<anyhow::Result<T>> for anyhow::Result<T> {
    fn send_msg(self)
    where
        Self: SameAs<anyhow::Result<T>>,
    {
        let tx = CENTRAL.sender();
        match self {
            Ok(msg) => {
                tokio::spawn(async move {
                    let _ = tx.send(Msg::Ok(format!("{msg}"))).await;
                });
            }
            Err(err) => {
                tokio::spawn(async move {
                    let _ = tx.send(Msg::Err(format!("{err}"))).await;
                });
            }
        }
    }
}

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

enum Interactions {
    Command(ApplicationCommandInteraction),
    #[allow(dead_code)]
    Component(Box<MessageComponentInteraction>),
}

impl Interactions {
    pub async fn create_interaction_response<F>(
        &self,
        http: impl AsRef<Http>,
        f: F,
    ) -> anyhow::Result<()>
    where
        F: FnOnce(&mut CreateInteractionResponse) -> &mut CreateInteractionResponse,
    {
        match self {
            Interactions::Command(command) => command.create_interaction_response(http, f).await?,
            Interactions::Component(component) => {
                (*component).create_interaction_response(http, f).await?
            }
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn edit_original_interaction_response<F>(
        &self,
        http: impl AsRef<Http>,
        f: F,
    ) -> anyhow::Result<serenity::model::channel::Message>
    where
        F: FnOnce(&mut EditInteractionResponse) -> &mut EditInteractionResponse,
    {
        Ok(match self {
            Interactions::Command(command) => {
                command.edit_original_interaction_response(http, f).await?
            }
            Interactions::Component(component) => {
                (*component)
                    .edit_original_interaction_response(http, f)
                    .await?
            }
        })
    }
}

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
    }

    async fn interaction_create(&self, ctx: serenity::client::Context, interaction: Interaction) {
        use regexsoup::parser::Parser;
        let response: Option<Result<(Response, Interactions), (anyhow::Error, Interactions)>> = if let Some(command) = interaction.clone().application_command() {
            let dictionary = command.data.parse().unwrap();

            match &*dictionary {
                [(_, Notification::SlashCommand(SlashCommand::Command(cmd))), _num]
                    if cmd.eq("start") =>
                {
                    // TODO: call start
                    None
                }
                [(_, Notification::SlashCommand(SlashCommand::Command(cmd))), _str]
                    if cmd.eq("query") =>
                {
                    // TODO: call query
                    None
                }
                [(_, Notification::SlashCommand(SlashCommand::Command(cmd))), _regex]
                    if cmd.eq("guess") =>
                {
                    // TODO: call guess
                    None
                }
                [(_, Notification::SlashCommand(SlashCommand::Command(cmd)))]
                    if cmd.eq("summary") =>
                {
                    // TODO: call summary
                    None
                }
                [(_, Notification::SlashCommand(SlashCommand::Command(cmd)))]
                    if cmd.eq("join") =>
                {
                    // TODO: call join
                    None
                }
                [(_, Notification::SlashCommand(SlashCommand::Command(cmd)))]
                    if cmd.eq("give-up") =>
                {
                    // TODO: call give_up
                    None
                }
                [unknown, ..] => Some(Err((
                    anyhow::anyhow!("unknown command: {:?}", unknown),
                    Interactions::Command(command.clone()),
                ))),
                [] => Some(Err((
                    anyhow::anyhow!("empty command"),
                    Interactions::Command(command.clone()),
                ))),
            }
        } else if let Some(component) = interaction.clone().message_component() {
            let _ = component.data.parse().unwrap();
            // TODO:
            None
        } else {
            None
        };
        let result = if let Some(res) = response {
            res
        } else {
            // un-expected interaction => skip
            return;
        };

        match result {
            Err((err, interactions)) => {
                let mut embed = CreateEmbed::default();
                embed
                    .colour(Colour::RED)
                    .title("INTERACTION ERROR:")
                    .description(format!("{err:?}"));

                let json = serde_json::to_string(&embed.0);

                interactions
                    .create_interaction_response(&ctx.http, |response| {
                        response
                            .kind(InteractionResponseType::ChannelMessageWithSource)
                            .interaction_response_data(|message| message.add_embed(embed))
                    })
                    .await
                    .map(|_| format!(r#"{{ "notification" => "{json:?}" }}"#))
                    .map_err(|#[allow(unused)] err| anyhow!("http error: {err} with {json:?}"))
                    .send_msg();

                let _ = CENTRAL.sender().send(Msg::Err(format!("{err:?}"))).await;
            }
            Ok((response, interactions)) => match response {
                Response::Message(msg) => match msg {
                    Message::String(msg) => {
                        interactions
                            .create_interaction_response(&ctx.http, |response| {
                                response
                                    .kind(InteractionResponseType::ChannelMessageWithSource)
                                    .interaction_response_data(|message| message.content(&msg))
                            })
                            .await
                            .map(|_| format!(r#"{{ "notification" => "{msg}" }}"#))
                            .map_err(|#[allow(unused)] err| anyhow!("http error: {err} with {msg}"))
                            .send_msg();
                    }
                    Message::Embed(embed) => {
                        let json = serde_json::to_string(&embed.0);
                        interactions
                            .create_interaction_response(&ctx.http, |response| {
                                response
                                    .kind(InteractionResponseType::ChannelMessageWithSource)
                                    .interaction_response_data(|message| message.add_embed(embed))
                            })
                            .await
                            .map(|_| format!(r#"{{ "notification" => {json:?}"#))
                            .map_err(|#[allow(unused)] err| {
                                anyhow!("http error: {err} with {json:?}")
                            })
                            .send_msg();
                    }
                },
                Response::Components(component) => {
                    interactions
                        .create_interaction_response(&ctx.http, |response| {
                            response
                                .kind(InteractionResponseType::ChannelMessageWithSource)
                                .interaction_response_data(|data| match component {
                                    response::Component::Buttons { content, buttons } => {
                                        data.content(content).components(|components| {
                                            components.create_action_row(|action_row| {
                                                for button in buttons.into_iter() {
                                                    action_row.add_button(button);
                                                }
                                                action_row
                                            })
                                        })
                                    }
                                    response::Component::SelectMenu {
                                        custom_id,
                                        content,
                                        placeholder,
                                        min_value,
                                        max_value,
                                        options,
                                    } => data.content(content).components(|components| {
                                        components.create_action_row(|act| {
                                            act.create_select_menu(|select_menu| {
                                                select_menu
                                                    .placeholder(placeholder)
                                                    .custom_id(custom_id)
                                                    .min_values(min_value)
                                                    .max_values(max_value)
                                                    .options(|builder| {
                                                        for opt in options {
                                                            builder.create_option(|o| {
                                                                o.description(opt.description)
                                                                    .value(opt.value)
                                                                    .label(opt.label)
                                                            });
                                                        }
                                                        builder
                                                    })
                                            })
                                        })
                                    }),
                                })
                        })
                        .await
                        .map(|_| "Succeeded")
                        .map_err(|err| anyhow!("http error: {}", err))
                        .send_msg();
                }
            },
        }
    }
}

pub async fn bot_client() -> anyhow::Result<Client> {
    // Configure the client with your Discord bot token in the environment.
    let token = std::env::var("REGEX_SOUP_TOKEN").unwrap();

    // The Application Id is usually the Bot User Id.
    let application_id = std::env::var("BOT_ID").unwrap().parse::<u64>().unwrap();

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
                o.name("string")
                    .description("Please enter the string you wish to test for a match.")
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
        a.name("summary").description("Dump the results of the query so far.")
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

/// Struct that holds sender and receiver
pub struct Tsx<T> {
    sender: Arc<Sender<T>>,
    receiver: Arc<Mutex<Receiver<T>>>,
}

/// Getter for sender and receiver
impl<T> Tsx<T> {
    pub fn sender(&self) -> Arc<Sender<T>> {
        Arc::clone(&self.sender)
    }

    pub fn receiver(&self) -> Arc<Mutex<Receiver<T>>> {
        Arc::clone(&self.receiver)
    }
}

/// Sender/Receiver
pub static CENTRAL: Lazy<Tsx<Msg>> = Lazy::new(|| {
    let (sender, receiver) = channel(8);
    Tsx {
        sender: Arc::new(sender),
        receiver: Arc::new(Mutex::new(receiver)),
    }
});

pub enum Msg {
    Ok(String),
    Err(String),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // spawn bot client
    tokio::spawn(async move {
        let mut client = bot_client().await.expect("client");
        if let Err(why) = client.start().await {
            let _ = CENTRAL.sender().send(Msg::Err(format!("{why}"))).await;
        }
    });

    // lock receiver
    if let Ok(ref mut guardian) = CENTRAL.receiver().try_lock() {
        let rx = &mut *guardian;
        // streaming
        while let Some(msg) = rx.recv().await {
            match msg {
                Msg::Ok(msg) => {
                    println!("{msg:?}");
                }
                Msg::Err(err) => {
                    println!("{err:?}");
                }
            }
        }
    }
    Ok(())
}
