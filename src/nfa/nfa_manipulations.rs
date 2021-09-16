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
    /// Maximum index present in the NFA.
    /// Notice that NFA is nonempty, so this is always greater than or equal to 0.
    ///
    /// The datatype invariant that should be maintained is that there is no index strictly
    /// greater than [max_index] in [edges] or [finals].
    max_index: usize,

    /// Vector specifying edges in NFA. An edge is a triple of start index, label and target index,
    /// where label is [None] for epsilon-transitions and [Some] for transitions with letter.
    edges: Vec<(usize, Option<A>, usize)>,

    /// Indices of accepting states.
    finals: Vec<usize>,
}

impl<A: Alphabet> NfaData<A> {
    /// The NFA that accepts no word.
    pub fn empty() -> NfaData<A> {
        NfaData {
            max_index: 0,
            edges: vec![],
            finals: vec![],
        }
    }

    /// The NFA that only accepts the empty word.
    pub fn epsilon() -> NfaData<A> {
        NfaData {
            max_index: 0,
            edges: vec![],
            finals: vec![0],
        }
    }

    /// An NFA that only accepts the single-letter word containing the given alphabet.
    pub fn literal(alphabet: A) -> NfaData<A> {
        NfaData {
            max_index: 1,
            edges: vec![(0, Some(alphabet), 1)],
            finals: vec![1],
        }
    }

    /// Obtain a new NFA that accepts no words,
    /// but with all indices of nodes increased by [shift_size].
    ///
    /// For example, the NFA
    ///
    /// ```text
    ///  -> 0 --ε->[1]--A->[2]
    ///             |       ^
    ///             |       B
    ///             |       |
    ///             |---B-> 3
    /// ```
    ///
    /// will be, after operation `.disconnect_with_shift(3)`, will be
    ///
    /// ```text
    ///     3 --ε->[4]--A->[5]
    ///             |       ^
    ///             |       B
    ///             |       |
    ///             |---B-> 6
    ///
    ///  -> 0   1   2
    /// ```
    ///
    /// Note that nodes 0, 1, 2 will not appear anywhere in the data structure,
    /// so they can be considered to be nonexistent.
    fn disconnect_with_shift(&self, shift_size: usize) -> NfaData<A> {
        NfaData {
            max_index: self.max_index + shift_size,
            edges: self
                .edges
                .iter()
                .map(|(from, label, to)| (from + shift_size, *label, to + shift_size))
                .collect(),
            finals: self.finals.iter().map(|idx| idx + shift_size).collect(),
        }
    }

    /// An NFA that recognizes `L(left)^L(right)` where `^` is the concatenation of languages.
    pub fn concat(&self, right: &NfaData<A>) -> NfaData<A> {
        // We will join all accepting states of self
        // to the start state of another automaton using epsilon-transition.
        // See https://www.cs.odu.edu/~toida/nerzic/390teched/regular/fa/kleene-1.html for details.

        let shifted_right_start_index = self.max_index + 1;
        let shifted_right = right.disconnect_with_shift(shifted_right_start_index);

        let combined_transitions = self
            .edges
            .iter()
            .cloned()
            .chain(shifted_right.edges.iter().cloned())
            .chain(
                self.finals
                    .iter()
                    .map(|self_final| (*self_final, None, shifted_right_start_index)),
            )
            .collect();

        NfaData {
            max_index: shifted_right.max_index,
            edges: combined_transitions,
            finals: shifted_right.finals,
        }
    }

    /// An NFA that accepts concatenations of words each of which is recognized by NFAs in the slice.
    pub fn concat_all(nfas: Vec<NfaData<A>>) -> NfaData<A> {
        assert!(
            !nfas.is_empty(),
            "argument for concat_all must be nonempty slice"
        );

        // we can only sequentially concatenate NFAs
        nfas.into_iter()
            .fold1(|nfa1, nfa2| nfa1.concat(&nfa2))
            .unwrap()
    }

    /// An NFA that recognizes the union of languages of NFAs in the given slice.
    pub fn union_all(nfas: Vec<NfaData<A>>) -> NfaData<A> {
        assert!(
            !nfas.is_empty(),
            "argument for union_all must be nonempty slice"
        );

        if nfas.len() == 1 {
            return nfas.first().unwrap().clone();
        }

        // We will essentially start from a new start node, add all transitions from [nfas] and then
        // add epsilon-transitions from the start node to all the start nodes of [nfas].
        //
        // See https://www.cs.odu.edu/~toida/nerzic/390teched/regular/fa/kleene-1.html for the case
        // where `nfas.len() == 2`.

        nfas.iter().fold(Self::empty(), |accum, nfa| {
            let shifted_right_start_index = accum.max_index + 1;
            let shifted_nfa = nfa.disconnect_with_shift(shifted_right_start_index);

            let extended_edges = accum
                .edges
                .iter()
                .cloned()
                .chain(shifted_nfa.edges.iter().cloned())
                .chain(std::iter::once((0, None, shifted_right_start_index)))
                .collect();

            let extended_finals = accum
                .finals
                .iter()
                .cloned()
                .chain(shifted_nfa.finals.iter().cloned())
                .collect();

            NfaData {
                max_index: shifted_nfa.max_index,
                edges: extended_edges,
                finals: extended_finals,
            }
        })
    }

    /// An NFA that recognizes zero or more repetition of words recognized by the given NFA.
    pub fn star(nfa: &NfaData<A>) -> NfaData<A> {
        // See https://www.cs.odu.edu/~toida/nerzic/390teched/regular/fa/kleene-1.html for details.
        let shifted = nfa.disconnect_with_shift(1);
        let extended_edges = shifted
            .edges
            .iter()
            .cloned()
            .chain(std::iter::once((0, None, 1)))
            .chain(shifted.finals.iter().map(|idx| (*idx, None, 0)))
            .collect();

        NfaData {
            max_index: shifted.max_index,
            edges: extended_edges,
            finals: vec![0],
        }
    }
}

impl<A: Alphabet> From<NfaData<A>> for Nfa<A> {
    fn from(nfa_data: NfaData<A>) -> Self {
        Nfa::from_edges(nfa_data.edges, nfa_data.finals)
    }
}
