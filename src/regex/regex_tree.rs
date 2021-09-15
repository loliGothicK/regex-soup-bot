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

use anyhow::anyhow;
use combine::{choice, parser, unexpected_any, value, ParseError, Parser, Stream};
use itertools::Itertools;
use parser::char::{char, letter};
use std::{
    borrow::Borrow,
    fmt::{Display, Formatter},
    vec::Vec,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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
    J,
}

impl Alphabet {
    fn from_char(input: &char) -> anyhow::Result<Alphabet> {
        match input {
            'a' | 'A' => Ok(Alphabet::A),
            'b' | 'B' => Ok(Alphabet::B),
            'c' | 'C' => Ok(Alphabet::C),
            'd' | 'D' => Ok(Alphabet::D),
            'e' | 'E' => Ok(Alphabet::E),
            'f' | 'F' => Ok(Alphabet::F),
            'g' | 'G' => Ok(Alphabet::G),
            'h' | 'H' => Ok(Alphabet::H),
            'i' | 'I' => Ok(Alphabet::I),
            'j' | 'J' => Ok(Alphabet::J),
            _ => Err(anyhow!("Character {input} is not a valid Alphabet")),
        }
    }

    pub fn vec_from_str(string: &str) -> anyhow::Result<Vec<Alphabet>> {
        string.chars().map(|c| Self::from_char(&c)).collect::<anyhow::Result<Vec<_>>>()
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

/// An abstract syntax tree of a regular expression
/// which denotes a nonempty language over [Alphabet].
///
/// In our problem domain, we do not care about empty languages since setting ∅ as the answer for a quiz
/// is very uninteresting. We therefore restrict ourselves in nonempty regular languages,
/// and the class of regular expressions corresponding to this language class will not require ∅ as a
/// constant symbol. The proof is by a simple induction over set of regular expressions.
///
/// In a string representation of this datatype, Epsilon is mapped to a character `ε`
/// and literals are mapped to either upper-case or lower-case of corresponding alphabets
/// (`fmt` method will format literals to lower-cases).
/// Star will be denoted by the postfix operator `*`,
/// alternations will be the infix operator `|` and concatenations will have no symbols.
///
/// The precedence of operators should be:
/// `Star`, `Concatenation` and then `Alternation`
/// in a descending order.
///
/// For example, `ab*|cd` should be equivalent to `(a((b)*))|(cd)`.
#[derive(Clone, Debug, PartialEq, Eq)]
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
    Alternation(Vec<RegexAst>),
}

fn regex_parser_<Input>() -> impl Parser<Input, Output = RegexAst>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    let parse_epsilon = parser::char::string("ε").map(|_s| RegexAst::Epsilon);

    let parse_literal = letter().then(|letter| match Alphabet::from_char(&letter) {
        Ok(a) => value(RegexAst::Literal(a)).left(),
        Err(_) => unexpected_any(letter).message("Unexpected literal").right(),
    });

    let parse_epsilon_literal_or_parens = choice!(
        parse_epsilon,
        parse_literal,
        char('(').with(regex_parser()).skip(char(')'))
    );

    let parse_repetitions = parse_epsilon_literal_or_parens.then(|ast| {
        combine::many::<Vec<_>, _, _>(char('*')).map(move |reps| {
            if !reps.is_empty() {
                RegexAst::Star(Box::new(ast.clone()))
            } else {
                ast.clone()
            }
        })
    });

    let parse_concat = combine::many1::<Vec<_>, _, _>(parse_repetitions).map(|asts| {
        if asts.len() > 1 {
            RegexAst::Concatenation(asts)
        } else {
            asts.first().unwrap().clone()
        }
    });

    combine::sep_by1::<Vec<_>, _, _, _>(parse_concat, char('|')).map(|asts| {
        if asts.len() > 1 {
            RegexAst::Alternation(asts)
        } else {
            asts.first().unwrap().clone()
        }
    })
}

parser! {
    fn regex_parser[Input]()(Input) -> RegexAst
    where [Input: Stream<Token = char>]
    {
        regex_parser_()
    }
}

impl RegexAst {
    pub fn parse_str(string: &str) -> anyhow::Result<RegexAst> {
        let (ast, remaining) = regex_parser().parse(string)?;
        assert!(remaining.is_empty());
        Ok(ast)
    }

    /// Format the AST in a way that there cannot be any ambiguity.
    fn fmt_with_extra_parens(&self) -> String {
        fn join_with_separator(sep: &str, asts: &[RegexAst]) -> String {
            asts.iter().map(|ast| ast.fmt_with_extra_parens()).join(sep)
        }

        match self {
            RegexAst::Epsilon => "ε".to_owned(),
            RegexAst::Literal(a) => format!("{}", a),
            RegexAst::Star(ast) => format!("({})*", (*ast).fmt_with_extra_parens()),
            RegexAst::Concatenation(asts) => format!("({})", join_with_separator("", asts)),
            RegexAst::Alternation(asts) => format!("({})", join_with_separator("|", asts)),
        }
    }

