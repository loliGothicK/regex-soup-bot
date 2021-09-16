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

use automata::{nfa::Nfa, Alphabet};
use itertools::Itertools;

/// A representation of a nondeterministic finite automaton.
/// States have arbitrary numbering by [usize], but the start state is fixed to 0.
///
/// This has different structure compared to [automata::nfa::Nfa], but
/// can be used to construct larger NFAs and then finally convert everything to [automata::nfa::Nfa].
#[derive(Clone, Debug)]
pub struct NfaData<A: Alphabet> {
    /// Vector specifying edges in NFA. An edge is a triple of start index, label and target index,
    /// where label is [None] for epsilon-transitions and [Some] for transitions with letter.
    pub edges: Vec<(usize, Option<A>, usize)>,

    /// Indices of accepting states.
    pub finals: Vec<usize>,
}

impl <A: Alphabet> NfaData<A> {
    /// The NFA that only accepts the empty word.
    pub fn epsilon() -> NfaData<A> {
        NfaData {
            edges: vec![],
            finals: vec![0]
        }
    }

    /// An NFA that only accepts the single-letter word containing the given alphabet.
    pub fn literal(alphabet: A) -> NfaData<A> {
        NfaData {
            edges: vec![(0, Some(alphabet), 1)],
            finals: vec![1]
        }
    }

    /// An NFA that recognizes `L(left)^L(right)` where `^` is the concatenation of languages.
    pub fn concat(&self, _right: &NfaData<A>) -> NfaData<A> {
        todo!()
    }

    /// An NFA that accepts concatenations of words each of which is recognized by NFAs in the slice.
    pub fn concat_all(nfas: Vec<NfaData<A>>) -> NfaData<A> {
        assert!(nfas.len() > 0, "argument for concat_all must be nonempty slice");
        nfas.into_iter().fold1(|nfa1, nfa2| nfa1.concat(&nfa2)).unwrap()
    }

    /// An NFA that recognizes union of languages of NFAs in the given slice.
    pub fn union_all(nfas: Vec<NfaData<A>>) -> NfaData<A> {
        assert!(nfas.len() > 0, "argument for union_all must be nonempty slice");
        todo!()
    }

    /// An NFA that recognizes zero or more repetition of words recognized by the given NFA.
    pub fn star(_nfa: &NfaData<A>) -> NfaData<A> {
        todo!()
    }
}

impl <A: Alphabet> Into<Nfa<A>> for NfaData<A> {
    fn into(self) -> Nfa<A> {
        Nfa::from_edges(self.edges, self.finals)
    }
}
