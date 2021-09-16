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

use super::RegexAst;
use crate::regex::Alphabet;
use rand::{distributions::Slice, Rng};
use rand_distr::{Binomial, Distribution};
use std::{num::NonZeroU8};
use strum::IntoEnumIterator;

pub struct Difficulty(pub NonZeroU8);

pub struct AlphabetSet(pub Vec<Alphabet>);

struct WordDistribution<L, A>(L, A);
impl<'a, L: Distribution<usize>, A: Distribution<&'a Alphabet>> Distribution<Vec<Alphabet>>
    for WordDistribution<L, A>
{
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Vec<Alphabet> {
        let length = self.0.sample(rng);

        std::iter::repeat_with(|| *self.1.sample(rng))
            .take(length)
            .collect()
    }
}

fn alphabet_distribution(alphabets: &AlphabetSet) -> impl Distribution<&Alphabet> {
    Slice::new(&alphabets.0).unwrap()
}

fn word_distribution(alphabets: &AlphabetSet) -> impl Distribution<Vec<Alphabet>> + '_ {
    let length_distribution = Binomial::new(15, 0.3).unwrap().map(|n| n as usize);

    WordDistribution(length_distribution, alphabet_distribution(alphabets))
}

#[allow(dead_code)]
fn estimate_acceptance_probability(alphabets: &AlphabetSet, regex_ast: &RegexAst) -> f64 {
    let compiled_ast = regex_ast.compile_to_string_regex();

    let thread_rng = rand::thread_rng();

    let sample_size = 1000;
    let matched = word_distribution(alphabets)
        .sample_iter(thread_rng)
        .take(sample_size)
        .filter(|w| compiled_ast.is_match(Alphabet::slice_to_plain_string(w).as_str()))
        .count();

    (matched as f64) / (sample_size as f64)
}

fn alphabets_used_with(diff: &Difficulty) -> AlphabetSet {
    AlphabetSet(Alphabet::iter().take(diff.0.get().into()).collect())
}

pub fn randomly_generate(diff: &Difficulty) -> RegexAst {
    let _alphabets = alphabets_used_with(diff);

    todo!()
}
