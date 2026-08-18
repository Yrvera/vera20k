# Adversarial verdicts — dispatcher-rng-gate

Auditor stance: refute-by-default. Each D re-checked live against the binary
(`decompile_function` / `disassemble_function`) and against current Rust source.

Live anchors re-confirmed this session:
- `Apply_area_damage @ 0x00489280` (`get_function_by_address` → name+entry match;
  `disassemble_function 0x00489280` full body 0x00489280–0x0048a4e5).
- `ApplyDamageToCell @ 0x00587180` (`get_function_by_address` + `decompile_function`).
- `Random__RandomRanged @ 0x0065C7E0` (`decompile_function`).
- `DestroyBridge_Low @ 0x0057baa0`, `DestroyBridge_High @ 0x0057ccf0`
  (`get_function_by_address` — both match the Block C/D `CALL` targets at
  0x0048a25a / 0x0048a2b4).

Rust re-read: `bridge_orchestrator.rs::run_dispatch_loop` (lines 1374–1478),
`apply_bridge_damage_events` (56–138), `bridge_state/mod.rs::path_matches_cell`
(836–905), `combat/combat_aoe.rs::bridge_adjusted_impact_z` (45–59).

---

## D1: First-match `break` collapses 4 independent RNG branches — VERDICT=REAL

Confirmed live. `disassemble_function 0x00489280` shows the four bridge blocks run
with NO early-out / NO break:
- Block A (`LAB_00489f77`): RNG gate `0x00489fe0` → `CALL 0x0065c7e0`
  (`Random__RandomRanged`) at `0x00489ff5`; `0x00489ffe JGE 0x0048a099` on fail;
  pass → `ApplyDamageToCell` `0x0048a00e` + 3-retry loop (`0x0048a015 MOV ESI,3`).
  Block A exit `LAB_0048a099` → `0x0048a0a5` (Block B). No break.
- Block B (`0x0048a0a5`): RNG gate `0x0048a165` → `CALL 0x0065c7e0` at `0x0048a179`;
  `0x0048a182 JGE 0x0048a214` → falls to Block C. No break.
- Block C (`0x0048a214`): `CMP 0x4a JL` / `CMP 0x63 JG`; RNG `0x0048a245`;
  `DestroyBridge_Low` `0x0048a25a` → falls to Block D. No break.
- Block D (`0x0048a26a`): `CMP 0xcd JL` / `CMP 0xe6 JG`; RNG `0x0048a29f`;
  `DestroyBridge_High` `0x0048a2b4`.

Each gate calls `Random__RandomRanged(1, Rules+0x1740)` from the `Scenario+0x218` RNG
(`LEA ECX,[EDX+0x218]` before each call). Rust (`bridge_orchestrator.rs:1407–1474`)
evaluates the 4 paths and `break`s at :1473 after the first matching path — at most
1 BridgeStrength draw per event. Finder reading holds.

Corrected delta: Rust = 1 BridgeStrength draw per event (first matching path, then
`break`) → gamemd = each of the 4 blocks (A,B,C,D) that matches its own
tile-index/overlay test independently rolls its own `R(1,BridgeStrength)` from
Scenario RNG with NO early-out; a cell satisfying ≥2 block predicates rolls ≥2 draws.

---

## D2: Z-window lower bound off-by-one + wrong unit frame — VERDICT=REAL

Confirmed live. Block A Z window (`disassemble_function 0x00489280`):
- `0x00489f82 MOVSX EAX,byte ptr [EDI+0x11b]` (Level)
- `0x00489f8d LEA ESI,[EAX+1]` (Level+1); `0x00489f90 IMUL ESI,[0x0089e870]` (LevelHeight);
  `0x00489f97 ADD ESI,[0x0089e864]` (BridgeHeight); `0x00489fa0 CMP ECX,ESI; JG 0x0048a0a5`
  — upper bound reject if `impact_z > (Level+1)*LH+BH`.
- `0x00489fa8 ADD EAX,-0x2` (Level-2); `0x00489fb1 IMUL EAX,[0x0089e870]`;
  `0x00489fb8 ADD EAX,ESI(=BridgeHeight)`; `0x00489fba CMP ECX,EAX; JLE 0x0048a0a5`
  — lower bound reject if `impact_z <= (Level-2)*LH+BH` (exclusive floor).
  `ECX = local_c8[2] = param_1[2]` = detonation coord Z (CoordStruct/lepton units).
Block B mirror at `0x0048a10d–0x0048a141`.

Rust `path_matches_cell` (`bridge_state/mod.rs:895–901`) rejects when
`impact_z < level-1 || impact_z > level+1` — raw level units, lower bound `level-1`
inclusive. Rust's `impact_z` is `cell.level + bridge_height_for_selector(cell)` in
LEVEL units (`combat_aoe.rs:54–56`), not a lepton Z. Two real defects: (1) lower
bound `-1`-inclusive vs binary `-2`-exclusive; (2) entire window expressed in level
units vs binary's `(Level±k)*LevelHeight + BridgeHeight` lepton bounds — non-equivalent
at any LevelHeight. Finder reading holds.

