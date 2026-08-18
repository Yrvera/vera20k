\
# `FUN_004BF650` — ore/gem intensity helper used by the terrain-preview pixel pass

Scope: `FUN_004BF650` itself, the state it reads, and the immediate call-site context in
`GenerateTerrainPreview @ 0x00641140` needed to characterize its inputs/outputs. Does NOT
re-derive `GenerateTerrainPreview`'s projection/bounds/markers (already plate-commented) or
`CellClass__GetRadarPixelColor` / `IsometricTileTypeClass__GetRadarColorPair` (already
plate-commented). Does not investigate the general overlay/OverlayTypeClass colour-selection
system beyond what is needed to name `FUN_004BF650`'s two arguments.

**Target question**: what does `FUN_004BF650` compute from its intensity argument, what
state does it read, is it RNG-consuming, what shape is its return value, and is the
`CellClass+0x11E` threshold read signed or unsigned.

**Non-goals**: the rest of `GenerateTerrainPreview` (bounds/projection/markers), the dialog,
the RMG generator, `OverlayTypeClass`'s general colour-selection logic, `OverlayClass__GetRadarColor`
internals.

**Evidence needed to mark COMPLETE**: `decompile_function` + `disassemble_function` of
`0x004BF650`; `get_assembly_context`/disassembly at the call site proving the `+0x11E`
comparison's signedness; `get_function_callees` of `0x004BF650` to settle the RNG question;
`get_xrefs_to` on the two globals it reads to characterize their nature.

**Stop conditions**: once the formula, state reads, return shape, determinism, and
signedness are each pinned with a citation, stop — do not expand into the OverlayTypeClass
colour-selection branch (`+0x2a9`, `+0x2b6/0x2b7/0x2b8`, `OverlayClass__GetRadarColor`) beyond
naming it as the source of `FUN_004BF650`'s colour arguments.

## Verified facts

**1. `FUN_004BF650` is a pure, argument-only arithmetic function with ZERO callees** — it
cannot consume RNG (verified via `get_function_callees 0x004BF650` → "No callees found").
`get_function_callers 0x004BF650` shows exactly three call sites, all inside
`GenerateTerrainPreview` (`0x006414e4`, `0x0064166e`, `0x0064167f`) — no other caller in the
binary. Active in YR: Yes (this is the only reachable caller; the function has no TS-only
gate).

**2. Full decompiled body** (verified via `decompile_function 0x004BF650` and
`disassemble_function 0x004BF650`), `__fastcall` with `param_1`=ECX, `param_2`=EDX,
`param_3`=intensity (stack arg):

```
switch(param_3) {
  case 0x00: return param_2 & 0xffff;                                        // never used by this caller
  case 0x19: return ((param_1 & 0xffff) >> 2 & WORD[0x008a0dea]) << 2;        // param_2 UNUSED
  case 0x32: return ((param_1 & 0xffff) >> 1 & (DWORD[0x008a0de8] & 0xffff))
                   + ((param_2 & 0xffff) >> 1 & (DWORD[0x008a0de8] & 0xffff)); // BOTH used
  case 0x4b: return ((param_1 & 0xffff) >> 2 & WORD[0x008a0dea]) << 2;        // param_2 UNUSED
  default:   return param_1;                                                  // unreachable here
}
```

Cases `0x19` and `0x4b` are **byte-identical instruction sequences** (verified via
`disassemble_function 0x004BF650`: `0x004bf66b`–`0x004bf680` vs. `0x004bf6a8`–`0x004bf6bd`
— same `MOV r16,word ptr [0x008a0dea]` / `AND EAX,0xffff` / `SHR EAX,2` / `AND EAX,EDX` /
`SHL EAX,2` shape, same source address). Case `0x32` is the only branch that reads
`param_2` at all; it is a classic "average two packed 16-bit colours without channel
bleed" idiom: mask each operand's low bit(s) via the shared mask before adding, so the
add cannot carry between RGB channels.

**3. The two globals it reads overlap and are DirectDraw-surface-derived, not RNG,
not gameplay state, not a palette table** — `0x008a0dea` (WORD, used by cases `0x19`/`0x4b`)
is the **high 16 bits** of the same 4-byte region as `0x008a0de8` (DWORD, used masked
`&0xffff` — i.e. its **low** 16 bits — by case `0x32`); Ghidra's own decompiler flags this
overlap (`/* WARNING: Globals starting with '_' overlap smaller symbols at the same
address */`). `get_xrefs_to 0x008a0de8` and `get_xrefs_to 0x008a0dea` show the **only
writer of both is `DSurface__Constructor`** (`0x004baac1` writes the DWORD half, `0x004baaff`
writes the WORD half); readers besides `FUN_004BF650` are `ConvertClass__Constructor`,
`FUN_00491100`, and `TMP_TileBlitter` — all pixel-blit/convert helpers. This is a
per-surface pixel-format blend/dim mask pair, freshly set whenever a `DSurface` is
constructed (the preview's own `DSurface__Constructor` call, documented in the
`GenerateTerrainPreview` plate comment, runs earlier in the same function) — not random,
not per-match state. `read_memory 0x008a0de8` on the static image returns all zero
(`00000000`), confirming the value is not a link-time constant but is populated at
runtime by `DSurface__Constructor` — see Unverified section for what this means for
reproducing the exact numeric mask. Active in YR: Yes (runs whenever the preview surface
is built, which is every random-map Generate/accept).

