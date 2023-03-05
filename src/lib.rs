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

// Yeah!
#![allow(incomplete_features)]
#![feature(
    adt_const_params,
    const_generics_defaults,
    const_trait_impl,
    format_args_capture,
    generic_const_exprs,
    in_band_lifetimes
)]
// Generic Const-Expression:
// <const N: {Integer}>: where Foo<{N + 1}>
//  ~~~~~~~~~~~~~~~~~~             ~~~~~~~
//  const generics (stable)        generic_const_exprs

// Format Arguments Capture:
// let foo = ...;
// println!("{foo}");
//          ~~~~~~~ format args capture (seem to be C#)

pub mod bot;
pub mod command_ext;
pub mod commands;
pub mod concepts;
pub mod errors;
pub mod notification;
pub mod parser;
pub mod regex;
pub mod response;
