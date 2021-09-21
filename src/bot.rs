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
use itertools::Itertools;
use serenity::{
    builder::CreateEmbed,
    model::id::{ChannelId, UserId},
    utils::Colour,
};
use std::{
    convert::TryInto,
    num::NonZeroU8,
    sync::{Arc, Mutex},
};
use strum::IntoEnumIterator;
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

/// opaque-type of `anyhow::Result<String>` for logging
pub enum Msg {
    Ok(String),
    Err(anyhow::Error),
}

pub struct Quiz {
    size: u8,
    regex: RegexAst,
    history: IndexMap<String, String>,
    participants: IndexSet<UserId>,
}

pub enum Inspection {
    Accepted(String),
    WrongAnswer(String),
}

impl ToString for Inspection {
    fn to_string(&self) -> String {
        match self {
            Inspection::Accepted(input) => format!("{input} => AC"),
            Inspection::WrongAnswer(input) => format!("{input} => WA"),
        }
    }
}

pub enum IsMatch {
    Yes(String),
    No(String),
}

impl ToString for IsMatch {
    fn to_string(&self) -> String {
        match self {
            IsMatch::Yes(input) => format!("{input} => Yes"),
            IsMatch::No(input) => format!("{input} => No"),
        }
    }
}

impl Quiz {
    pub fn new() -> Self {
        let regex = randomly_generate(&Difficulty(3u8.try_into().unwrap()));
        println!("{}", regex);
        Self {
            size: 3u8,
            regex,
            history: indexmap! {},
            participants: indexset! {},
        }
    }

    pub fn new_with_difficulty(difficulty: NonZeroU8) -> Self {
        let regex = randomly_generate(&Difficulty(difficulty));
        println!("{}", regex);
        Self {
            size: difficulty.into(),
            regex,
            history: indexmap! {},
            participants: indexset! {},
        }
    }

    pub fn query(&mut self, input: &str) -> anyhow::Result<IsMatch> {
        let alphabets = if input.eq(r#""""#) {
            vec![]
        } else {
            Alphabet::vec_from_str(input)?
        };
        self.validate(&alphabets)?;
        let is_match = self.regex.matches(&alphabets);
        self.history
            .entry(input.to_string())
            .or_insert((if is_match { "Yes" } else { "No" }).to_string());
        if is_match {
            Ok(IsMatch::Yes(input.to_string()))
        } else {
            Ok(IsMatch::No(input.to_string()))
        }
    }

    pub fn inspect(&self, input: &str) -> anyhow::Result<Inspection> {
        let ast = RegexAst::parse_str(input)?;
        let alphabets = ast.used_alphabets().iter().cloned().collect_vec();
        self.validate(&alphabets)?;
        Ok(self
            .regex
            .equivalent_to(&ast)
            .then(|| Inspection::Accepted(format!("{} => AC", &input)))
            .unwrap_or_else(|| Inspection::WrongAnswer(format!("{} => WA", &input))))
    }

    pub fn register(&mut self, user: UserId) -> anyhow::Result<()> {
        self.participants
            .insert(user)
            .then(|| ())
            .ok_or_else(|| anyhow!("already registered."))
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
            embed.field(
                query.eq("").then(|| "ε").unwrap_or(query),
                dbg!(result.clone()),
                true,
            );
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
        self.history.len()
    }

    pub fn get_answer_regex(&self) -> RegexAst {
        self.regex.clone()
    }

    fn validate(&self, input: &[Alphabet]) -> anyhow::Result<()> {
        let domain = Alphabet::iter().take(self.size.into()).collect_vec();
        let invalid = input.iter().filter(|c| !domain.contains(c)).collect_vec();
        invalid.is_empty().then(|| ()).ok_or_else(|| {
            anyhow!(
                indoc::indoc! {"
                    Domain Error: {:?} {}.
                    Valid Alphabets are {:?}.
                "},
                invalid,
                if invalid.len() == 1 {
                    "is not a valid Alphabet"
                } else {
                    "are not valid Alphabets"
                },
                domain
            )
        })
    }
}

impl Default for Quiz {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Container {
    pub channel_map: IndexMap<ChannelId, Option<Quiz>>,
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
