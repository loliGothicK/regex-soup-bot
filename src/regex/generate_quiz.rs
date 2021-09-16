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
use itertools::Itertools;
use rand::{distributions::Slice, Rng};
use rand_distr::{Binomial, Distribution, Uniform, WeightedIndex};
use std::num::NonZeroU8;
use strum::IntoEnumIterator;

pub struct Difficulty(pub NonZeroU8);

pub struct AlphabetSet(pub Vec<Alphabet>);

// constants related to generation of quizzes
const MAX_QUIZ_TREE_SIZE: u8 = 12;
const MINIMUM_ALLOWED_ACCEPTANCE_RATE: f64 = 0.2;
const MAXIMUM_ALLOWED_ACCEPTANCE_RATE: f64 = 1.0 - MINIMUM_ALLOWED_ACCEPTANCE_RATE;

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

fn good_as_a_quiz_problem(alphabets: &AlphabetSet, ast: &RegexAst) -> bool {
    let estimated_acceptance = estimate_acceptance_probability(alphabets, ast);

    MINIMUM_ALLOWED_ACCEPTANCE_RATE < estimated_acceptance
        && estimated_acceptance < MAXIMUM_ALLOWED_ACCEPTANCE_RATE
}

fn alphabets_used_with(diff: &Difficulty) -> AlphabetSet {
    AlphabetSet(Alphabet::iter().take(diff.0.get().into()).collect())
}

/// A distribution generating a sequence of integers whose sum is no more than [max_sum].
/// The resulting vector is used in determining maximum size of subtrees when generating AST.
///
/// The distribution logic is embedded into the [Distribution] impl of this struct.
struct PartitionTreeSize {
    max_sum: u8,
}
impl Distribution<Vec<u8>> for PartitionTreeSize {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Vec<u8> {
        let target_sum: u64 = self.max_sum.into();

        let partition_count = Binomial::new(target_sum, 0.3).unwrap().sample(rng);

        // randomly place `partition_count` partitions between [0, target_sum]
        let partitions = Uniform::new(0u8, target_sum as u8 + 1)
            .sample_iter(rng)
            .take(partition_count as usize)
            .chain(vec![0u8, target_sum as u8]);

        // sort partitions with 0 and target_sum added
        let sorted_partitions = partitions.sorted().dedup().collect::<Vec<_>>();

        (&sorted_partitions)
            .windows(2)
            .map(|w| w[1] - w[0])
            .collect()
    }
}

/// A distribution generating a sequence of [RegexAst] such that
/// sum of tree sizes is no more than [max_sum].
///
/// The distribution logic is embedded into the [Distribution] impl of this struct.
struct RegexTreeVec<'a> {
    alphabet_set: &'a AlphabetSet,
    /// maximum sum of generated trees' sizes
    maximum_size_sum: u8,
}
impl Distribution<Vec<RegexAst>> for RegexTreeVec<'_> {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Vec<RegexAst> {
        let sizes = PartitionTreeSize {
            max_sum: self.maximum_size_sum,
        }
        .sample(rng);

        sizes
            .into_iter()
            .map(|size| {
                BoundedRegexAstDistribution {
                    alphabet_set: self.alphabet_set,
                    max_tree_size: size,
                }
                .sample(rng)
            })
            .collect()
    }
}
impl <'a> From<BoundedRegexAstDistribution<'a>> for RegexTreeVec<'a> {
    fn from(d: BoundedRegexAstDistribution<'a>) -> RegexTreeVec<'a> {
        RegexTreeVec {
            alphabet_set: d.alphabet_set,
            maximum_size_sum: d.max_tree_size,
        }
    }
}

/// A distribution generating a Regex AST of size no more than [max_tree_size].
/// The distribution logic is embedded into the [Distribution] impl of this struct.
struct BoundedRegexAstDistribution<'a> {
    alphabet_set: &'a AlphabetSet,
    max_tree_size: u8,
}

impl<'a> BoundedRegexAstDistribution<'a> {
    /// Get distribution with the same alphabet set but [max_tree_size] replaced.
    fn tree_size_replaced(&self, new_max_tree_size: u8) -> Self {
        BoundedRegexAstDistribution {
            alphabet_set: self.alphabet_set,
            max_tree_size: new_max_tree_size,
        }
    }

    /// Get distribution with tree size decremented.
    /// Panics if [max_tree_size] is already 0.
    fn tree_size_decremented(&self) -> Self {
        self.tree_size_replaced(self.max_tree_size.checked_sub(1).unwrap())
    }
}

impl<'a> Distribution<RegexAst> for BoundedRegexAstDistribution<'a> {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> RegexAst {
        let alphabets = self.alphabet_set;
        let max_tree_size = self.max_tree_size;

        // weights of cases to choose in AST:
        let case_weights = vec![
            // weight of Epsilon branch
            2,
            // weight of Literal branch
            10,
            // weight of Star branch
            if max_tree_size >= 2 { 6 } else { 0 },
            // weight of Concatenation branch
            if max_tree_size >= 3 { 4 } else { 0 },
            // weight of Alternation branch
            if max_tree_size >= 3 { 4 } else { 0 },
        ];

        let case_index = WeightedIndex::new(case_weights).unwrap().sample(rng);

        match case_index {
            0 => RegexAst::Epsilon,
            1 => RegexAst::Literal(*alphabet_distribution(alphabets).sample(rng)),
            2 => RegexAst::Star(Box::new(self.tree_size_decremented().sample(rng))),
            3 => RegexAst::Concatenation(
                RegexTreeVec::from(self.tree_size_decremented()).sample(rng),
            ),
            4 => {
                RegexAst::Alternation(RegexTreeVec::from(self.tree_size_decremented()).sample(rng))
            }
            _ => unreachable!(),
        }
    }
}

fn generate_ast_smaller_than(alphabets: &AlphabetSet, tree_size: u8) -> RegexAst {
    let mut rng = rand::thread_rng();

    BoundedRegexAstDistribution {
        alphabet_set: alphabets,
        max_tree_size: tree_size,
    }
    .sample(&mut rng)
}

pub fn randomly_generate(diff: &Difficulty) -> RegexAst {
    let alphabets = alphabets_used_with(diff);

    loop {
        let ast = generate_ast_smaller_than(&alphabets, MAX_QUIZ_TREE_SIZE);

        if good_as_a_quiz_problem(&alphabets, &ast) {
            return ast;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::regex::{randomly_generate, Difficulty};
    use std::convert::TryInto;

    #[test]
    fn randomly_generate_returns() {
        println!(
            "{:?}",
            randomly_generate(&Difficulty(3u8.try_into().unwrap()))
        );
    }
}