**4. `CellClass+0x11E` is read as an UNSIGNED byte and compared UNSIGNED** — verified via
`get_assembly_context` at the call site (`xref_sources=0x006414e4`, 30 instructions of
context): `0x006414b9 MOV AL,byte ptr [EBP+0x11e]` (plain byte load, no
`MOVZX`/`MOVSX` — the raw byte only ever feeds 8-bit `CMP`s) then `0x006414c4 CMP AL,0x2`
/ `0x006414cb JNC 0x006414d4` and `0x006414d4 CMP AL,0x5` / `0x006414d6 JNC 0x006414dd`.
`JNC`/`JC` are the **carry-flag (unsigned "above-or-equal"/"below")** conditional jumps,
not `JGE`/`JL` (sign-flag-based) — this settles signedness independent of how the byte was
loaded. `EBP` at this point is the `CellClass` pointer returned by
`MapClass__Get_CellClass` (matches the decompile's `*(byte *)(iVar14 + 0x11e)`, `iVar14`
being that same CellClass pointer, per the already-verified `GenerateTerrainPreview` plate
comment). The identical `CellClass+0x11E` byte is *also* passed as the second argument to
`OverlayClass__GetRadarColor(&iStack_78, *(undefined1*)(iVar14+0x11e))` in the sibling
overlay-colour code visible in the same decompile — consistent with it being the
overlay/ore-gem growth-stage byte (0–11 in gamemd's ore/gem staging), not an
overlay-type-specific field. Active in YR: Yes (ore/gem growth staging is a live default
YR mechanic; not a TS-only flag).

**5. Intensity selection and the "call twice" mechanics are gated on the OverlayType's
`+0x2a9` byte** — the ONLY branch in `GenerateTerrainPreview` that computes an
intensity from `+0x11E` and calls `FUN_004BF650` is inside
`else { iVar17 = OverlayTypeClass_Array[cell->+0x44]; ...; if (*(char*)(iVar17+0x2a9) != 0) { ... iVar16 = intensity(cell->+0x11e); uVar7 = FUN_004bf650(iVar16); ... } }`
(verified via `decompile_function 0x00641140`) — i.e. only overlay types with this byte
set (which, given the intensity/growth-stage machinery attached to it, is the
ore/gem-tinting flag on `OverlayTypeClass`) take the density-scaled path; the sibling
`*(char*)(iVar17+0x2a9) == 0` branch never sets a non-`100` `iVar16` and never calls
`FUN_004BF650`. **The exact struct-field name/semantics of `+0x2a9` beyond "gates this
code path" were not independently re-derived — out of scope per task boundaries; see
Remaining Uncertainty.**

**6. Return shape — one 16-bit-ish value per call; the "pair" is built by the CALLER
calling twice with different arguments, not by one call returning two values** —
verified via `get_assembly_context` on all three call sites
(`xref_sources=0x006414e4,0x0064166e,0x0064167f`): each call site does
`CALL 0x004bf650` → `AND EAX,0xffff` immediately after, i.e. every call returns exactly
one masked 16-bit scalar in EAX. For an ore/gem cell the sequence is:
  - Call #1 (`0x006414e4`, ECX=`ESI`, EDX=`[ESP+0x18]`, param3=intensity) — its masked
    result is stored, then split into `EBP = result & 0xffff` and `EDI = result >> 0x10`
    (`= 0`, since the value was already masked). This result is **not the final pixel
    pair** — it only survives as an input to call #2.
  - Because `EBX` (the intensity, `0x19`/`0x32`/`0x4b`) is never `0x64` (100) on this
    path, the shared tail unconditionally takes the "redo" branch at `0x00641669`:
    - Call #2 (`0x0064166e`, ECX=`ESI`, EDX=`EBP`=call #1's masked result, param3=intensity)
      → becomes `EBP` (left pixel).
    - Call #3 (`0x0064167f`, ECX=`ESI`, EDX=`EDI`=0, param3=intensity) → becomes `EDI`
      (right pixel).
  `ESI` is one of the two OverlayTypeClass-derived RGB565 colours already computed earlier
  in the same branch (the fallback colour at `OverlayTypeClass+0x2b6/0x2b7/0x2b8` or the
  `OverlayClass__GetRadarColor` result — which of the two was not independently
  re-derived, out of scope; see Remaining Uncertainty). Active in YR: Yes.

**7. Left/right pixel relationship depends on the intensity bucket** — because cases
`0x19`/`0x4b` ignore `param_2` entirely (fact 2) and both call #2/#3 pass the *same*
`ECX=ESI`, **for low-density (`<2`) and high-density (`>=5`) ore/gem cells the left and
right preview pixels are byte-identical** (both `= ((ESI>>2) & WORD[0x008a0dea]) << 2`).
**For mid-density (`2..5`) cells the two pixels differ**: left
`= (ESI>>1 & M) + (EBP_call1>>1 & M)` (an average of `ESI` with call #1's discarded
result), right `= (ESI>>1 & M) + 0` (`ESI` simply halved/dimmed) — verified via the
`0x0064166a MOV EDX,EBP` vs. `0x00641679 MOV EDX,EDI` (`EDI==0` at that point, from the
`0x00641502 SHR EDI,0x10` zeroing) instructions at the two "redo" call sites. Active in
YR: Yes.

## Implementation Handoff

- **Verified behavior** → `FUN_004BF650`'s per-bucket formula (fact 2) operates on RGB565-style
  packed 16-bit colours using DirectDraw-surface-derived masks (fact 3), not a lookup table
  and not RNG. **Rust delta** → `src/map/rmg/preview.rs` currently has no ore/gem branch at
  all (`PreviewCell { left, right }` is filled purely from `CellClass__GetRadarPixelColor`'s
  plain tile pair per the parent's summary); it needs an ore/gem branch that (a) reads the
  cell's overlay growth/data byte (the field feeding `CellClass+0x11E` — locate its Rust
  equivalent in `resolved_terrain.rs` / cell overlay data, do not assume the name), (b)
  buckets it unsigned `<2` / `<5` / else, (c) computes the fallback overlay colour (the
  `ESI` input — this requires wiring in whatever Rust already resolves as the
  OverlayTypeClass minimap/fallback colour, out of this report's scope to name), and
  (d) applies the two-call formula per bucket (identical pixel pair for buckets 1/3,
  averaged-vs-halved pair for bucket 2) instead of feeding the plain terrain radar pair.
  **Affected surface** → `src/map/rmg/preview.rs::render_preview` (ore/gem cell branch),
  `src/map/resolved_terrain.rs` (needs the overlay data byte available per-cell, not just
  `radar_left`/`radar_right`). **Acceptance scenario** → for a synthetic cell with a
  Tiberium/Gem-flagged overlay type and growth-data byte `0` (bucket `<2`), the emitted
  left and right preview pixels are identical and equal the masked/shifted fallback colour;
  for growth-data byte `10` (bucket `>=5`), same identical-pair property holds with the
  same formula; for growth-data byte `3` (bucket `2..5`), left and right differ per the
  average/halve relationship in fact 7. **Proposed test name** →
  `test_preview_ore_low_and_high_density_pixels_are_identical`,
  `test_preview_ore_mid_density_pixels_differ_via_average_and_half`. **Risk** → the exact
  numeric value of the two DirectDraw blend masks (`0x008a0de8`/`0x008a0dea`) cannot be
  read from the static binary (fact 3) — a literal byte-for-byte port needs either an
  emulation/live-capture of `DSurface__Constructor`'s mask derivation or an equivalent
  mask computed from the Rust preview's own pixel format (same RGB565 "no-channel-bleed
  average" idiom); a wrong mask constant would silently produce close-but-not-exact colours,
  which the parity bar (CLAUDE.md) treats as DRIFT, not acceptable "close enough".

- **Verified behavior** → the intensity-scaled path is reachable ONLY when
  `OverlayTypeClass+0x2a9` is set (fact 5); non-flagged overlay/terrain cells never call
  `FUN_004BF650` at all and just get the two OverlayTypeClass-derived colours directly
  (identical to each other, per the shared-tail `EBX==0x64` fallback at `0x00641516`/
  `0x00641518`, both set from `ESI`). **Rust delta** → the Rust port must gate its new
  ore/gem branch on the equivalent "is this overlay type ore/gem-tinted" flag, not on
  overlay-type-ID membership in some ad-hoc ore/gem list — whatever Rust surface already
  tracks per-`OverlayType` INI flags is the right place to look (out of this report's scope
  to name; do not invent a name). **Affected surface** → wherever Rust resolves
  `OverlayTypeClass`/`OverlayType=` data for the preview. **Acceptance scenario** → a
  non-ore/gem overlay (e.g. a wall or crate overlay type) on the preview never triggers the
  new intensity branch and renders with an identical left/right pixel pair from the plain
  overlay colour, exactly as today. **Proposed test name** →
  `test_preview_non_ore_overlay_bypasses_intensity_helper`. **Risk** → misclassifying which
  overlay types carry the `+0x2a9`-equivalent flag would apply density shading to the wrong
  overlays (e.g. walls) or skip it for real ore/gem — low effort to get right if the
  existing Rust overlay-type INI parse already surfaces the flag, otherwise needs a small
  follow-up investigation (explicitly out of this report's scope).

- **Verified behavior** → `FUN_004BF650` has zero callees and is 100% deterministic given
  its 3 integer inputs plus the two DSurface-derived masks (fact 1); it never advances any
  RNG stream. **Rust delta** → the port must NOT add any RNG draw when computing ore/gem
  preview pixels — any implementation that calls into a random source for this path
  introduces a determinism/lockstep-irrelevant-but-still-wrong divergence from gamemd
  (this path only runs in the single-player preview renderer, not in sim, but per
  CLAUDE.md's "no disparity too small" rule it still must not draw from any shared RNG
  instance). **Affected surface** → `src/map/rmg/preview.rs`. **Acceptance scenario** →
  calling the ore/gem pixel computation twice with the same inputs (same overlay type,
  same growth byte, same base colour) yields byte-identical output both times, and doing so
  does not perturb any RNG state Rust may have in scope. **Proposed test name** →
  `test_preview_ore_gem_pixel_computation_is_pure_and_deterministic`. **Risk** → none
  identified; this is the most mechanically simple of the three deltas.

## Negative Facts / Do Not Do

- Do NOT treat cases `0x19` and `0x4b` as reachable-but-different — they are the
  byte-identical instruction sequence (fact 2); do not invent a distinct formula for the
  high-density bucket.
- Do NOT read `param_2` for buckets `0x19`/`0x4b` in a Rust port — the original ignores it
  entirely for those two cases (fact 2); porting a formula that uses both args unconditionally
  for all three buckets would be a fabricated behavior, not a faithful port.
- Do NOT treat `0x008a0de8`/`0x008a0dea` as static named constants with a fixed numeric
  value you can hardcode from this report — the static image is zero; the real value is
  set by `DSurface__Constructor` at runtime (fact 3) and was not captured live this session.
- Do NOT assume `FUN_004BF650` reads a palette or colour table — it reads only the two
  DirectDraw blend-mask globals and its 3 arguments; no `PaletteClass`/palette pointer is
  touched anywhere in its body (verified via `decompile_function 0x004BF650`).
- Do NOT expand the ore/gem-tinting gate assumption to "any overlay with `Tiberium=yes` in
  the port's current INI model" without checking what Rust's existing OverlayType parse
  actually calls this flag — `+0x2a9`'s concrete field name/semantics were not re-derived
  this session (see Remaining Uncertainty).

## Remaining Uncertainty

- The exact identity of `ESI` (which of the two OverlayTypeClass-derived candidate colours —
  the `+0x2b6/0x2b7/0x2b8` fallback triple or the `OverlayClass__GetRadarColor` result — ends
  up as the fastcall `ECX` argument to all three calls) was traced from the call-site
  assembly far enough to fix its role (the shared "base colour" reused across all three
  calls) but its ultimate byte-level source was not independently re-verified beyond the
  `GenerateTerrainPreview` decompile already on file — this sits right at the boundary of the
  "general overlay system," which is explicitly out of scope for this report.
- The concrete numeric value(s) of `0x008a0de8`/`0x008a0dea` at runtime (i.e. for the
  preview's actual pixel format) — confirmed to be `DSurface__Constructor`-derived (fact 3)
  but not captured via a live trace this session; a literal-value port needs either an
  emulation run or a live-capture logger hooked to `DSurface__Constructor`.
- The precise semantic name of `OverlayTypeClass+0x2a9` (the byte gating the intensity path)
  was not re-derived — flagged only as "the gate," per the task's non-goal on the general
  overlay system.

## Unverified (YELLOW)

- Whether `0x008a0de8`/`0x008a0dea`'s runtime value is identical across all supported
  display pixel formats (555 vs. 565) or varies — inferred from the `DD_{R,G,B}{Loss,Shift}`
  sibling globals' well-known role (pixel-format-dependent) but not directly confirmed for
  this specific mask pair.

## Status: COMPLETE

All five numbered UNKNOWNs from the task brief are answered with citations (facts 1–7
above); the one open thread (exact identity of the `ESI` base-colour candidate) is a
scope boundary explicitly excluded by the task ("do NOT investigate ... the general
overlay system") and is recorded under Remaining Uncertainty rather than blocking
completion.
