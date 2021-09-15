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

use std::vec::Vec;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum Alphabet {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J
}

/// An abstract syntax tree of a regular expression
/// which denotes a nonempty language over [Alphabet].
///
/// In our problem domain, we do not care about empty languages since setting ∅ as the answer for a quiz
/// is very uninteresting. We therefore restrict ourselves in nonempty regular languages,
/// and the class of regular expressions corresponding to this language class will not require ∅ as a
/// constant symbol. The proof is by a simple induction over set of regular expressions.
#[derive(Debug)]
pub enum RegexAst {
    /// The expression that matches the empty string
    Epsilon,
    /// An expression that matches an alphabetic literal
    Literal(Alphabet),
    /// An expression that matches a repetition of words matching inner expression
    Star(Box<RegexAst>),
    /// An expression that matches if all expressions match successively
    Concatenation(Vec<RegexAst>),
    /// An expression that matches if one of expressions matches
    Alternation(Vec<RegexAst>)
}

pub fn from_raw_string(_string: &str) -> anyhow::Result<RegexAst> {
    todo!()
}

impl RegexAst {
    pub fn matches(_input: &[Alphabet]) -> bool {
        todo!()
    }

    pub fn equivalent_to(_another_ast: &RegexAst) -> bool {
        todo!()
    }
}

impl Display for RegexAst {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}
