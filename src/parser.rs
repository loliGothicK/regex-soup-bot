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

use crate::notification::{Notification, SlashCommand};

use serde::{Deserialize, Serialize};
use serenity::model::interactions::{
    application_command::{
        ApplicationCommandInteractionData, ApplicationCommandInteractionDataOption,
        ApplicationCommandOptionType,
    },
    message_component::{ComponentType, MessageComponentInteractionData},
};

type DataOptions = Vec<ApplicationCommandInteractionDataOption>;

pub trait CommandParser {
    fn parse(&self) -> anyhow::Result<Vec<(String, Notification)>>;
}
pub trait ComponentParser {
    fn parse(&self) -> anyhow::Result<CustomId>;
}

/// # Parse an Message Component
/// Parse an interaction containing messages.
/// More detail, see [DEVELOPER PORTAL](https://discord.com/developers/docs/interactions/slash-commands#data-models-and-types).
impl CommandParser for ApplicationCommandInteractionData {
    fn parse(&self) -> anyhow::Result<Vec<(String, Notification)>> {
        type ParserImpl<'a> = &'a dyn Fn(
            &Parser,
            &mut Vec<(String, Notification)>,
            &DataOptions,
        ) -> anyhow::Result<Vec<(String, Notification)>>;

        let mut items = vec![(
            "command".to_string(),
            Notification::SlashCommand(SlashCommand::Command(self.name.clone())),
        )];

        struct Parser<'a> {
            parser: ParserImpl<'a>,
        }

        let parser = Parser {
            parser: &|succ, ret, options| {
                if options.is_empty() {
                    Ok(ret.clone())
                } else {
                    type Type = ApplicationCommandOptionType;
                    for option in options {
                        match option.kind {
                            Type::SubCommand => {
                                ret.push((
                                    "sub_command".to_string(),
                                    Notification::SlashCommand(SlashCommand::SubCommand(
                                        option.name.clone(),
                                    )),
                                ));
                            }
                            Type::String
                            | Type::Integer
                            | Type::Boolean
                            | Type::User
                            | Type::Channel
                            | Type::Role => {
                                ret.push((
                                    option.name.clone(),
                                    Notification::SlashCommand(SlashCommand::Option(Box::new(
                                        option.resolved.as_ref().unwrap().clone(),
                                    ))),
                                ));
                            }
                            x => {
                                anyhow::bail!("invalid option type: {:?}", x);
                            }
                        }
                    }
                    if let Some(last) = options.last() {
                        (succ.parser)(succ, ret, &last.options)
                    } else {
                        Ok(ret.clone())
                    }
                }
            },
        };
        (parser.parser)(&parser, &mut items, &self.options)
    }
}

#[derive(Serialize, Deserialize)]
pub enum CustomId {
    Feedback { label: String, regex: String },
}

impl ToString for CustomId {
    fn to_string(&self) -> String {
        serde_json::to_string(&self).expect("valid json")
    }
}

/// # Parse an Message Component
/// Parse an interaction containing messages.
/// More detail, see [DEVELOPER PORTAL](https://discord.com/developers/docs/interactions/message-components).
impl ComponentParser for MessageComponentInteractionData {
    fn parse(&self) -> anyhow::Result<CustomId> {
        match self.component_type {
            // [Buttons](https://discord.com/developers/docs/interactions/message-components#buttons)
            ComponentType::Button => Ok(serde_json::from_str(&self.custom_id)?),
            // [Select Menus](https://discord.com/developers/docs/interactions/message-components#select-menus)
            ComponentType::SelectMenu => unimplemented!(),
            _ => anyhow::bail!("{:?}", &self),
        }
    }
}
