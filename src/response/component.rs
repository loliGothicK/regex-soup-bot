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

use crate::concepts::{Condition, Satisfied};

#[derive(Debug)]
pub enum Component {
    Buttons {
        content: String,
        buttons: Buttons,
    },
    SelectMenu {
        custom_id: String,
        content: String,
        placeholder: String,
        min_value: u64,
        max_value: u64,
        options: Vec<SelectMenuOption>,
    },
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct SelectMenuOption {
    pub description: String,
    pub label: String,
    pub value: String,
}

#[derive(Debug)]
pub struct Buttons {
    buttons: Vec<serenity::builder::CreateButton>,
}

impl Buttons {
    pub fn new<const N: usize>(buttons: &[serenity::builder::CreateButton; N]) -> Buttons
    where
        Condition<{ N <= 5 }>: Satisfied,
    {
        Buttons {
            buttons: buttons.to_vec(),
        }
    }
}

impl IntoIterator for Buttons {
    type IntoIter = std::vec::IntoIter<Self::Item>;
    type Item = serenity::builder::CreateButton;

    fn into_iter(self) -> Self::IntoIter {
        self.buttons.into_iter()
    }
}
