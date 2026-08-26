---
title: Disparity Scan - GSI-04.01 shared-dummy CellClass target Z
date: 2026-08-25
scope: Retained shared-dummy CellClass target-coordinate Z resolution only
methodology: docs-first discovery, direct Rust verification, selective active-YR verification
---

# Disparity Scan - GSI-04.01 shared-dummy CellClass target Z

## Scope and evidence basis

This scan tests one critic claim: that `dummy_cell_target_coord` uses the
104-lepton object/VXL height domain instead of the active-retail 90-lepton
`CellClass` surface domain. It does not reopen the broader shared-dummy,
projectile, bridge, Railgun, LaserDraw, Sonic Wave, destroyable-cliff, or
TS-legacy mechanisms.

Native evidence was taken from the active-YR-verified reports
`PARTICLE_SPARK_LIVE_COLLISION_INPUTS_GHIDRA_REPORT.md` and
`AAHEATSEEKER2_CELLCLASS_RETARGET_AI_BEHAVIOR_GHIDRA_REPORT.md`. Current Rust
was read directly at committed HEAD
`b157d2d83ef25dcbb38f99465dd1292d1415c1da`.

## Summary

- 1 documented candidate behavior inventoried
- 1 active-YR claim verified
- 0 verified gaps
- 0 doc-derived candidates awaiting verification
- 1 verified match / false positive
- 0 deferred or prerequisite-blocked findings

This report is a dated disparity snapshot, not a parity percentage or
completion certificate.

## Verified gaps

None.

## Doc-derived candidates needing verification

None.

## Deferred / blocked by prerequisites

None.

## Doc errors discovered

None. The 104-lepton values in bridge/object/VXL reports belong to separate
coordinate domains; they do not override the independently verified
`CellClass::ComputeGroundHeightAtCoord @ 0x0047B3A0` 90-lepton surface scalar.

## Appendix - verified matches and false positives

| Preliminary claim | Evidence state | Actual Rust state |
|---|---|---|
| `dummy_cell_target_coord` computes target Z in the 104-lepton domain | **ACTIVE-YR VERIFIED false positive.** `CellClass` target virtual `+0x58 @ 0x00486890` delegates to center-coordinate virtual `+0x48 @ 0x00486840`, whose Z comes from `0x0047B3A0`. Active retail initializes that surface domain to 90 leptons per signed level; level `0xFF`, slope 0 at cell center yields `-89`. | `src/sim/projectile.rs::dummy_cell_target_coord` already calls `util::lepton::cellclass_ground_height_leptons`; `gsi_04_01_dummy_target_reads_live_coord_level_and_slope` literally asserts `ProjectileCoord::new(128, 128, -89)`. `git blame` attributes the fix and test to `9ff21254`. |

The later `c023841b` change preserves the same 90-lepton floor while adding the
live shared-dummy structural-bridge flag delta. No current call to
`cell_kernel::cell_floor_height` remains in this helper or its focused test.

## Ghidra annotation candidates

None.

## Recommendations

Do not edit this mechanism. Preserve the existing 90-lepton helper and literal
negative-level regression fixture. Treat any future 104-lepton replacement as
a regression unless new active-binary evidence changes the proven virtual-call
chain.