    pub fn matches(&self, input: &[Alphabet]) -> bool {
        let regex = format!("^({})$", self.fmt_with_extra_parens());
        let compiled = regex::Regex::new(&regex).unwrap();
        let input_str = input.iter().map(|a| format!("{}", a)).join("");

        compiled.is_match(&input_str)
    }

    pub fn equivalent_to(&self, _another_ast: &RegexAst) -> bool {
        todo!()
    }
}

impl Display for RegexAst {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // TODO reduce extra parentheses; for example, write (a|b)* instead of ((a|b))*
        match self {
            RegexAst::Epsilon => write!(f, "ε"),
            RegexAst::Literal(a) => a.fmt(f),
            RegexAst::Star(ast) => write!(f, "({})*", ast),
            RegexAst::Concatenation(asts) => write!(
                f,
                "({})",
                asts.iter().map(|ast| format!("{}", ast)).join("")
            ),
            RegexAst::Alternation(asts) => write!(
                f,
                "({})",
                asts.iter().map(|ast| format!("{}", ast)).join("|")
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::regex::{Alphabet, RegexAst};

    #[test]
    fn str_to_alphabets() {
        assert_eq!(
            Alphabet::vec_from_str("ABCJ").unwrap(),
            vec![Alphabet::A, Alphabet::B, Alphabet::C, Alphabet::J]
        );

        assert_eq!(
            Alphabet::vec_from_str("abcj").unwrap(),
            vec![Alphabet::A, Alphabet::B, Alphabet::C, Alphabet::J]
        );

        assert_eq!(
            Alphabet::vec_from_str("abCg").unwrap(),
            vec![Alphabet::A, Alphabet::B, Alphabet::C, Alphabet::G]
        )
    }

    #[test]
    #[should_panic]
    fn str_to_alphabets_panic() {
        Alphabet::vec_from_str("Z").unwrap();
    }

    #[test]
    fn str_to_regex_ast() {
        assert_eq!(
            RegexAst::parse_str("abc").unwrap(),
            RegexAst::Concatenation(vec![
                RegexAst::Literal(Alphabet::A),
                RegexAst::Literal(Alphabet::B),
                RegexAst::Literal(Alphabet::C),
            ])
        );

        assert_eq!(
            RegexAst::parse_str("ab|c").unwrap(),
            RegexAst::Alternation(vec![
                RegexAst::Concatenation(vec![
                    RegexAst::Literal(Alphabet::A),
                    RegexAst::Literal(Alphabet::B)
                ]),
                RegexAst::Literal(Alphabet::C)
            ])
        );

        assert_eq!(
            RegexAst::parse_str("ab*|cd").unwrap(),
            RegexAst::Alternation(vec![
                RegexAst::Concatenation(vec![
                    RegexAst::Literal(Alphabet::A),
                    RegexAst::Star(Box::new(RegexAst::Literal(Alphabet::B))),
                ]),
                RegexAst::Concatenation(vec![
                    RegexAst::Literal(Alphabet::C),
                    RegexAst::Literal(Alphabet::D)
                ])
            ])
        );
    }

    #[test]
    fn fmt_regex_ast() {
        assert_eq!(
            "(abε)",
            format!(
                "{}",
                RegexAst::Concatenation(vec![
                    RegexAst::Literal(Alphabet::A),
                    RegexAst::Literal(Alphabet::B),
                    RegexAst::Epsilon,
                ])
            )
        );

        assert_eq!(
            "(a|b|ε)",
            format!(
                "{}",
                RegexAst::Alternation(vec![
                    RegexAst::Literal(Alphabet::A),
                    RegexAst::Literal(Alphabet::B),
                    RegexAst::Epsilon,
                ])
            )
        );

        assert_eq!(
            "((a|g))*",
            format!(
                "{}",
                RegexAst::Star(Box::new(RegexAst::Alternation(vec![
                    RegexAst::Literal(Alphabet::A),
                    RegexAst::Literal(Alphabet::G),
                ])))
            )
        );

        assert_eq!(
            "((a|(bc)))*",
            format!(
                "{}",
                RegexAst::Star(Box::new(RegexAst::Alternation(vec![
                    RegexAst::Literal(Alphabet::A),
                    RegexAst::Concatenation(vec![
                        RegexAst::Literal(Alphabet::B),
                        RegexAst::Literal(Alphabet::C),
                    ]),
                ])))
            )
        );

        assert_eq!(
            "(((a|c)|(bc)))*",
            format!(
                "{}",
                RegexAst::Star(Box::new(RegexAst::Alternation(vec![
                    RegexAst::Alternation(vec![
                        RegexAst::Literal(Alphabet::A),
                        RegexAst::Literal(Alphabet::C)
                    ]),
                    RegexAst::Concatenation(vec![
                        RegexAst::Literal(Alphabet::B),
                        RegexAst::Literal(Alphabet::C),
                    ]),
                ])))
            )
        );
    }
}
