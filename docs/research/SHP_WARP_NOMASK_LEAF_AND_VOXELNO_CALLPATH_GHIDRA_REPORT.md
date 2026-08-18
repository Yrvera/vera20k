# SHP Warp No-Mask Leaf And Voxel=no Call Path - Ghidra Research Report

**Address(es):** `0x0073B470`, `0x0073B140`, `0x00490B90`, `0x0048EBF0`, `0x004941E0`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** UnitClass `Voxel=no` SHP body path around `0x0073B470` and the standard no-mask 50% selector leaf selected by `Blitter_selector` slot `+0x78`.
**Non-Scope:** Re-proving `TechnoClass_DrawSHP` flag construction, VXL warp math, complete SHP asset frame sequencing, or the extended no-mask `+0x130` leaf except as contrast.
**Confidence:** High for `+0x78` leaf behavior; High for the read-only `0x0073B470 -> vtable+0x55C -> 0x0073B140` path; Medium for naming of every local in `0x0073B470` because Ghidra has no function boundary there.
**Active in YR:** Yes for UnitClass SHP vehicle path and `Blitter_selector`; Conditional for `+0x78` use when flags have `flags&6==4`, `0x800`, and no `0x3000/0x4000/0x8`.

## Working Notes

- Target question: Does a live YR `Voxel=no` techno body reach `TechnoClass_DrawSHP`, and what does the standard no-mask `+0x78` SHP 50% blitter leaf do for source/dest/Z?
- Non-goals: Do not redo `TechnoClass_DrawSHP` flag construction, VXL warp pixel math, or the extended no-mask `+0x130` branch except as contrast.
- Evidence needed to mark COMPLETE: decompile plus caller/xref evidence for the `Voxel=no` call path, and decompile/disassembly evidence for selector `+0x78` plus its leaf behavior.
- Stop conditions: Stop if the function boundary cannot be recovered read-only, if no active YR `Voxel=no` caller can be proven, or if the `+0x78` leaf cannot be tied to SHP warp selector use.

## 1. Overview

The `Voxel=no` UnitClass vehicle body path does not directly call `TechnoClass_DrawSHP`. The UnitClass vtable entry `+0x554` points at `0x0073B470`; that raw function sets up body/turret/locomotor state and later dispatches `vtable+0x55C`, which points at `UnitClass__Draw_Sprite_With_BridgeFudge @ 0x0073B140`. That callee chooses an SHP blitter from flags rooted at `0x2800`, so a warped `Voxel=no` UnitClass body takes the mask-family `+0xA4` path, not the no-mask `+0x78` path.

The standard no-mask `+0x78` leaf is still real and active in the shared selector. `Blitter_selector` returns `+0x78` for the 50% family only when `flags & 6 == 4`, `0x800` is set, `0x3000` is clear, and `0x4000` is clear. `Blitter_init` constructs that slot with `vtable_ZBuf_50pct_blend`, whose scanline method at `0x004941E0` performs post-remap/intensity 16-bit half blending against the destination and does not access `g_ZBuffer`.

## 2. Class / Table Offsets

| Object / table | Offset | Value / callee | Meaning | Active in YR |
|---|---:|---|---|---|
| UnitClass vtable `0x007F5C70` | `+0x554` | `0x0073B470` | UnitClass body draw wrapper with no Ghidra boundary | Yes |
| UnitClass vtable `0x007F5C70` | `+0x55C` | `0x0073B140` | `UnitClass__Draw_Sprite_With_BridgeFudge`, direct SHP blitter path | Yes |
| UnitClass vtable `0x007F5C70` | `+0x1C8` | `0x0073C5F0` | `UnitClass__Draw_Body_And_Turret`, calls body helper slots | Yes |
| Blitter selector table | `+0x78` | runtime object with `vtable_ZBuf_50pct_blend` | standard no-mask 50% remap blend | Conditional |
| `vtable_ZBuf_50pct_blend` | `+0x4` | `0x004941E0` | forward scanline method | Conditional |
| Blitter selector table | `+0xA4` | runtime object with vtable `0x007E56A8` | mask-family 50% remap blend | Conditional |

