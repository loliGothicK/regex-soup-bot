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

use crate::{bot::Quiz, errors::CommandError};
use anyhow::{anyhow, Context};
use serenity::{
    builder::CreateEmbed,
    http::Http,
    model::interactions::application_command::{ApplicationCommand, ApplicationCommandOptionType},
    utils::Colour,
};
use std::{num::NonZeroU8, time::Duration};
use tokio::{sync::oneshot, time::timeout};

pub async fn generate_regex(difficulty: NonZeroU8) -> anyhow::Result<Quiz> {
    let (tx, rx) = oneshot::channel();

    tokio::task::spawn(async move {
        let quiz = Quiz::new_with_difficulty(difficulty);
        let _ = tx.send(quiz);
    });

    // Wrap the future with a `Timeout` set to expire in 1000 ms.
    match timeout(Duration::from_millis(1000), rx).await {
        Ok(quiz) => quiz.with_context(|| anyhow!("receive error")),
        Err(_) => Err(anyhow::Error::from(CommandError::Timeout {
            limit: "Time Limit Exceeded".to_string(),
        }))
        .context("timeout while generating regex"),
    }
}

pub fn help() -> CreateEmbed {
    use indoc::indoc;
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
                You have to `/join` first to take part in the quiz!
            "#},
            false,
        )
        .field(
            "/give-up",
            indoc! {r#"
                When all participants have `give-up`,
                the quiz will end and the answers will be revealed!
            "#},
            false,
        );
    embed
}

pub async fn create_slash_commands(
    http: impl AsRef<Http>,
) -> anyhow::Result<Vec<ApplicationCommand>> {
    // start [DIFFICULTY]: ゲームセッション開始コマンド
    // query: マッチクエリ
    // guess: 回答試行
    // summary: 今までのクエリのサマリ表示
    // join: 参加表明
    // give-up: 投了

    ApplicationCommand::set_global_application_commands(&http, |commands| {
        commands
            .create_application_command(|command| {
                command
                    .name("start")
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
            .create_application_command(|command| {
                command
                    .name("query")
                    .description("Query whether is matched with regular expression.")
                    .create_option(|o| {
                        o.name("input")
                            .description("Please enter the input you wish to test for a match.")
                            .kind(ApplicationCommandOptionType::String)
                            .required(true)
                    })
            })
            .create_application_command(|command| {
                command
                    .name("guess")
                    .description("Check your answer.")
                    .create_option(|o| {
                        o.name("regex")
                            .description("Please enter the regex you guess.")
                            .kind(ApplicationCommandOptionType::String)
                            .required(true)
                    })
            })
            .create_application_command(|command| {
                command
                    .name("summary")
                    .description("Dump the results of the query so far.")
            })
            .create_application_command(|command| {
                command
                    .name("join")
                    .description("Register your participation.")
            })
            .create_application_command(|command| {
                command
                    .name("give-up")
                    .description("Register your despair.")
            })
            .create_application_command(|command| command.name("help").description("helpful"))
    })
    .await
    .with_context(|| anyhow!("serenity error"))
}
