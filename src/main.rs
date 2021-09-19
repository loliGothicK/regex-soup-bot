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

use anyhow::{anyhow, Context};
use counted_array::counted_array;
use indoc::indoc;
use once_cell::sync::Lazy;
use regexsoup::{
    bot::{Container, Msg, Tsx},
    command_ext::CommandExt,
    commands,
    concepts::SameAs,
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
            Interaction,
        },
    },
    utils::Colour,
};
use std::{
    collections::HashMap,
    convert::TryInto,
    fmt::{Debug, Display},
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
                    println!("cmd: start");
                    let difficulty: NonZeroU8 = (dictionary
                        .get("size")
                        .map_or_else(|| Ok(3i64), |size| size.to::<i64>())
                        .unwrap() as u8)
                        .try_into()
                        .unwrap();

                    let quiz = commands::generate_regex(difficulty).await;

                    match quiz {
                        Ok(quiz) => {
                            CONTAINER
                                .lock()
                                .unwrap()
                                .channel_map
                                .insert(command.channel_id, Some(quiz));
                            let _ = command
                                .message(&ctx.http, "新しいREGガメのスープを開始します")
                                .await
                                .with_context(|| anyhow!("ERROR: fail to interaction"))
                                .logging_with(|_| "successfully started new regex-soup.")
                                .await;
                        }
                        Err(why) => {
                            let mut embed = CreateEmbed::default();
                            embed.colour(Colour::RED).title("ERROR").field(
                                "reason:",
                                format!("{why}"),
                                false,
                            );
                            let _ = command
                                .embed(&ctx.http, embed)
                                .await
                                .with_context(|| anyhow!("ERROR: fail to interaction"))
                                .logging_with(move |_| format!("ERROR: {why}"))
                                .await;
                        }
                    }
                }
                (_, Notification::SlashCommand(SlashCommand::Command(cmd))) if cmd.eq("query") => {
                    println!("cmd: query");
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
                                .message(&ctx.http, "まずは`join`コマンドで参加を登録してください")
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
                                let _ = command
                                    .message(&ctx.http, &msg)
                                    .await
                                    .with_context(|| anyhow!("ERROR: fail to interaction"))
                                    .logging_with(|_| "successfully finished query command.")
                                    .await;
                            }
                            Err(why) => {
                                let mut embed = CreateEmbed::default();
                                embed.colour(Colour::RED).title("ERROR").field(
                                    "reason: ",
                                    format!("{why}"),
                                    false,
                                );
                                let _ = command
                                    .embed(&ctx.http, embed)
                                    .await
                                    .with_context(|| anyhow!("ERROR: fail to interaction"))
                                    .logging_with(|_| {
                                        "parse error: successfully finished to send error message."
                                    })
                                    .await;
                            }
                        }
                    });
                }
                (_, Notification::SlashCommand(SlashCommand::Command(cmd))) if cmd.eq("guess") => {
                    println!("cmd: guess");
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
                                .message(&ctx.http, "まずは`join`コマンドで参加を登録してください")
                                .await
                                .with_context(|| anyhow!("ERROR: fail to interaction"))
                                .logging_with(|_| {
                                    "not yet joined: successfully finished to send error message."
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
                                                if quiz.guess(&valid_input) {
                                                    (
                                                        format!(
                                                            indoc! {"
                                                            - `{}` => AC
                                                            - original answer is `{}`
                                                            - {} queries
                                                        "},
                                                            original_input,
                                                            quiz.get_answer_regex(),
                                                            quiz.len(),
                                                        ),
                                                        true,
                                                    )
                                                } else {
                                                    (format!("`{original_input}` => WA"), false)
                                                }
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
                                let _ = command
                                    .message(&ctx.http, &msg)
                                    .await
                                    .with_context(|| anyhow!("ERROR: fail to interaction"))
                                    .logging_with(|_| "successfully finished guess command.")
                                    .await;
                            }
                            Err(why) => {
                                let mut embed = CreateEmbed::default();
                                embed.colour(Colour::RED).title("ERROR").field(
                                    "reason: ",
                                    format!("{why}"),
                                    false,
                                );
                                let _ = command
                                    .embed(&ctx.http, embed)
                                    .await
                                    .with_context(|| anyhow!("ERROR: fail to interaction"))
                                    .logging_with(|_| {
                                        "invalid input: successfully finished to send error \
                                         message."
                                    })
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
                        let embed = CONTAINER
                            .lock()
                            .unwrap()
                            .channel_map
                            .get(&command.channel_id)
                            .map_or_else(
                                || {
                                    let mut embed = CreateEmbed::default();
                                    embed.colour(Colour::DARK_RED).title("ERROR").field(
                                        "reason: ",
                                        "ゲームが開始してません",
                                        false,
                                    );
                                    embed
                                },
                                |quiz| {
                                    quiz.as_ref().map_or_else(
                                        || {
                                            let mut embed = CreateEmbed::default();
                                            embed.colour(Colour::DARK_RED).title("ERROR").field(
                                                "reason: ",
                                                "ゲームが開始してません",
                                                false,
                                            );
                                            embed
                                        },
                                        |quiz| quiz.get_query_history(),
                                    )
                                },
                            );
                        let _ = command
                            .embed(&ctx.http, embed)
                            .await
                            .with_context(|| anyhow!("ERROR: fail to interaction"))
                            .logging_with(|_| "parse error: successfully finished summary command.")
                            .await;
                    });
                }
                (_, Notification::SlashCommand(SlashCommand::Command(cmd))) if cmd.eq("join") => {
                    println!("cmd: join");
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
                                            |_| "すでに登録されています".to_string(),
                                            |_| format!("{} is added.", command.user.name.clone()),
                                        )
                                    } else {
                                        "まずは`start`コマンドでゲームを開始してください"
                                            .to_string()
                                    }
                                },
                            );

                        let _ = command
                            .message(&ctx.http, &msg)
                            .await
                            .with_context(|| anyhow!("ERROR: fail to interaction"))
                            .logging_with(|_| "successfully finished join command.")
                            .await;
                    });
                }
                (_, Notification::SlashCommand(SlashCommand::Command(cmd)))
                    if cmd.eq("give-up") =>
                {
                    println!("cmd: give-up");
                    tokio::task::spawn(async move {
                        let (msg, end) = CONTAINER
                            .lock()
                            .unwrap()
                            .channel_map
                            .get_mut(&command.channel_id)
                            .map_or_else(
                                || {
                                    (
                                        "まずは`start`コマンドでゲームを開始してください"
                                            .to_string(),
                                        None,
                                    )
                                },
                                |quiz| {
                                    if let Some(quiz) = quiz {
                                        quiz.accepts_give_up(&command.user.id).map_or_else(
                                            |_| ("まだ登録されていません".to_string(), None),
                                            |_| {
                                                (
                                                    format!(
                                                        "{} is removed.",
                                                        command.user.name.clone()
                                                    ),
                                                    quiz.is_empty()
                                                        .then(|| quiz.get_answer_regex()),
                                                )
                                            },
                                        )
                                    } else {
                                        (
                                            "まずは`start`コマンドでゲームを開始してください"
                                                .to_string(),
                                            None,
                                        )
                                    }
                                },
                            );
                        if let Some(ans) = end {
                            CONTAINER
                                .lock()
                                .unwrap()
                                .channel_map
                                .entry(command.channel_id)
                                .and_modify(|quiz| *quiz = None);
                            let _ = command.message(&ctx.http, format!("`{ans}`")).await;
                            return;
                        }
                        let _ = command
                            .message(&ctx.http, &msg)
                            .await
                            .with_context(|| anyhow!("ERROR: fail to interaction"))
                            .logging_with(|_| "successfully finished give-up command.")
                            .await;
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
                    let _ = command
                        .embed(&ctx.http, embed)
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
            let _ = component.data.parse().unwrap();
            // TODO:
        }
    }
}

pub async fn bot_client(token: impl AsRef<str>, application_id: u64) -> anyhow::Result<Client> {
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
    // Configure the client with your Discord bot token in the environment.
    let token = std::env::var("REGEX_SOUP_TOKEN").unwrap();

    // The Application Id is usually the Bot User Id.
    let application_id = std::env::var("REGEX_SOUP_ID")
        .unwrap()
        .parse::<u64>()
        .unwrap();

    // spawn bot client
    tokio::spawn(async move {
        let mut client = bot_client(token, application_id).await.expect("client");
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