Evidence: vtable bytes read from `gamemd.exe` at image base `0x00400000` show `0x007F61C4 = 70 B4 73 00`, `0x007F61CC = 40 B1 73 00`; Ghidra decompile for `0x0073B140`, `0x0073C5F0`, `0x0048EBF0`, `0x00490B90`, `0x004941E0`; Ghidra disassembly success for `0x0073B470..0x0073B61F`, `0x0048EBF0..0x0048F9BF`, `0x004941E0..0x0049429F`.

## 3. Core Logic

### UnitClass `Voxel=no` body path

Active in YR: Yes. Stock YR has UnitClass SHP vehicles through `artmd.ini` `Voxel=no`: `[DLPH]`, `[DRON]`, and `[SQD]`. The UnitClass vtable is live; prior reports also tie UnitClass body rendering to vtable slots in this same vtable.

Read-only `0x0073B470` evidence:

- `0x0073B470` has no Ghidra function boundary, so this report used Ghidra disassembly success plus PE byte disassembly from the retail `gamemd.exe`.
- The UnitClass vtable at `0x007F5C70` contains `+0x554 = 0x0073B470` and `+0x55C = 0x0073B140`.
- Inside the `0x0073B470` byte body, `0x0073C192..0x0073C1A5` pushes body rectangle/material arguments and calls `call dword ptr [eax + 0x55c]`.
- With the UnitClass vtable, that dispatch target is `0x0073B140`.
- No direct `call 0x00705E00` occurs in the inspected `0x0073B470` body range. The call chain for this UnitClass slot is therefore `0x0073B470 -> vtable+0x55C -> 0x0073B140`, not `TechnoClass_DrawSHP`.

`UnitClass__Draw_Sprite_With_BridgeFudge @ 0x0073B140` then builds SHP flags internally:

- base `uVar9 = 0x2800`;
- visual state `1` changes to `0x2802`;
- visual states `2`/`3` change to `0x2804`;
- visual state `4` changes to `0x280A` or `0x280C`;
- virtual warp predicates `+0x1D4/+0x1D8` OR `0x04`;
- house/player cloak adjustment may rewrite flags through virtual `+0x43C`;
- it calls `Blitter_selector(uVar9)` and `Standard_SHP_blitter`, not `CC_Draw_Shape` or `TechnoClass_DrawSHP`.

Because the UnitClass SHP path starts from `0x2800`, a normal warped `Voxel=no` UnitClass body reaches `0x2804`, where `0x3000 & flags != 0`; the standard selector chooses `+0xA4`, not `+0x78`.

### Standard no-mask `+0x78` leaf

Active in YR: Conditional. The selector and leaf are live shared render code. The `+0x78` slot is selected only for an SHP standard-frame draw whose flags satisfy:

- `param_2 & 0x10 == 0` in `Blitter_selector`;
- `(param_2 & 6) == 4`;
- `(param_2 & 0x4000) == 0`;
- `(_g_BlitterFlagMask_0x3000 & param_2) == 0`;
- `(param_2 & 0x800) != 0`.

Evidence: `Blitter_selector @ 0x00490B90` decompile; Ghidra disassembly success `0x00490B90..0x00490DF6`.

`Blitter_init @ 0x0048EBF0` constructs `param_1 + 0x78` with:

- `operator_new(0x10)`;
- vtable `vtable_ZBuf_50pct_blend`;
- `object+4 = *(param_1 + 0x170)`;
- `object+8 = FUN_00420140(*(param_1 + 0x16C))`;
- `*(undefined2 *)(object+0xC) = *(param_1 + 0x180)`.

The vtable readback for `vtable_ZBuf_50pct_blend @ 0x007E5798` gives method `+0x4 = 0x004941E0`. Decompiling `0x004941E0` shows:

```text
if source_index != 0:
    remapped = table[(intensity_table[abuffer_pixel] | source_index)]
    dst = (remapped >> 1 & mask) + (dst >> 1 & mask)
advance A-buffer pointer with wrap
```

