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

use crate::regex;
use anyhow::anyhow;
use indexmap::{indexmap, indexset, IndexMap, IndexSet};
use serenity::{
    builder::{CreateInteractionResponse, EditInteractionResponse},
    http::Http,
    model::{
        id::{ChannelId, UserId},
        interactions::{
            application_command::ApplicationCommandInteraction,
            message_component::MessageComponentInteraction,
        },
    },
};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::{Receiver, Sender};
use std::convert::TryInto;

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
    regex: regex::RegexAst,
    #[allow(unused)]
    history: IndexSet<String>,
    participants: IndexSet<UserId>,
}

#[allow(dead_code)]
pub struct Container {
    workers: IndexMap<ChannelId, Option<Quiz>>,
}

pub enum QueryResult {
    Yes,
    No,
}

impl Quiz {
    pub fn new() -> Self {
        Self {
            regex: regex::randomly_generate(&regex::Difficulty(3u8.try_into().unwrap())),
            history: indexset! {},
            participants: indexset! {},
        }
    }

    pub async fn query(&mut self, _text: &str) -> QueryResult {
        // TODO: converts text to Alphabet
        let alphabets: Vec<regex::Alphabet> = Vec::new();
        if self.regex.matches(&alphabets) {
            QueryResult::Yes
        } else {
            QueryResult::No
        }
    }

    pub async fn register(&mut self, user: UserId) -> anyhow::Result<()> {
        self.participants
            .insert(user)
            .then(|| ())
            .ok_or_else(|| anyhow!("already registered."))
    }

    pub async fn guess(&self, regexp: &str) -> anyhow::Result<bool> {
        let ast = regex::RegexAst::parse_str(regexp)?;
        Ok(self.regex.equivalent_to(&ast))
    }

    pub async fn accepts_give_up(&mut self, user: &UserId) -> anyhow::Result<()> {
        self.participants
            .remove(user)
            .then(|| ())
            .ok_or_else(|| anyhow!("not registered"))
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
            workers: indexmap! {},
        }
    }
}

impl Default for Container {
    fn default() -> Self {
        Self::new()
    }
}
