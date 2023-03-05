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

use crate::regex::Alphabet;
use std::{collections::HashSet, fmt::Debug};
use strum::IntoEnumIterator;
use thiserror::Error;

struct Alphabets();

impl std::fmt::Display for Alphabets {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let set = Alphabet::iter().collect::<HashSet<_>>();
        write!(f, "{:?}", set)
    }
}

#[derive(Debug, Error)]
pub enum CommandError {
    #[error(
        r#"
Invalid inputs: {invalid:?}.
=> Hint: Acceptable character set is {}.
"#,
        Alphabets()
    )]
    InvalidInputs { invalid: Vec<String> },
    #[error(
        r#"
Out of domain: {invalid:?}.
=> Hint: Domain character set is {domain:?}.
"#
    )]
    DomainError {
        invalid: Vec<String>,
        domain: HashSet<Alphabet>,
    },
    #[error("Time Limit Exceeded ({limit})")]
    Timeout { limit: String },
}
