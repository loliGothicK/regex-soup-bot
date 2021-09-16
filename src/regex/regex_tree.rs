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

use super::super::nfa::nfa_manipulations::NfaData;
use anyhow::anyhow;
use automata::nfa::Nfa;
use combine::{choice, parser, unexpected_any, value, ParseError, Parser, Stream};
use itertools::Itertools;
use parser::char::{char, letter};
use std::{
    collections::HashSet,
    fmt::{Display, Formatter},
    vec::Vec,
};

#[derive(Copy, Clone, Debug, PartialEq, Hash, Eq, PartialOrd, Ord)]
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
            _ => Err(anyhow!("Character {} is not a valid Alphabet", input)),
        }
    }

    pub fn vec_from_str(string: &str) -> anyhow::Result<Vec<Alphabet>> {
        string
            .chars()
            .map(|c| Self::from_char(&c))
            .collect::<anyhow::Result<Vec<_>>>()
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

// We need to tie the knot using `parser!` macro. See
// https://docs.rs/combine/4.6.1/combine/#examples for details.
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

    /// Compile the current AST to a regular expression that does not use a ε.
    fn compile_to_epsilonless_regex(&self) -> String {
        fn join_with_separator(sep: &str, asts: &[RegexAst]) -> String {
            asts.iter()
                .map(|ast| ast.compile_to_epsilonless_regex())
                .join(sep)
        }

        match self {
            RegexAst::Epsilon => "(.{0})".to_owned(),
            RegexAst::Literal(a) => format!("{}", a),
            RegexAst::Star(ast) => format!("({})*", (*ast).compile_to_epsilonless_regex()),
            RegexAst::Concatenation(asts) => format!("({})", join_with_separator("", asts)),
            RegexAst::Alternation(asts) => format!("({})", join_with_separator("|", asts)),
        }
    }

    pub fn matches(&self, input: &[Alphabet]) -> bool {
        let regex = format!("^({})$", self.compile_to_epsilonless_regex());
        let compiled = regex::Regex::new(&regex).unwrap();
        let input_str = input.iter().map(|a| format!("{}", a)).join("");

        compiled.is_match(&input_str)
    }

    fn compile_to_nfa_data(&self) -> NfaData<Alphabet> {
        match self {
            RegexAst::Epsilon => NfaData::epsilon(),
            RegexAst::Literal(a) => NfaData::literal(*a),
            RegexAst::Star(ast) => NfaData::star(&ast.compile_to_nfa_data()),
            RegexAst::Concatenation(asts) => {
                let compiled_asts = asts
                    .iter()
                    .map(|ast| ast.compile_to_nfa_data())
                    .collect::<Vec<_>>();
                NfaData::concat_all(compiled_asts)
            }
            RegexAst::Alternation(asts) => {
                let compiled_asts = asts
                    .iter()
                    .map(|ast| ast.compile_to_nfa_data())
                    .collect::<Vec<_>>();
                NfaData::union_all(compiled_asts)
            }
        }
    }

    /// Set of alphabets used within this AST.
    fn used_alphabets(&self) -> HashSet<Alphabet> {
        let mut accum = HashSet::new();
        let mut exprs_to_process = vec![self];

        while !exprs_to_process.is_empty() {
            let to_process = exprs_to_process.pop().unwrap();
            match to_process {
                RegexAst::Epsilon => {}
                RegexAst::Literal(a) => {
                    accum.insert(*a);
                }
                RegexAst::Star(ast) => exprs_to_process.push(ast),
                RegexAst::Concatenation(asts) => exprs_to_process.extend(asts),
                RegexAst::Alternation(asts) => exprs_to_process.extend(asts),
            }
        }

        accum
    }

    pub fn equivalent_to(&self, another: &RegexAst) -> bool {
        let nfa_1: Nfa<Alphabet> = self.compile_to_nfa_data().into();
        let nfa_2: Nfa<Alphabet> = another.compile_to_nfa_data().into();

        let alphabet_extension = self.used_alphabets();

        if alphabet_extension != another.used_alphabets() {
            // Proposition: A word containing a letter α is never accepted by RegexAst `r` if
            //              r does not contain α.
            //   Proof: By a straightforward induction on `r`.
            //
            // Proposition: If a RegexAst `r` contains a literal α, then there exists a word
            //              containing α that is accepted by `r`.
            //   Proof: Base case is immediate.
            //          For inductive part, notice that RegexAst always corresponds to a
            //          nonempty language, so by case-wise analysis
            //          we can always construct such a word.
            //
            // Corollary: if two RegexAst have different set of used_alphabets, they are not equivalent.
            return false;
        }

        let dfa_1 = nfa_1.into_dfa(alphabet_extension.clone());
        let dfa_2 = nfa_2.into_dfa(alphabet_extension);

        // Pair two DFAs with the decider function (_ && !_).
        // The decider function will essentially create a DFA that recognizes the intersection of
        // `L(dfa_1)` and `Complement(L(dfa_2))`.
        // Therefore, emptiness test done by `pair_empty` will check that
        // "there is some word recognized by either dfa_1 or dfa_2 but not by the other".
        // So by negating this result we are done.
        !dfa_1.pair_empty(&dfa_2, &|final_in_1, final_in_2| final_in_1 && !final_in_2)
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
    fn regex_ast_matches() {
        let positives = vec![
            ("ab|c", "ab"),
            ("ab|c", "c"),
            ("ε|a", ""),
            ("ε|a", "a"),
            ("a*bεcc*", "bc"),
            ("a*bεcc*", "aabccc"),
            ("ε", ""),
            ("ε*", ""),
        ];
        let negatives = vec![("ε|a", "ab"), ("ε|aaa*", "a"), ("a*bεcc*", "aac")];

        for (regex_str, input_str) in positives {
            let ast = RegexAst::parse_str(regex_str).unwrap();
            let input = Alphabet::vec_from_str(input_str).unwrap();
            assert!(
                ast.matches(&input),
                "The expression \"{}\" should match \"{}\"",
                regex_str,
                input_str
            )
        }

        for (regex_str, input_str) in negatives {
            let ast = RegexAst::parse_str(regex_str).unwrap();
            let input = Alphabet::vec_from_str(input_str).unwrap();
            assert!(
                !ast.matches(&input),
                "The expression \"{}\" should not match \"{}\"",
                regex_str,
                input_str
            )
        }
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

    #[test]
    fn regex_ast_used_alphabets() {
        let pairs = vec![("(agb|c*)g", "abcg"), ("agb|c*g", "abcg")];

        for (regex_str, alphabets_str) in pairs {
            let ast = RegexAst::parse_str(regex_str).unwrap();
            let alphabets = Alphabet::vec_from_str(alphabets_str)
                .unwrap()
                .into_iter()
                .collect();

            assert_eq!(
                ast.used_alphabets(),
                alphabets,
                r#"Alphabets used in "{}" should be "{:?}""#,
                ast,
                alphabets
            )
        }
    }

    #[test]
    fn regex_ast_equivalence() {
        fn compile_to_regex_ast(regex_str: &str) -> RegexAst {
            RegexAst::parse_str(regex_str).unwrap()
        }

        let positives = vec![
            ("abεc", "εabc"),
            ("ε|εεε*", "ε"),
            (
                "(a|b|c)*(a|b)(a|b)(a|b)",
                "((a|b|c)*c(a|b)(a|b)(a|b)+)|((a|b)(a|b)(a|b)+)",
            ),
            ("(a|b)*", "a*(ba*)*"),
        ];
        let negatives = vec![("abεc", "abbc"), ("ε", "a")];

        for (regex_str_1, regex_str_2) in positives {
            let ast_1 = compile_to_regex_ast(regex_str_1);
            let ast_2 = compile_to_regex_ast(regex_str_2);

            assert!(
                ast_1.equivalent_to(&ast_2),
                "The regular expression \"{}\" should be equivalent to \"{}\"",
                ast_1,
                ast_2
            )
        }

        for (regex_str_1, regex_str_2) in negatives {
            let ast_1 = compile_to_regex_ast(regex_str_1);
            let ast_2 = compile_to_regex_ast(regex_str_2);

            assert!(
                !ast_1.equivalent_to(&ast_2),
                "The regular expression \"{}\" should not be equivalent to \"{}\"",
                ast_1,
                ast_2
            )
        }
    }
}