Corrected delta: Rust = accept `level-1 <= impact_z <= level+1` (raw level units) →
gamemd = accept `(Level-2)*LevelHeight+BridgeHeight < impact_z <= (Level+1)*LevelHeight+BridgeHeight`
(lepton Z), and only when `Flags & 0x100` set (see D5).
Caveat (matches finder): `DAT_0089e870`/`DAT_0089e864` magnitudes are runtime-init
(BSS), so the numeric lepton gap is UNCHECKED; the structural off-by-one + unit-frame
mismatch are proven regardless.

---

## D3: High/Low SM routing uses `deck_level>=4` not tile-index+flag — VERDICT=REAL

Confirmed live. `decompile_function 0x00587180` (`ApplyDamageToCell`): after the two
overlay short-circuits, the High-vs-Low SM choice is driven purely by:
- `Flags & 0x100` + perpendicular-neighbor `OverlayTypeIndex == 0x18/0x19`
  → `ProcessBridgeDamageStateMachine_High`;
- else tile-index set `(IsoTile - DAT_00aa0e28)+1` ∈ {DAT_00abad30..+3, DAT_00aa1028..+3}
  also → High path; the second set `(IsoTile - DAT_00abad1c)+1` + neighbor `0xed/0xee`
  → `ProcessBridgeDamageStateMachine_Low`.
No `deck_level >= 4` (or any level-N threshold) anywhere in `ApplyDamageToCell`.

Rust `path_matches_cell` (`bridge_state/mod.rs:884–890`) discriminates `is_high =
cell.deck_level >= 4` and requires `is_high == want_high`. Proxy, not the binary
mechanism. Finder reading holds.

Corrected delta: Rust = HighSM iff `cell.deck_level >= 4` (single path matches) →
gamemd = High vs Low chosen by iso-tile-type-index membership in the high/low tile
sets + `0x18/0x19` vs `0xed/0xee` perpendicular-neighbor overlay; no deck-level test.
Frequency caveat (matches finder UNCHECKED): whether a stock YR map ever crosses the
`4` threshold against its tile family is unverified, so in-skirmish trigger frequency
is not pinned. Latent for modded/custom maps and the 20k scale target regardless.

---

## D4: ApplyDamageToCell is overlay-first dispatcher + duplicate Block C/D gates — VERDICT=REAL

Confirmed live. `decompile_function 0x00587180`: `ApplyDamageToCell` checks overlay
FIRST — `0x49 < OverlayIdx < 100` → `DestroyBridge_Low` (return); `0xcc < OverlayIdx
< 0xe7` → `DestroyBridge_High` (return) — before any tile-index SM branch. Block A's
own match test is on `IsoTileTypeIndex` (`EDI+0x38`, disasm `0x00489ec9 MOV ESI,[EDI+0x38]`),
NOT OverlayTypeIndex, so a raw in-band overlay cell can pass Block A's tile-index gate +
RNG roll, enter `ApplyDamageToCell`, and be routed to `DestroyBridge_High` by the overlay
short-circuit. Flow then falls through to Block D (`0x0048a26a CMP 0xcd / 0xe6`), whose
overlay gate ALSO matches → second RNG roll (`0x0048a29f`) → `DestroyBridge_High` again.
Two draws / two destroy attempts for the one cell.

Rust splits Direct vs SM into 4 sibling paths and `break`s after the first match:
HighDirect matches an in-band 0xD0 cell → 1 draw → `destroy_bridge_high` → break
(`bridge_orchestrator.rs:1456–1473`; SM matcher rejects in-band overlays at
`bridge_state/mod.rs:854–862`). 1 draw / 1 attempt. Finder reading holds; this is the
cleaner 2-vs-1 proof (independent of the D1 Block-A/B tile-index-overlap question).

Corrected delta: Rust = in-band overlay cell → 1 draw, 1 `DestroyBridge_*`, break →
gamemd = same cell can roll 2 draws (Block A/B gate → `ApplyDamageToCell` overlay
short-circuit → `DestroyBridge_*`, AND Block C/D gate → `DestroyBridge_*`), provided
its iso-tile index lands in Block A/B's range.
Caveat: the 2nd path requires the cell's iso-tile index ∈ Block A/B tile set
(`DAT_*` magnitudes runtime-init, UNCHECKED) — but the Block C/D duplicate gate is a
standalone overlay test, so the duplication is structurally proven; whether BOTH fire
for a given cell depends on that tile-set membership.

---

## D5: Z window applied unconditionally in Rust; binary gates it on structural flag — VERDICT=REAL

Confirmed live. `disassemble_function 0x00489280`: `0x00489f77 MOV EAX,[EDI+0x140]`;
`0x00489f7d TEST AH,0x1` (Flags & 0x100); `0x00489f80 JZ 0x00489fc2` — when the cell
is NOT structural, the JZ jumps PAST the entire Z window straight to the
`warhead+0x144` + RNG gate at `0x00489fc2`. A cell reaches `LAB_00489f77` either via
the structural+`0x18/0x19`-neighbor branch OR via the non-structural tile-index branch
(`LAB_00489f27`), so a non-structural anchor-adjacent cell that matched the tile-index
test gets the RNG roll with NO Z gate. Block B mirror: `0x0048a108 TEST AH,0x1` /
`0x0048a10b JZ 0x0048a147`.

