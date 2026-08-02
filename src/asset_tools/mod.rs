//! Headless asset browsing for automated callers.
//!
//! The `mix-browser` binary is an eframe GUI: a person clicks through archives
//! and looks at sprites. Everything underneath it is already CPU-only and
//! `pub`, but the useful half is trapped in bin-local modules. This module is
//! that half, in the library, behind a machine-readable interface — see the
//! `asset` binary in `src/bin/asset.rs`.
//!
//! Read-only by design. Nothing here writes into an archive, re-encodes an
//! asset, or emits a golden: what it reports about retail *file contents*
//! (dimensions, frame geometry, string values) is read straight from retail
//! bytes, but a render it produces is not parity evidence.
//!
//! ## Dependency rules
//! - May depend on `assets/`, `rules/`, `util/`, and the CPU-only parts of
//!   `render/` (bitmap font glyph data).
//! - Must NEVER depend on `sim/`, `ui/`, `sidebar/`, `audio/`, `net/`, or any
//!   `app*` module — and nothing in `sim/` may depend on this.

pub mod args;
pub mod canvas;
pub mod identify;
pub mod locate;
pub mod names;
pub mod palette;
pub mod report;
pub mod root;
pub mod verb_find;
pub mod verb_info;
pub mod verb_ls;
pub mod verb_palette;
pub mod verb_render;
