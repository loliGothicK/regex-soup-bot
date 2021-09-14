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

/// marker trait for [Condition]
pub trait Satisfied {}

/// Struct of condition for const generics
///
/// This structure is used to constrain the parameters of const generics.
/// You can use it if you want to accept non-empty arrays or arrays with less than 5 elements as arguments.
///
/// # Usage
///
/// ```
/// #![allow(incomplete_features)]
/// #![feature(const_evaluatable_checked)]
/// #![feature(const_generics)]
/// use mitama_bot::concepts::{Condition, Satisfied};
///
/// fn foo<const Size: usize>(_arr: &[i32; Size])
/// where
///     Condition<{ Size > 0 }>: Satisfied,
/// {
///     // _arr is non-empty array.
/// }
/// ```
pub struct Condition<const B: bool>;
impl Satisfied for Condition<true> {}

pub trait SameAs<T> {}
impl<T> SameAs<T> for T {}
