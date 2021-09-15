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
use itertools::Itertools;

#[derive(Debug, PartialEq, Eq)]
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
#[derive(Debug, PartialEq, Eq)]
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

impl Display for Alphabet {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Alphabet::A => write!(f, "a"),
            Alphabet::B => write!(f, "b"),
            Alphabet::C => write!(f, "c"),
            Alphabet::D => write!(f, "d"),
            Alphabet::E => write!(f, "e"),
            Alphabet::F => write!(f, "f"),
            Alphabet::G => write!(f, "g"),
            Alphabet::H => write!(f, "h"),
            Alphabet::I => write!(f, "i"),
            Alphabet::J => write!(f, "j"),
        }
    }
}

impl Display for RegexAst {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RegexAst::Epsilon => write!(f, "ε"),
            RegexAst::Literal(a) => a.fmt(f),
            RegexAst::Star(ast) => write!(f, "({})*", ast),
            RegexAst::Concatenation(asts) =>
                write!(f, "({})", asts.iter().map(|ast| format!("{}", ast)).join("")),
            RegexAst::Alternation(asts) =>
                write!(f, "({})", asts.iter().map(|ast| format!("{}", ast)).join("|")),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::regex::{RegexAst, Alphabet};

    #[test]
    fn fmt_regex_ast() {
        assert_eq!("(abε)",
                   format!("{}",
                           RegexAst::Concatenation(vec![
                               RegexAst::Literal(Alphabet::A),
                               RegexAst::Literal(Alphabet::B),
                               RegexAst::Epsilon,
                           ])
                   ));

        assert_eq!("(a|b|ε)",
                   format!("{}",
                           RegexAst::Alternation(vec![
                               RegexAst::Literal(Alphabet::A),
                               RegexAst::Literal(Alphabet::B),
                               RegexAst::Epsilon,
                           ])
                   ));

        assert_eq!("((a|g))*",
                   format!("{}",
                           RegexAst::Star(Box::new(
                               RegexAst::Alternation(vec![
                                   RegexAst::Literal(Alphabet::A),
                                   RegexAst::Literal(Alphabet::G),
                               ])
                           ))
                   ));

        assert_eq!("((a|(bc)))*",
                   format!("{}",
                           RegexAst::Star(Box::new(
                               RegexAst::Alternation(vec![
                                   RegexAst::Literal(Alphabet::A),
                                   RegexAst::Concatenation(vec![
                                       RegexAst::Literal(Alphabet::B),
                                       RegexAst::Literal(Alphabet::C),
                                   ]),
                               ])
                           ))
                   ));

        assert_eq!("(((a|c)|(bc)))*",
                   format!("{}",
                           RegexAst::Star(Box::new(
                               RegexAst::Alternation(vec![
                                   RegexAst::Alternation(vec![
                                       RegexAst::Literal(Alphabet::A),
                                       RegexAst::Literal(Alphabet::C)
                                   ]),
                                   RegexAst::Concatenation(vec![
                                       RegexAst::Literal(Alphabet::B),
                                       RegexAst::Literal(Alphabet::C),
                                   ]),
                               ])
                           ))
                   ));
    }
}

