// SPDX-License-Identifier: MIT
//! Cobblestone first-party plugin API.
//!
//! Public surface for statically linked Rust plugins. Does not load foreign
//! JavaScript plugin bundles. Attribution for referenced MIT type definitions
//! lives in the repository `NOTICE` file.

pub mod compat;

pub use compat::*;