The method reads `g_ABuffer` and destination pixels, skips source index `0`, clamps the light/intensity selector to `0xFE`, and uses the half-mask at `object+0x0C`. It does not read or write `g_ZBuffer`. Its "ZBuf" vtable name is misleading for this leaf's scanline body.

## 4. INI Keys

| Key | Location | Stock value / examples | Effect in this slice | Active in YR |
|---|---|---|---|---|
| `Voxel=no` | `artmd.ini` Unit art sections | `[DLPH]`, `[DRON]`, `[SQD]` | Makes these UnitClass bodies SHP-rendered rather than voxel-rendered | Yes |
| `WalkFrames=` / `FiringFrames=` | `artmd.ini` Unit art sections | e.g. DLPH/DRON/SQD define SHP vehicle frame groups | Consumed by UnitClass body frame selection before SHP blit | Yes |
| `Teleporter=yes` | `rulesmd.ini` techno sections | `[CLEG]` and chrono units | Proves SHP teleport liveness for infantry/Techno paths, but not the UnitClass `Voxel=no` vehicle body path | Yes/conditional |

## 5. Integration Points

| Stage | Function / address | Condition | Result |
|---|---|---|---|
| Unit draw wrapper | `UnitClass vtable +0x554 -> 0x0073B470` | UnitClass draw path; stock SHP vehicles exist with `Voxel=no` | Sets up body data, calls `vtable+0x55C` |
| SHP body/sprite blit | `UnitClass vtable +0x55C -> 0x0073B140` | UnitClass SHP body/shadow path | Builds `0x2800`-rooted flags and calls `Blitter_selector` + `Standard_SHP_blitter` |
| Selector | `Blitter_selector @ 0x00490B90` | standard/non-RLE SHP frame | Picks `+0xA4` for `0x2804`, `+0x78` for no-mask `0x0E04`-style flags |
| Leaf constructor | `Blitter_init @ 0x0048EBF0` | first selector use | Initializes `+0x78` with `vtable_ZBuf_50pct_blend` |
| Scanline leaf | `0x004941E0` | selected no-mask 50% standard path | Post-remap 16-bit 50% blend, no `g_ZBuffer` access |

## 6. Current Rust Implementation Status

Current Rust has SHP vehicle support and a generic `SpriteInstance` render path, but no native material-key split matching the binary selector table.

Observed surfaces:

