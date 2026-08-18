# Bridge Body Y Offset - Ghidra Verification

**Date:** 2026-05-15
**Scope:** High bridge body positioning: whether bridge body Y offset is `-16/-31` by state range, and how that maps to current Rust `Axis`.
**Primary addresses:** `0x00480110` (`CellClass__Get_Draw_Offset`), `CellClass__DrawOverlay_Body`, `0x005FDCC0` overlay-type base offset helper.
**Confidence:** HIGH for the binary offset formula. Rust-axis interpretation was visually confirmed on 2026-05-16 by live runtime logging plus in-game inspection.
**Active in YR:** Yes. This is the normal per-frame bridge overlay draw path.

## Finding

`gamemd.exe` applies bridge body Y offset by **`cell+0x11E` damage-state byte range**, not by SHP frame range:

- HasBridge clear: no bridge-specific body offset.
- HasBridge set and state byte `0..=8`: base overlay Y offset minus `0x10` = `-16`.
- HasBridge set and state byte `9..=17`: base overlay Y offset minus `0x10` minus `0x0F` = `-31`.

Pre-2026-05-16 Rust constants were therefore swapped relative to Rust's own state-byte encoding:

- Rust `Axis::NS` encodes to state bytes `0..=8`, so binary-compatible body Y offset is `-16`.
- Rust `Axis::EW` encodes to state bytes `9..=17`, so binary-compatible body Y offset is `-31`.

Pre-fix Rust used:

- `Axis::NS => -31`
- `Axis::EW => -16`

That matched the WAE-style equalized visual workaround described in older notes, not the binary state-byte offset rule. This was corrected on 2026-05-16.

## Binary Evidence

### `CellClass__Get_Draw_Offset @ 0x00480110`

Relevant decompile:

```c
iVar2 = base_offset_y;
if ((cell->flags & 0x80) == 0) {
    if (cell->overlay != 0xef) goto done;
} else {
    iVar2 = iVar2 + -0x10;
    if ((cell->bridge_damage_state < 9) || (0x11 < cell->bridge_damage_state)) goto done;
}
iVar2 = iVar2 + -0xf;
done:
out.y = iVar2 + viewport_y + (char)cell->Level * -0xf + 0xf;
```

Details:

- `cell+0x140 & 0x80` is the HasBridge flag.
- `cell+0x11E` is the bridge damage-state byte.
- `cell+0x11B` is signed level and contributes the normal height term.
- The extra `-0x0F` is only reached for HasBridge states `9..=17`.
- There is no SHP frame lookup or axis-name lookup inside this function.

### `CellClass__DrawOverlay_Body`

Relevant decompile details:

- Calls `CellClass__Get_Draw_Offset`.
- Computes local draw position from `draw_offset + screen_xy - clip_origin`.
- For HasBridge cells:
  - reads `uVar7 = *(byte *)(cell + 0x11E)`;
  - if `uVar7 == 0 || uVar7 == 9`, applies `g_OverlayVarietyLatinSquare[((y & 3) << 2) | (x & 3)]`;
  - calls `CC_Draw_Shape(shape, uVar7, &local_pos, clip, 0x4e00, ..., effective_height * -15 - 2, ..., cell+0x10E, ...)`.

This confirms that `Get_Draw_Offset` handles the body Y offset before the SHP draw call, while frame selection separately uses the state byte.

### `FUN_005FDCC0`

This helper supplies the generic overlay-type base offset. It does not contain bridge-specific axis logic. For bridge overlay types, the bridge-specific offset comes from `CellClass__Get_Draw_Offset`'s HasBridge branch.

## Rust Status

Current code after the 2026-05-16 correction:

- `src/sim/bridge_state/mod.rs` documents and implements:
  - `Axis::NS -> state bytes 0..=8`
  - `Axis::EW -> state bytes 9..=17`
- `src/app_instances/bridges.rs` sets offsets by state-byte range:
  - state bytes `0..=8` -> `-16.0`
  - state bytes `9..=17` -> `-31.0`
- `compute_bridge_body_shp_frame` previously mapped:
  - `Axis::EW -> SHP frames 0..=8`
  - `Axis::NS -> SHP frames 9..=17`

**Correction 2026-05-16:** that SHP frame mapping was wrong for Rust's
runtime `Axis` labels. A live runtime check on a visible BRIDGE2 case showed
`axis=EW state_byte=9` while the renderer selected frame-family `0..3`,
producing the observed 90-degree visual rotation. Body frame selection, body
Y offset, and shadow offset should all follow the same state-byte family:
`Axis::NS -> 0..=8`, `Axis::EW -> 9..=17`. Raw asset-frame physical labels
are still useful for visual inspection, but they must not override
`DamageState::to_state_byte(axis)`.

## Doc Conflict Resolution

`BRIDGE_DISPLAY_TABLE_GHIDRA_REPORT.md` is correct where it says:

- state bytes `0..=8` use `-16`;
- state bytes `9..=17` use `-31`.

The later WAE/gamemd comparison text in `BRIDGE_RENDERING_GHIDRA_REPORT.md` that recommends `-16/-16` or frames this as a WAE-only compensation is stale/misleading. The live decompile shows `gamemd.exe` does apply `-31` for state bytes `9..=17`.

The implementation-specific issue was not `-16/-16`; it was that old Rust applied `-31/-16` to the opposite `Axis` labels relative to its own state-byte encoding. Current Rust follows the state byte directly.

## Sources

- Ghidra decompile: `CellClass__Get_Draw_Offset @ 0x00480110`
- Ghidra decompile: `CellClass__DrawOverlay_Body`
- Ghidra decompile: `FUN_005FDCC0 @ 0x005FDCC0`
- Rust scan: `src/sim/bridge_state/mod.rs`
- Rust scan: `src/app_instances/bridges.rs`
