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

use crate::regex::{randomly_generate, Alphabet, Difficulty, RegexAst};
use anyhow::anyhow;
use indexmap::{indexmap, indexset, IndexMap, IndexSet};
use serenity::{
    builder::{CreateEmbed, CreateInteractionResponse, EditInteractionResponse},
    http::Http,
    model::{
        id::{ChannelId, UserId},
        interactions::{
            application_command::ApplicationCommandInteraction,
            message_component::MessageComponentInteraction,
        },
    },
    utils::Colour,
};
use std::{
    convert::TryInto,
    num::NonZeroU8,
    sync::{Arc, Mutex},
};
use tokio::sync::mpsc::{Receiver, Sender};

/// Struct that holds sender and receiver
pub struct Tsx<T> {
    pub sender: Arc<Sender<T>>,
    pub receiver: Arc<Mutex<Receiver<T>>>,
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

pub enum Interactions {
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

pub enum Msg {
    Ok(String),
    Err(anyhow::Error),
}

pub struct Quiz {
    regex: RegexAst,
    history: IndexMap<String, String>,
    participants: IndexSet<UserId>,
}

#[allow(dead_code)]
pub struct Container {
    pub channel_map: IndexMap<ChannelId, Option<Quiz>>,
}

impl Quiz {
    pub fn new() -> Self {
        let regex = randomly_generate(&Difficulty(3u8.try_into().unwrap()));
        println!("{}", regex);
        Self {
            regex,
            history: indexmap! {},
            participants: indexset! {},
        }
    }

    pub fn new_with_difficulty(difficulty: NonZeroU8) -> Self {
        let regex = randomly_generate(&Difficulty(difficulty));
        println!("{}", regex);
        Self {
            regex,
            history: indexmap! {},
            participants: indexset! {},
        }
    }

    pub fn query(&mut self, input: &[Alphabet]) -> bool {
        let is_match = self.regex.matches(input);
        let input_string = Alphabet::slice_to_plain_string(input);
        self.history
            .entry(input_string)
            .or_insert((if is_match { "Yes" } else { "No" }).to_string());
        is_match
    }

    pub fn register(&mut self, user: UserId) -> anyhow::Result<()> {
        self.participants
            .insert(user)
            .then(|| ())
            .ok_or_else(|| anyhow!("already registered."))
    }

    pub fn guess(&self, input: &RegexAst) -> bool {
        self.regex.equivalent_to(input)
    }

    pub fn accepts_give_up(&mut self, user: &UserId) -> anyhow::Result<()> {
        self.participants
            .remove(user)
            .then(|| ())
            .ok_or_else(|| anyhow!("not registered"))
    }

    pub fn get_query_history(&self) -> CreateEmbed {
        let mut embed = CreateEmbed::default();
        embed.colour(Colour::DARK_BLUE).title("query history");
        if self.history.is_empty() {
            embed.field("Nothing to show", "-", false);
        }
        for (query, result) in self.history.iter() {
            embed.field(query.eq("").then(|| "ε").unwrap_or(query), dbg!(result.clone()), true);
        }
        embed
    }

    pub fn is_participant(&self, id: &UserId) -> bool {
        self.participants.contains(id)
    }

    pub fn is_empty(&self) -> bool {
        self.participants.is_empty()
    }

    pub fn len(&self) -> usize {
        self.participants.len()
    }

    pub fn get_answer_regex(&self) -> RegexAst {
        self.regex.clone()
    }
}

impl Default for Quiz {
    fn default() -> Self {
        Self::new()
    }
}

impl Container {
    pub fn new() -> Self {
        Self {
            channel_map: indexmap! {},
        }
    }
}

impl Default for Container {
    fn default() -> Self {
        Self::new()
    }
}
