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

use crate::concepts::SameAs;
use serenity::model::{interactions::application_command, user::User};

type OptionValue = serenity::model::interactions::application_command::ApplicationCommandInteractionDataOptionValue;

#[derive(Debug, Clone)]
pub enum SlashCommand {
    Command(String),
    SubCommand(String),
    Option(Box<application_command::ApplicationCommandInteractionDataOptionValue>),
}

#[derive(Debug, Clone)]
pub enum Component {
    Button(String),
    SelectMenu(Vec<String>),
}

#[derive(Debug, Clone)]
pub enum Notification {
    SlashCommand(SlashCommand),
    Component(Component),
}

pub trait To<Target> {
    fn to<T>(&self) -> anyhow::Result<Target>
    where
        T: SameAs<Target>;
}

impl To<String> for Notification {
    fn to<T>(&self) -> anyhow::Result<String>
    where
        T: SameAs<String>,
    {
        if let Notification::SlashCommand(SlashCommand::Option(boxed)) = self {
            if let OptionValue::String(value) = &**boxed {
                return Ok(value.clone());
            }
        }
        Err(anyhow::anyhow!(
            "cannot convert self to String: {:?}",
            &self
        ))
    }
}

impl To<User> for Notification {
    fn to<T>(&self) -> anyhow::Result<User>
    where
        T: SameAs<User>,
    {
        if let Notification::SlashCommand(SlashCommand::Option(boxed)) = self {
            if let OptionValue::User(user, ..) = &**boxed {
                return Ok(user.clone());
            }
        }
        Err(anyhow::anyhow!("cannot convert self to User {:?}", &self))
    }
}