Rust `path_matches_cell` applies the Z gate to EVERY SM-path candidate
(`bridge_state/mod.rs:892–901`), no structural-flag bypass. Finder reading holds.

Corrected delta: Rust = Z gate on every SM candidate → gamemd = Z gate runs only when
`Flags & 0x100` set; non-structural (anchor-zone-via-neighbor) cells skip the Z window
and go straight to the RNG gate.
Observability caveat (matches finder): whether the Rust combat boundary ever emits an
SM-routable event for a non-structural anchor-zone cell depends on `role`/`overlay`
population in `from_resolved_terrain` (outside this facet's two files), so this is
LIKELY-DRIFT / latent rather than a proven in-game divergence. Mechanism on the binary
side is proven.

---

## PARITY-CONFIRMED spot-checks (auditor agrees with finder)

- RNG range inclusivity: `decompile_function 0x0065C7E0` returns `uVar2 + param_2`
  with `uVar2 ∈ [0, range]` → inclusive `[lo, hi]`. Rust `next_range_u32_inclusive(1, S)`
  matches. Gate strictness `0x00489ffe JGE skip` = pass iff `roll < damage`; Rust
  `!((roll as u16) < damage) => continue` (`:1420`) matches (equality fails in both).
- IonCannon bypass: `0x00489fd8 CMP ECX,[EAX+0xff0]; JZ 0x0048a004` skips the RNG call;
  Block B/C/D mirror (`0x0048a163`, `0x0048a22f`, `0x0048a289`). Rust `if !ctx.is_ion_cannon`
  (`:1418`) matches — zero gate draws for Ion on every path.
- 3-retry / 4-attempt SM-only: `0x0048a015 MOV ESI,3` retry loop on `ApplyDamageToCell`
  (Block A) + mirror `0x0048a199`; Block C/D (`DestroyBridge_*`) have NO retry loop.
  Rust `max_attempts = if is_ion && path.is_state_machine() {4} else {1}` (:1429) matches.
- Direct-overlay ranges 0x4A..=0x63 / 0xCD..=0xE6 confirmed at Block C/D disasm and in
  `ApplyDamageToCell` (`0x49<x<100`, `0xcc<x<0xe7`). Rust ranges match (`mod.rs:848–849`).
- Outer DestroyableBridges gate: `0x00489eb2 TEST CH,0x80; JZ 0x0048a2c4` (Scenario&0x8000)
  + `0x00489ec1 [warhead+0x144]` Bridge= check. Rust `is_destroyable()` bail (:66–69) matches
  (the warhead Bridge= half is pre-resolved at the Rust combat boundary).

---

## MISS (new disparities the finder did not surface)

- MISS [LOW, latent]: The binary's Block A/B Direct route inside `ApplyDamageToCell`
  (overlay-first → `DestroyBridge_Low`/`DestroyBridge_High`) is reached only AFTER the
  Block A/B **structural-flag + Z-window** gate (the SM gate at `0x00489f77`/`0x0048a102`).
  Rust routes ALL in-band overlay cells through `HighDirect`/`LowDirect`, which have NO
  Z gate at all (`path_matches_cell` :848–849 return immediately). So for a STRUCTURAL
  in-band overlay cell hit OUTSIDE the Z window, the binary's Block-A/B `DestroyBridge_*`
  route does NOT fire (Z gate rejects), and only the Block C/D route (also no Z gate)
  fires once — whereas Rust's `HighDirect` always fires regardless of Z. Net: this caps
  the binary at 1 draw for an off-Z structural in-band cell (only Block C/D), while Rust
  also gives 1 draw — so draw COUNT happens to coincide here, but the binary additionally
  runs `ApplyDamageToCell`'s SM-vs-direct internal logic that Rust never reaches for
  in-band cells. This is a routing/precedence difference adjacent to D4 but distinct:
  it is the Z-gate's effect on the Block-A/B Direct sub-route, which D4 framed only for
  the in-Z-window case. Latent; depends on whether such cells are emitted.

- MISS [INFO, not a drift]: `Apply_area_damage` recurses on bridge-destruction
  (`0x0048a371 CALL 0x00489280` with warhead `Rules+0xfa8`) inside the overlay-destroy
  fallout block (`LAB_0048a2c4`), and that block (animations / VoxelAnim 15% / Particle
  25% via `Random__RandomRanged(0,99)` at `0x0048a394`/`0x0048a3e3`) consumes additional
  Scenario-RNG draws on overlay destruction. This facet's two Rust files do not cover the
  `LAB_0048a2c4` fallout path (it lives in the cascade/debris helpers); not scored here,
  but flagged: the overlay-clear → secondary-`Apply_area_damage` + 2 chance draws are a
  separate RNG-order surface outside `run_dispatch_loop`/`path_matches_cell`.
