//! Shared front-end shell substrate (Framework B: Win32-native dialog shells).
//!
//! Holds the geometry primitives the three pixel-parity shells (main menu 0xE2,
//! single player 0x100, skirmish 0x102) used to each re-implement. Render-agnostic:
//! depends on nothing above this layer (no sim/render/assets), so it honors the
//! ui/ layering rule. The wider descriptor/layout/controller/modal/slide substrate
//! is roadmap (see docs/plans/2026-05-31-shell-substrate-design.md §5); only geom
//! is shared today.

pub mod geom;
