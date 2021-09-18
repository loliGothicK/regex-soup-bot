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

use crate::bot::Quiz;
use anyhow::{anyhow, bail, Context};
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
        Err(_) => {
            bail!("Time out while generating regex (size = {}).", difficulty);
        }
    }
}
