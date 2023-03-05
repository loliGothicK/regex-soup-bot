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

use crate::concepts::Satisfied;
use anyhow::{anyhow, Context};
use serenity::{
    async_trait,
    builder::{CreateButton, CreateEmbed},
    http::Http,
    model::interactions::{
        application_command::ApplicationCommandInteraction,
        message_component::MessageComponentInteraction, InteractionResponseType,
    },
};

/// workaround
pub struct Button<const N: usize> {}
impl Satisfied for Button<1> {}
impl Satisfied for Button<2> {}
impl Satisfied for Button<3> {}
impl Satisfied for Button<4> {}
impl Satisfied for Button<5> {}

/// Common interface of Command and Component
#[async_trait]
pub trait CommandExt {
    async fn message<T: ToString + Send + Sync>(
        &self,
        http: impl AsRef<Http> + Send + Sync + 'async_trait,
        content: T,
    ) -> anyhow::Result<()>;
    async fn embed(
        &self,
        http: impl AsRef<Http> + Send + Sync + 'async_trait,
        embed: CreateEmbed,
    ) -> anyhow::Result<()>;
    async fn button<const N: usize>(
        &self,
        http: impl AsRef<Http> + Send + Sync + 'async_trait,
        msg: impl ToString + Send + Sync + 'async_trait,
        buttons: [CreateButton; N],
    ) -> anyhow::Result<()>
    where
        Button<N>: Satisfied;
}

#[async_trait]
impl CommandExt for ApplicationCommandInteraction {
    async fn message<T: ToString + Send + Sync>(
        &self,
        http: impl AsRef<Http> + Send + Sync + 'async_trait,
        content: T,
    ) -> anyhow::Result<()> {
        self.create_interaction_response(&http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| message.content(content))
        })
        .await
        .with_context(|| anyhow!("serenity error"))
    }

    async fn embed(
        &self,
        http: impl AsRef<Http> + Send + Sync + 'async_trait,
        embed: CreateEmbed,
    ) -> anyhow::Result<()> {
        self.create_interaction_response(&http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| message.add_embed(embed))
        })
        .await
        .with_context(|| anyhow!("serenity error"))
    }

    async fn button<const N: usize>(
        &self,
        http: impl AsRef<Http> + Send + Sync + 'async_trait,
        msg: impl ToString + Send + Sync + 'async_trait,
        buttons: [CreateButton; N],
    ) -> anyhow::Result<()>
    where
        Button<N>: Satisfied,
    {
        self.create_interaction_response(&http, |response| {
            response.interaction_response_data(|message| {
                message.content(msg).components(|component| {
                    component.create_action_row(|action_row| {
                        for button in buttons {
                            action_row.add_button(button);
                        }
                        action_row
                    })
                })
            })
        })
        .await
        .with_context(|| anyhow!("serenity error"))
    }
}

#[async_trait]
impl CommandExt for MessageComponentInteraction {
    async fn message<T: ToString + Send + Sync>(
        &self,
        http: impl AsRef<Http> + Send + Sync + 'async_trait,
        content: T,
    ) -> anyhow::Result<()> {
        self.create_interaction_response(&http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| message.content(content))
        })
        .await
        .with_context(|| anyhow!("serenity error"))
    }

    async fn embed(
        &self,
        http: impl AsRef<Http> + Send + Sync + 'async_trait,
        embed: CreateEmbed,
    ) -> anyhow::Result<()> {
        self.create_interaction_response(&http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| message.add_embed(embed))
        })
        .await
        .with_context(|| anyhow!("serenity error"))
    }

    async fn button<const N: usize>(
        &self,
        http: impl AsRef<Http> + Send + Sync + 'async_trait,
        msg: impl ToString + Send + Sync + 'async_trait,
        buttons: [CreateButton; N],
    ) -> anyhow::Result<()>
    where
        Button<N>: Satisfied,
    {
        self.create_interaction_response(&http, |response| {
            response.interaction_response_data(|message| {
                message.content(msg).components(|component| {
                    component.create_action_row(|action_row| {
                        for button in buttons {
                            action_row.add_button(button);
                        }
                        action_row
                    })
                })
            })
        })
        .await
        .with_context(|| anyhow!("serenity error"))
    }
}