- `src/app_sim_tick.rs`: comments SHP vehicles as `Voxel=no` and builds SHP vehicle sequences.
- `src/sim/world/world_spawn.rs`: sets `is_voxel` from art data and initializes SHP vehicle animation for non-voxel Unit/Aircraft entities.
- `src/rules/art_data.rs`: parses `Voxel=no` and SHP vehicle frame tags.
- `src/app_instances/units.rs`: applies simple alpha/render state for units, not native SHP selector families.
- `src/render/batch.rs` and `src/render/sprite_voxel_shader.wgsl`: generic `SpriteInstance` alpha/fx metadata, no selector-family material identity for `+0x78` vs `+0xA4`.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Working notes | verified | report preface | none |
| UnitClass vtable `+0x554` identity | verified | PE vtable read `0x007F61C4 = 0x0073B470`; Ghidra disassembly success | none |
| `0x0073B470` raw body | verified-for-scope | PE disassembly from `0x0073B470`; Ghidra disassembly success `0x0073B470..0x0073B61F` | complete decompile remains unavailable without mutating Ghidra |
| `0x0073B470 -> vtable+0x55C` | verified | raw bytes `0x0073C192..0x0073C1A5` call `[eax+0x55C]`; vtable `+0x55C=0x0073B140` | none |
| Absence of direct `TechnoClass_DrawSHP` in `0x0073B470` | verified-for-scope | raw byte pass over body range; no direct `call 0x00705E00`; observed dispatch to `+0x55C` | indirect exotic dispatch outside inspected body not claimed |
| `UnitClass__Draw_Sprite_With_BridgeFudge` flags | verified | decompile `0x0073B140` | none |
| `Blitter_selector +0x78` predicate | verified | decompile `0x00490B90`, disassembly success | none |
| `Blitter_init +0x78` construction | verified | decompile `0x0048EBF0`, disassembly success | none |
| `0x004941E0` scanline leaf | verified | decompile `0x004941E0`, disassembly success | none |
| Runtime framebuffer sample | deferred | requires live capture | pixel fixture follow-up |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is `0x0073B470` in the UnitClass vtable? -> Yes, UnitClass vtable base `0x007F5C70` has `+0x554 = 0x0073B470`.` (evidence: PE vtable read; Active in YR: Yes)
- `[RESOLVED] OQ-02 - Does `0x0073B470` have a Ghidra function boundary? -> No; Ghidra reports no function, so this report uses read-only byte disassembly without creating a function.` (evidence: `decompile_function 0x0073B470` failure; Active in YR: Yes)
- `[RESOLVED] OQ-03 - Where does `0x0073B470` send SHP/body blit work? -> It dispatches `vtable+0x55C`, which UnitClass maps to `0x0073B140`.` (evidence: raw disassembly `0x0073C192..0x0073C1A5`; vtable read)
- `[RESOLVED] OQ-04 - Does this UnitClass path directly call `TechnoClass_DrawSHP`? -> No direct call was found in the inspected body; the proven path is `0x0073B470 -> 0x0073B140`.` (evidence: raw byte pass; Active in YR: Yes)
- `[RESOLVED] OQ-05 - Is `Voxel=no` active stock UnitClass content? -> Yes, stock `[DLPH]`, `[DRON]`, and `[SQD]` have `Voxel=no`.` (evidence: `ini/artmd.ini`; Active in YR: Yes)
- `[RESOLVED] OQ-06 - Does UnitClass SHP warp select no-mask `+0x78`? -> Not for the normal `0x0073B140` path; it starts from `0x2800`, so warped flags include `0x2000` and select `+0xA4`.` (evidence: `0x0073B140`, `0x00490B90`; Active in YR: Yes/Conditional)
- `[RESOLVED] OQ-07 - What predicate selects standard no-mask `+0x78`? -> `flags&6==4`, no `0x10`, no `0x4000`, `0x3000` clear, and `0x800` set.` (evidence: `0x00490B90`; Active in YR: Conditional)
- `[RESOLVED] OQ-08 - What method implements `+0x78` forward scanline? -> `vtable_ZBuf_50pct_blend +0x4 = 0x004941E0`.` (evidence: `0x0048EBF0`, vtable read `0x007E5798`; Active in YR: Conditional)
- `[RESOLVED] OQ-09 - Does `0x004941E0` write Z? -> No observed `g_ZBuffer` access; it reads A-buffer/remap/intensity and writes destination pixels only.` (evidence: decompile `0x004941E0`; Active in YR: Conditional)
- `[DEFERRED] OQ-10 - Exact CLEG/Unit SHP warp framebuffer samples.` (category: needs-runtime-debugger; reason: requires native runtime capture or software blit fixture; next-step-if-pursued: build one-pixel source/destination fixture for `+0x78` and `+0xA4`)

## 9. Visual Composition Ledger

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---|---|---|---|---|---|---|---|
| 1 | `0x0073B470` UnitClass vtable `+0x554` | UnitClass draw body path; stock SHP vehicles exist | Unit body SHP for `Voxel=no` units | caller/body computed | house/lighting inputs carried through body setup | Yes | Unit body wrapper |
| 2 | `0x0073B140` UnitClass vtable `+0x55C` | `0x0073B470` dispatch at `0x0073C1A5` | same SHP body/shadow | `DAT_00B1CFC0..CC` body rect | `Blitter_selector` selected remap/intensity path | Yes | SHP body/sprite blit |
| 3 | `Blitter_selector @ 0x00490B90` | UnitClass normal warp gives `0x2804`; no-mask conditional gives `0x0E04`-style flags | standard SHP frame bytes | clipped scanline | selector slot family | Yes/Conditional | blitter select |
| 4a | `+0xA4` mask-family | `0x2804` | standard SHP frame | scanline | post-remap 50% family | Conditional | normal UnitClass warped SHP path |
| 4b | `+0x78` no-mask family -> `0x004941E0` | `flags&6==4`, `0x800`, no `0x3000` | standard SHP frame | scanline | post-remap 16-bit 50% blend | Conditional | no-mask 50% path |

Asset role matrix:

| Asset / class | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|---|
| `[DLPH]` SHP body | yes | yes | yes | body | no | no | no | no | `artmd.ini Voxel=no`; UnitClass path |
| `[DRON]` SHP body | yes | yes | yes | body | no | no | no | no | `artmd.ini Voxel=no`; UnitClass path |
| `[SQD]` SHP body | yes | yes | yes | body | no | no | no | no | `artmd.ini Voxel=no`; UnitClass path |
| CLEG SHP body | yes | yes | yes | body | no | no | no | no | `rulesmd.ini Teleporter=yes`; Techno/SHP path from prior report, not this UnitClass slot |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| UnitClass `Voxel=no` bodies route through `0x0073B470 -> vtable+0x55C -> 0x0073B140`, not directly through `TechnoClass_DrawSHP`. | vtable read `+0x554/+0x55C`; raw bytes `0x0073C192..0x0073C1A5`; decompile `0x0073B140` | missing/unchecked: Rust SHP vehicle draw path is generic sprite batching | `src/app_sim_tick.rs`, `src/sim/world/world_spawn.rs`, `src/app_instances/units.rs`, SHP vehicle render path | Model UnitClass SHP vehicle warp material from the UnitClass SHP selector path, not only from `TechnoClass_DrawSHP`. | `Voxel=no` DRON/DLPH/SQD body with warp predicate uses UnitClass SHP material key from `0x2804`. Proposed test: `voxel_no_unit_warp_uses_unitclass_shp_selector_path`. | Do not assume every SHP techno body enters `TechnoClass_DrawSHP`. |
| Normal UnitClass SHP warp flags include `0x2000` through `0x0073B140`, so standard selector chooses `+0xA4` rather than no-mask `+0x78`. | decompile `0x0073B140`; decompile `0x00490B90` | missing: no native selector-family material keys | `src/render/batch.rs`, `src/render/sprite_voxel_shader.wgsl`, material batching | Preserve `0x3000`/mask-family state in the render material key. | A warped `Voxel=no` UnitClass body traces to mask-family 50% material, while a no-mask fixture traces to no-mask 50% material. Proposed test: `unitclass_shp_warp_0x2804_selects_mask_family_not_nomask`. | Do not key material only on `flags & 6 == 4`. |
| Standard no-mask `+0x78` forward leaf performs post-remap/intensity 16-bit 50% blending and does not access `g_ZBuffer`. | decompile `0x004941E0`; `Blitter_init @ 0x0048EBF0`; selector `0x00490B90` | mismatch likely: Rust GPU alpha is not native 16-bit indexed blend | `src/render/sprite_voxel_shader.wgsl`, future software/reference SHP blitter | Implement final SHP 50% parity as indexed/remap blend, with separate material identity for no-mask `+0x78`. | Source index nonzero over known 16-bit destination matches native half blend; source index zero leaves destination unchanged. Proposed test: `shp_nomask_50pct_leaf_blends_post_remap_and_skips_zero`. | Do not treat `alpha=0.5` as final parity and do not invent Z writes for this leaf. |

## 11. Negative Facts / Do Not Do

- Do not claim the UnitClass `Voxel=no` SHP body path directly calls `TechnoClass_DrawSHP`; the proven `0x0073B470` path dispatches `vtable+0x55C` to `0x0073B140`.
- Do not use no-mask `+0x78` for the normal UnitClass SHP warp path rooted at `0x2800`; warped `0x2804` selects the mask-family branch.
- Do not key SHP 50% material solely on `flags & 6 == 4`; `0x3000`, `0x4000`, `0x800`, and `0x8` change selector family.
- Do not infer Z writes from the misleading `vtable_ZBuf_50pct_blend` name; the inspected `0x004941E0` body does not access `g_ZBuffer`.
- Do not collapse UnitClass SHP vehicles, infantry SHP bodies, and building SHP bodies into one caller path just because they share SHP assets.

## 12. Remaining Uncertainty

- Exact native framebuffer samples for CLEG/DRON/DLPH/SQD warp over real terrain remain uncaptured.
- `0x0073B470` still lacks a Ghidra function boundary; this report proves the scoped call path from read-only bytes but does not name every local or every branch in that large function.
- SHP frame compression distribution for every stock SHP unit was not enumerated; this report distinguishes standard `Blitter_selector` from extended selector behavior only where needed for `+0x78`.
- Direct callers above UnitClass vtable `+0x554` were not globally censused; UnitClass vtable liveness and stock `Voxel=no` content prove active reachability for this slice.

## 13. Stale Docs / Replacement Wording

- `C:/Users/enok/Documents/ra2-rust-game/docs/research/SHP_TECHNO_WARP_DRAW_PATH_GHIDRA_REPORT.md`: replace the deferred UnitClass wording with: "UnitClass `Voxel=no` SHP vehicle bodies do not route directly through `TechnoClass_DrawSHP`. The UnitClass vtable at `0x007F5C70` has `+0x554 = 0x0073B470`; read-only bytes in that function dispatch `vtable+0x55C`, which resolves to `UnitClass__Draw_Sprite_With_BridgeFudge @ 0x0073B140`. That path builds `0x2800`-rooted SHP flags and calls `Blitter_selector` plus `Standard_SHP_blitter` directly."
- `C:/Users/enok/Documents/ra2-rust-game/docs/research/CLOAKING_VISUAL_PIPELINE.md`: replace blanket `flags&6 == 4 -> +0x78` wording with: "`+0x78` is only the standard no-mask 50% family selected when `0x3000` is clear and `0x800` is set. UnitClass SHP vehicle warp via `0x0073B140` normally uses `0x2804`, so it selects `+0xA4`, while no-mask `0x0E04`-style callers select `+0x78`."
- `C:/Users/enok/Documents/ra2-rust-game/docs/research/VXL_RASTERIZER_DISPATCH_GHIDRA_REPORT.md`: add: "SHP UnitClass `Voxel=no` vehicle bodies have a separate UnitClass path (`0x0073B470 -> 0x0073B140`) and should not be generalized from VXL `TechnoClass::Draw` or `TechnoClass_DrawSHP` alone."
- `C:/Users/enok/Documents/ra2-rust-game/docs/research/ZBUFFER_DEPTH_SYSTEM.md`: refine the `+0x78` row with: "The standard no-mask `+0x78` forward leaf uses `vtable_ZBuf_50pct_blend +0x4 = 0x004941E0`; despite the vtable name, the inspected scanline method does not read/write `g_ZBuffer`; it reads A-buffer/remap/intensity and half-blends into destination pixels."

## Sources

- Ghidra read-only decompile: `UnitClass__Draw_Sprite_With_BridgeFudge @ 0x0073B140`.
- Ghidra read-only decompile: `UnitClass__Draw_Body_And_Turret @ 0x0073C5F0`.
- Ghidra read-only decompile: `TechnoClass__Draw @ 0x00706640` and `TechnoClass_DrawSHP @ 0x00705E00` for contrast only.
- Ghidra read-only decompile: `Blitter_selector @ 0x00490B90`.
- Ghidra read-only decompile: `Blitter_init @ 0x0048EBF0`.
- Ghidra read-only decompile: `Blitter_Scanline_Blend50pct_Remap @ 0x004941E0`.
- Ghidra read-only disassembly success: `0x0073B470..0x0073B61F`, `0x00490B90..0x00490DF6`, `0x0048EBF0..0x0048F9BF`, `0x004941E0..0x0049429F`.
- Retail `gamemd.exe` PE byte read, image base `0x00400000`, vtable pointers at `0x007F61C4` and `0x007F61CC`.
- INI: `C:/Users/enok/Documents/ra2-rust-game/ini/artmd.ini` (`[DLPH]`, `[DRON]`, `[SQD] Voxel=no`).
- INI: `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini` (`[CLEG] Teleporter=yes` for SHP teleport contrast).
