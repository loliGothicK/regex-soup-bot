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
use indoc::indoc;
use once_cell::sync::Lazy;
use regexsoup::{
    bot::{Container, Msg, Quiz, Tsx},
    notification::{Notification, SlashCommand, To},
    regex::{Alphabet, RegexAst},
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
    convert::TryInto,
    fmt::Debug,
    num::NonZeroU8,
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
        "help",
    ]
);

pub static CONTAINER: Lazy<Arc<Mutex<Container>>> = Lazy::new(|| {
    let container = Container::default();
    Arc::new(Mutex::new(container))
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
                        let difficulty: NonZeroU8 = (dictionary
                            .get("size")
                            .map_or_else(|| Ok(3i64), |size| size.to::<i64>())
                            .unwrap() as u8)
                            .try_into()
                            .unwrap();

                        CONTAINER
                            .lock()
                            .unwrap()
                            .channel_map
                            .entry(command.channel_id)
                            .and_modify(|quiz| {
                                *quiz = Some(Quiz::new_with_difficulty(difficulty));
                            })
                            .or_insert_with(|| Some(Quiz::new_with_difficulty(difficulty)));

                        let msg = "新しいREGガメのスープを開始します".to_string();
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
                        let is_joined = CONTAINER
                            .lock()
                            .unwrap()
                            .channel_map
                            .get_mut(&command.channel_id)
                            .map_or_else(
                                || false,
                                |quiz| {
                                    quiz.as_ref().map_or_else(
                                        || false,
                                        |quiz| quiz.is_participant(&command.user.id),
                                    )
                                },
                            );

                        if !is_joined {
                            let _ = command
                                .create_interaction_response(&ctx.http, |response| {
                                    response
                                        .kind(InteractionResponseType::ChannelMessageWithSource)
                                        .interaction_response_data(|message| {
                                            message.content(
                                                "まずは`join`コマンドで参加を登録してください",
                                            )
                                        })
                                })
                                .await;
                            return;
                        }

                        let original_input =
                            dictionary.get("input").unwrap().to::<String>().unwrap();
                        let input = if original_input == r#""""# {
                            Ok(vec![])
                        } else {
                            Alphabet::vec_from_str(&original_input)
                        };
                        match input {
                            Ok(valid_input) => {
                                let msg = CONTAINER
                                    .lock()
                                    .unwrap()
                                    .channel_map
                                    .get_mut(&command.channel_id)
                                    .map_or_else(
                                        || "ゲームが開始していません".to_string(),
                                        |quiz| {
                                            if let Some(quiz) = quiz {
                                                let is_match = quiz.query(&valid_input);
                                                format!(
                                                    "{original_input} => {}",
                                                    if is_match { "Yes" } else { "No" }
                                                )
                                            } else {
                                                "ゲームが開始していません".to_string()
                                            }
                                        },
                                    );
                                match command
                                    .create_interaction_response(&ctx.http, |response| {
                                        response
                                            .kind(InteractionResponseType::ChannelMessageWithSource)
                                            .interaction_response_data(|message| {
                                                message.content(&msg)
                                            })
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
                            }
                            Err(why) => {
                                let mut embed = CreateEmbed::default();
                                embed.colour(Colour::RED).title("ERROR").field(
                                    "reason: ",
                                    format!("{why}"),
                                    false,
                                );
                                match command
                                    .create_interaction_response(&ctx.http, |response| {
                                        response
                                            .kind(InteractionResponseType::ChannelMessageWithSource)
                                            .interaction_response_data(|message| {
                                                message.add_embed(embed)
                                            })
                                    })
                                    .await
                                {
                                    Ok(_) => {
                                        let _ = tx
                                            .send(Msg::Ok(
                                                "successfully finished error message.".to_owned(),
                                            ))
                                            .await;
                                    }
                                    Err(err) => {
                                        let _ = tx.send(Msg::Err(err.into())).await;
                                    }
                                }
                            }
                        }
                    });
                }
                (_, Notification::SlashCommand(SlashCommand::Command(cmd))) if cmd.eq("guess") => {
                    println!("cmd: guess");
                    let tx = CENTRAL.sender();
                    tokio::task::spawn(async move {
                        let is_joined = CONTAINER
                            .lock()
                            .unwrap()
                            .channel_map
                            .get_mut(&command.channel_id)
                            .map_or_else(
                                || false,
                                |quiz| {
                                    quiz.as_ref().map_or_else(
                                        || false,
                                        |quiz| quiz.is_participant(&command.user.id),
                                    )
                                },
                            );

                        if !is_joined {
                            let _ = command
                                .create_interaction_response(&ctx.http, |response| {
                                    response
                                        .kind(InteractionResponseType::ChannelMessageWithSource)
                                        .interaction_response_data(|message| {
                                            message.content(
                                                "まずは`join`コマンドで参加を登録してください",
                                            )
                                        })
                                })
                                .await;
                            return;
                        }

                        let original_input =
                            dictionary.get("regex").unwrap().to::<String>().unwrap();
                        let input = RegexAst::parse_str(&original_input);
                        match input {
                            Ok(valid_input) => {
                                let (msg, is_accepted) = CONTAINER
                                    .lock()
                                    .unwrap()
                                    .channel_map
                                    .get_mut(&command.channel_id)
                                    .map_or_else(
                                        || ("ゲームが開始していません".to_string(), false),
                                        |quiz| {
                                            if let Some(quiz) = quiz {
                                                let is_match = quiz.guess(&valid_input);
                                                (
                                                    format!(
                                                        "`{original_input}` => {}",
                                                        if is_match { "AC" } else { "WA" }
                                                    ),
                                                    is_match,
                                                )
                                            } else {
                                                ("ゲームが開始していません".to_string(), false)
                                            }
                                        },
                                    );
                                if is_accepted {
                                    CONTAINER
                                        .lock()
                                        .unwrap()
                                        .channel_map
                                        .entry(command.channel_id)
                                        .and_modify(|quiz| *quiz = None);
                                }
                                match command
                                    .create_interaction_response(&ctx.http, |response| {
                                        response
                                            .kind(InteractionResponseType::ChannelMessageWithSource)
                                            .interaction_response_data(|message| {
                                                message.content(&msg)
                                            })
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
                            }
                            Err(why) => {
                                let mut embed = CreateEmbed::default();
                                embed.colour(Colour::RED).title("ERROR").field(
                                    "reason: ",
                                    format!("{why}"),
                                    false,
                                );
                                match command
                                    .create_interaction_response(&ctx.http, |response| {
                                        response
                                            .kind(InteractionResponseType::ChannelMessageWithSource)
                                            .interaction_response_data(|message| {
                                                message.add_embed(embed)
                                            })
                                    })
                                    .await
                                {
                                    Ok(_) => {
                                        let _ = tx
                                            .send(Msg::Ok(
                                                "successfully finished error message.".to_owned(),
                                            ))
                                            .await;
                                    }
                                    Err(err) => {
                                        let _ = tx.send(Msg::Err(err.into())).await;
                                    }
                                }
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
                        let embed = CONTAINER
                            .lock()
                            .unwrap()
                            .channel_map
                            .get(&command.channel_id)
                            .unwrap()
                            .as_ref()
                            .map(|quiz| quiz.get_query_history())
                            .unwrap_or_else(|| {
                                let mut embed = CreateEmbed::default();
                                embed.colour(Colour::DARK_RED).title("ERROR").field(
                                    "reason: ",
                                    "ゲームが開始してません",
                                    false,
                                );
                                embed
                            });
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
                        let msg = CONTAINER
                            .lock()
                            .unwrap()
                            .channel_map
                            .get_mut(&command.channel_id)
                            .map_or_else(
                                || "まずは`start`コマンドでゲームを開始してください".to_string(),
                                |quiz| {
                                    if let Some(quiz) = quiz {
                                        quiz.register(command.user.id).map_or_else(
                                            |_| "すでにとうとくされています".to_string(),
                                            |_| format!("{} is added.", command.user.name.clone()),
                                        )
                                    } else {
                                        "まずは`start`コマンドでゲームを開始してください"
                                            .to_string()
                                    }
                                },
                            );

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
                                    .send(Msg::Ok(format!(
                                        "{} is added.",
                                        command.user.name.clone()
                                    )))
                                    .await;
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
                        let (msg, is_empty) = CONTAINER
                            .lock()
                            .unwrap()
                            .channel_map
                            .get_mut(&command.channel_id)
                            .map_or_else(
                                || {
                                    (
                                        "まずは`start`コマンドでゲームを開始してください"
                                            .to_string(),
                                        false,
                                    )
                                },
                                |quiz| {
                                    if let Some(quiz) = quiz {
                                        quiz.accepts_give_up(&command.user.id).map_or_else(
                                            |_| ("まだとうとくされていません".to_string(), false),
                                            |_| {
                                                (
                                                    format!(
                                                        "{} is removed.",
                                                        command.user.name.clone()
                                                    ),
                                                    quiz.is_empty(),
                                                )
                                            },
                                        )
                                    } else {
                                        (
                                            "まずは`start`コマンドでゲームを開始してください"
                                                .to_string(),
                                            false,
                                        )
                                    }
                                },
                            );
                        if is_empty {
                            let ans = CONTAINER
                                .lock()
                                .unwrap()
                                .channel_map
                                .get(&command.channel_id)
                                .map(|x| x.as_ref().map(|quiz| quiz.get_answer_regex()));
                            if let Some(Some(ans)) = ans {
                                let _ = command
                                    .create_interaction_response(&ctx.http, |response| {
                                        response
                                            .kind(InteractionResponseType::ChannelMessageWithSource)
                                            .interaction_response_data(|message| {
                                                message.content(format!("`{ans}`"))
                                            })
                                    })
                                    .await;
                            }
                            CONTAINER
                                .lock()
                                .unwrap()
                                .channel_map
                                .entry(command.channel_id)
                                .and_modify(|quiz| *quiz = None);
                            return;
                        }
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
                                    .send(Msg::Ok(format!(
                                        "{} is removed.",
                                        command.user.name.clone()
                                    )))
                                    .await;
                            }
                            Err(err) => {
                                let _ = tx.send(Msg::Err(err.into())).await;
                            }
                        }
                    });
                }
                (_, Notification::SlashCommand(SlashCommand::Command(cmd))) if cmd.eq("help") => {
                    let mut embed = CreateEmbed::default();
                    embed.colour(Colour::DARK_GREEN).title("HELP");
                    embed
                        .field(
                            "REGEX-SOUP 101",
                            indoc! {
                                "`/start` => `/join` => `/query` => (`/summary`) => `/guess`"
                            },
                            false,
                        )
                        .field(
                            "/start [DIFFICULTY]",
                            indoc! {
                                "[DIFFICULTY]: number of alphabets"
                            },
                            false,
                        )
                        .field(
                            "/query [INPUT]",
                            indoc! {r#"
                                [INPUT]: alphabets to test (`""` is accepted as empty string)
                            "#},
                            false,
                        )
                        .field(
                            "/guess [INPUT]",
                            indoc! {r#"
                                Check your answer.
                                [INPUT]: regex you guess
                            "#},
                            false,
                        )
                        .field(
                            "/summary",
                            indoc! {r#"
                                Shows the history of querries.
                            "#},
                            false,
                        )
                        .field(
                            "/join",
                            indoc! {r#"
                                You have to `/join` first to take part in the quiz!.
                            "#},
                            false,
                        )
                        .field(
                            "/give-up",
                            indoc! {r#"
                                When all participants have `give-up`,
                                the quiz will end and the answers will be revealed!.
                            "#},
                            false,
                        );
                    let tx = CENTRAL.sender();
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
                                .send(Msg::Ok("successfully finished summary command.".to_owned()))
                                .await;
                        }
                        Err(err) => {
                            let _ = tx.send(Msg::Err(err.into())).await;
                        }
                    }
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
    // start [DIFFICULTY]: ゲームセッション開始コマンド
    // query: マッチクエリ
    // guess: 回答試行
    // summary: 今までのクエリのサマリ表示
    // join: 参加表明
    // give-up: 投了

    let _ = ApplicationCommand::create_global_application_command(&http, |a| {
        a.name("start")
            .description("Starting new regex-soup")
            .create_option(|o| {
                o.name("size")
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
            .description("Query whether is matched with regular expression.")
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
            .description("Check your answer.")
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

    let _ = ApplicationCommand::create_global_application_command(&http, |a| {
        a.name("help").description("helpful")
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
