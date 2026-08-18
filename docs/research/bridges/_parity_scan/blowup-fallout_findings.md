# Parity Scan — BlowUpBridge Fallout Ordering (kill / drop-in / debris per cell)

Facet KEY: `blowup-fallout`
Rust under test: `src/sim/world/bridge_orchestrator.rs`
gamemd anchors verified live this session:
- `CellClass__BlowUpBridge @ 0x0047dd70` (`get_function_by_address` → "Function: CellClass__BlowUpBridge at 0047dd70", body `0047dd70-0047e036`)
- `CellClass__SetBridgeDirection_NESW @ 0x0047e040`, `_NWSE @ 0x0047e470` (decompiled; byte-identical twins)
- `ObjectClass__DropIn @ 0x005F4160` (decompiled)
- Float gate constants read live (`read_memory`): `0x007e4f58 = 0.95`, `0x007e1738 = 0.5`, `0x007e3570 = 2^-31`, `0x007e4f50 = 50.0`.

Authority: live Ghidra decompile/disassembly/read_memory this session > research doc
`05-damage-collapse-repair-cabhut/BRIDGE_COLLAPSE_FALLOUT_ORDERING_GHIDRA_REPORT.md` (status: verified;
its core ordering claims reproduced here from the live binary).

---

## Summary verdict

The per-cell fallout **ordering** (ground kill → deck DropIn → debris) and the **RNG gate
thresholds** (outer 95%, metallic 50%) and **draw count/order** are PARITY. The jitter offset is
algebraically PARITY (floor of the same real expression). The real disparities are in the *kill
primitive itself*: Rust's `kill_ground_occupants_at` is a hand-rolled health=0 kill that (a) does
NOT route through the death-effect pipeline a real `Take_Damage(C4Warhead, force_kill)` produces
(no death AnimList explosion / wreck / smudge / passenger eject / score), and (b) over-selects —
its cell filter catches airborne units hovering over the cell, which gamemd's ground object-list
(`CellClass+0xE4`) never contains. Also the debris/explosion anim Z differs (Rust uses deck level;
binary computes Z=0 from two zeroed TS-era constants).

---

### D1: Ground-occupant force-kill skips the real death-effect pipeline

- Rust now: `kill_ground_occupants_at` (`bridge_orchestrator.rs:998-1024`) sets
  `entity.health.current = 0; entity.dying = true; clears targets/selection; if animation: switch_to(death_seq)`.
  Nothing emits a death-explosion AnimList, wreck/husk, building/vehicle smudge, passenger
  ejection, or killer-score credit. Non-animated victims (voxel vehicles) are cleaned up later by
  the animation system's `dying && no-animation → dying_finished` path
  (`src/sim/animation.rs:403-406`) — i.e. they just vanish, no explosion.
- gamemd: `CellClass__BlowUpBridge @ 0x0047dd70` ground loop (asm `0047dd84..0047ddb8`) calls the
  victim's vtable slot `+0x16c` = `Take_Damage(&Health, 0, RulesClass+0xfa8 (=C4Warhead), 0, 1, 1, 0)`
  for every object in `CellClass+0xE4`. The `force-kill` Take_Damage runs the full
  ObjectClass/TechnoClass death pipeline: a C4Warhead/`InfDeath`-driven death produces the normal
  destruction visuals (vehicle explosion AnimList, husk/wreck, smudge) exactly as a lethal hit
  would, plus passenger handling.
- Fixture: a Rhino tank (voxel, no SHP `animation`) sits on the bridge ground layer (NOT on the
  deck) of a high bridge at cell (40,30). The 3-cell `SetBridgeDirection` collapse calls
  `BlowUpBridge` on (40,30). gamemd: Take_Damage(C4Warhead, force_kill) → tank explodes with its
  death AnimList + leaves a wreck/smudge. Rust: `kill_ground_occupants_at` sets health=0/dying=true,
  no animation → `dying_finished` removes it silently next anim tick — no explosion, no wreck.
- Player sees: a unit standing under/beside a collapsing bridge span on the ground layer dies
  without its normal death explosion/wreck. Triggers any time a ground-layer unit occupies a
  BlowUpBridge cell at collapse — uncommon but reproducible (units crossing under a high bridge, or
  on the ground at a low-bridge collapse cell). MED.
- Severity: MED (player-visible missing death FX, but the trigger — a ground unit exactly on a
  collapse cell — is occasional, not every match).
- Confidence: PROVEN-DRIFT (binary calls full Take_Damage; Rust uses a bespoke health-zero kill
  that bypasses the combat death-effect path).
- Verify-call: `decompile_function 0x0047dd70` + `disassemble_function 0x0047dd70` (ground loop
  `0047dd84..0047ddb8`, `CALL dword ptr [EAX + 0x16c]` at `0047ddae` with args
  `PUSH 0/1/1/0/[Rules+0xfa8]/0/&Health`).

---

### D2: Ground-kill cell filter over-selects airborne units

- Rust now: `kill_ground_occupants_at` victim filter (`bridge_orchestrator.rs:1004-1009`):
  `e.position.rx == rx && e.position.ry == ry && !e.is_on_bridge_layer() && e.health.current > 0`.
  `is_on_bridge_layer()` returns `entity.on_bridge` (`game_entity.rs:604-606`). Aircraft keep a cell
  position (`aircraft/mod.rs` reads `entity.position.rx/ry`) and have `on_bridge == false`, so an
  aircraft hovering/passing over the collapse cell matches the filter and is force-killed.
- gamemd: `BlowUpBridge` walks only `CellClass+0xE4` (asm `0047dd84`). Airborne objects are not in
  the cell's ground occupancy object-list; flying `AircraftClass`/in-air units are tracked off the
  cell ground list (the cell ground/`AltObject` lists hold surface and bridge-deck occupants, not
  in-flight aircraft). So a hovering jet/heli over a collapsing bridge cell is never touched.
- Fixture: a Harrier hovers over cell (40,30) (in-air, `on_bridge=false`, `position.rx/ry=(40,30)`)
  while the bridge span there collapses. gamemd: aircraft not in `+0xE4` → untouched. Rust: filter
  `rx==40 && ry==30 && !on_bridge && health>0` matches → Harrier force-killed.
- Player sees: an aircraft flying over a collapsing bridge is instantly destroyed for no reason.
  Triggers when any aircraft overlaps a BlowUpBridge cell at the moment of collapse — rare but
  fully reproducible and very surprising. MED.
- Severity: MED (clearly wrong, low frequency).
- Confidence: PROVEN-DRIFT (binary scope = ground object-list only; Rust scope = any entity at the
  cell not flagged on-bridge, which includes airborne).
- Verify-call: `disassemble_function 0x0047dd70` (`MOV ECX,[ESI + 0xe4]` at `0047dd84` — the ground
  loop reads only the cell's ground object list; no air-layer or altitude test).

---

### D3: Debris/explosion anim Z uses bridge deck level; gamemd computes Z = 0

- Rust now: `blow_up_bridge_cell_fallout → spawn_bridge_debris` sets the metallic/explosion
  `WorldEffect.z = deck_level` where `deck_level = bridge_deck_level_if_any().unwrap_or(c.level)`
  (`bridge_orchestrator.rs:1189-1194`, `:1216`, `:1240`).
- gamemd: `BlowUpBridge` computes the anim height `iStack_4 = (char)this->Level * DAT_0089e7c0 +
  DAT_0089e7b4` (asm `0047de78..0047dea6`). Both `DAT_0089e7c0` and `DAT_0089e7b4` read live as
  `0.0` (`read_memory 0x0089e7c0` / `0x0089e7b4` → all-zero doubles), so the height passed to
  `AnimClass__Constructor` is **0** regardless of bridge level. (X/Y are `coord*0x100+0x80` plus the
  ±25-lepton jitter; only Z is the zeroed term.)
- Fixture: high bridge deck at level 8, collapse cell (40,30). Binary spawns the TWLT0xx /
  MetallicDebris anim at world Z=0; Rust spawns at Z=deck_level (≈ the raised deck height).
- Player sees: bridge collapse explosion/metallic-debris puffs render at a different vertical
  offset than the original (Rust draws them at deck height; gamemd at Z=0). Visible on every high
  bridge collapse that passes the 95% gate. The exact on-screen delta depends on how the render
  layer maps anim Z + bridge fudge; needs an in-engine check to quantify pixels, but the Z input
  differs. LOW–MED.
- Severity: LOW (purely visual vertical offset of the explosion sprite; fires most collapses).
- Confidence: LIKELY-DRIFT (the binary Z term is provably 0 from the two zeroed constants; whether
  the final rendered pixel differs depends on the Rust render path's bridge-fudge handling, which I
  did not trace — hence LIKELY not PROVEN).
- Verify-call: `disassemble_function 0x0047dd70` (`0047de83 IMUL ECX,[0x0089e7c0]`,
  `0047dea0..dea6 MOV/ADD [0x0089e7b4]`) + `read_memory 0x0089e7c0` and `0x0089e7b4` (both 0.0).

---

### D4: Global collapsed-cell queue append (BlowUpBridge step 4) not modelled

- Rust now: `blow_up_bridge_cell_fallout` does kill → DropIn → debris and nothing else per cell
  (`bridge_orchestrator.rs:984-996`). No equivalent of the global cell-coordinate queue append.
- gamemd: `BlowUpBridge` appends the cell's packed coord (`ESI+0x24`) into a fixed-capacity global
  vector at `DAT_0087f8c0` when capacity allows (asm `0047ddd5..0047de2b`; index `DAT_0087f8cc`,
  capacity `DAT_0087f8c4`, with a `vtable+8` grow attempt when full). `get_xrefs_to 0x0087f8c0`
  shows the base pointer is read only inside `BlowUpBridge` in the visible xrefs — it is a
  presentation/collapse-redraw queue drained elsewhere, with no sim-state mutation here.
- Fixture: collapse cell (40,30) — binary pushes coord 0x001E0028 (packed) into the queue at the
  current index; Rust pushes nothing.
- Player sees: no distinct observable beyond redraw, which Rust already covers via
  `bridge_state_changed` → PathGrid rebuild and the radar/terrain-dirty path. No proven separate
  visible effect found. LOW.
- Severity: LOW.
- Confidence: UNCHECKED as a player-visible disparity — the queue's consumer was not decompiled;
  classified as an internal redraw queue. Surfaced for completeness per "no disparity is too small."
- Verify-call: `disassemble_function 0x0047dd70` (`0047de13..0047de2b`), `get_xrefs_to 0x0087f8c0`.

---

## PARITY-CONFIRMED (checked, found matching)

1. **Per-cell fallout order = ground kill, THEN deck DropIn, THEN debris.** Binary asm: ground loop
   `0047dd84..0047ddb8`, deck loop `0047ddba..0047ddd3`, debris block `0047ddd5..0047e02c`. Rust
   `blow_up_bridge_cell_fallout` (`:991-995`) calls `kill_ground_occupants_at` →
   `drop_in_bridge_deck_entities` → `spawn_bridge_debris` in that order. Match.

2. **Deck (bridge-list) occupants are DROPPED, not killed, and survive.** Binary deck loop calls
   vtable `+0xEC` (`= ObjectClass__DropIn`) with NO damage call (`0047ddc9 CALL [EAX + 0xec]`).
   `decompile 0x005F4160` confirms DropIn sets falling bytes `+0x8d/+0x8f=1`, removes from layer,
   clears `OnBridge (+0x8c)=0`, re-submits — no Take_Damage, no drown. Rust
   `drop_in_bridge_deck_entities` (`:1320-1370`) clears `on_bridge`, snaps z to ground level,
   relayers to Ground occupancy, never damages/despawns. Match (units survive collapse).

3. **Next-object snapshot before mutating each occupant.** Binary snapshots `obj+0x30` (the
   `NextObject` link) before the kill/drop call in BOTH loops (`0047dd96`, `0047ddc6`). Rust
   collects victim/snap IDs into a `Vec` first, then mutates (`:1001-1011`, `:1331-1336`) — the
   same "snapshot-then-iterate" effect, immune to list mutation during the walk. Match.

4. **Force-kill uses C4Warhead with damage 0 + force flags.** Binary `0047dd99..0047ddae` pushes
   `Rules+0xfa8` (= C4Warhead), `damage=0`, force flags `1,1`. Rust passes the C4Warhead's
   `InfDeath` via `c4_inf_death` (`:978-982`, `rules.c4_warhead_id()` → warhead `.inf_death`,
   default 1). Match on warhead identity + damage-0 force-kill intent.

5. **C4Warhead InfDeath byte lookup matches combat.** Bridge path:
   `death_sequence_for_inf_death(c4_inf_death)` (`:999-1000`). Combat path: identical
   `death_sequence_for_inf_death(inf_death)` where `inf_death = killing_warhead.inf_death.unwrap_or(1)`
   (`combat/mod.rs:981-992`). Same function (`animation.rs:694-702`): `2→Die2, 3→Die3, 4→Die4,
   5→Die5, else Die1`, clamped `min(5)`. Match.

6. **BlowUpBridge cell set = 4 cells (not 3).** `SetBridgeDirection_NESW/NWSE` with `state=0`
   (`cVar14==0`) call `CellClass__BlowUpBridge` on the anchor, anchor+dir, anchor+2·dir, and the
   opposite (`(dir-4)&7`) cell — four `if (cVar14=='\0') BlowUpBridge(...)` blocks (the third
   forward cell only clears a flag, no BlowUpBridge). Rust `bridge_specs::set_bridge_direction(..,
   set=false)` emits BlowUpBridge for slots 0,1,2,4 = 4 cells (`bridge_specs.rs:474-487`, test
   `set_bridge_direction_destruction_emits_4_blow_up_actions` asserts `== 4`). Match. (The doc's
   "three-cell row/column" wording in §3.4 is loose; live binary + Rust both = 4.)

7. **Outer 95% debris gate threshold is bit-identical.** Binary: pass iff
   `RandomRanged(0,0x7ffffffe) * 2^-31 < 0.95`; `0.95·2^31 = 2040109465.6` → pass iff
   `draw <= 2040109465`. Rust `outer_draw >= 2_040_109_466 → skip` ⇒ pass iff `draw <= 2040109465`
   (`:1182`, const `:373`). Identical at the integer boundary. (Constants verified live:
   `0x007e4f58=0.95`, `0x007e3570=2^-31`.)

8. **MetallicDebris 50% gate threshold is bit-identical.** Binary: pass iff `draw·2^-31 < 0.5`
   (`0x007e1738=0.5`) → `draw <= 0x3FFFFFFF`. Rust `metallic_draw < 0x4000_0000` ⇒ `draw <=
   0x3FFFFFFF` (`:1200`, const `:374`). Match.

9. **Debris RNG draw COUNT and ORDER per cell.** Binary per cell that passes the outer gate:
   (1) outer gate draw, (2) jitter X draw, (3) jitter Y draw, (4) metallic 50% gate draw,
   (5) metallic slot draw *only if gate+debris*, (6) explosion delay `Random(1,5)`,
   (7) explosion slot draw. Rust `spawn_bridge_debris` draws in exactly this order
   (`:1179` outer, `:1187` jitter pair, `:1199` metallic gate, `:1205` metallic slot only inside the
   pass branch, `:1229` delay 1..=5, `:1230` explosion slot). Match — lockstep draw sequence
   preserved. (The metallic slot draw is correctly short-circuited to fire only when both gate and
   non-empty pool hold, matching the binary's `JZ` skip at `0047df61`.)

10. **Jitter offset formula is algebraically the same floor.** Binary:
    `coord = ftol((X·256+128) + ((draw·2^-31) - 0.5)·50.0)`. For positive coords ftol = floor and
    base is integral, so `= base - 25 + floor(draw·50/2^31)`. Rust `bridge_jittered_subcell`
    (`:1304-1308`): `128 + ((draw·50)/2^31 as u64-div) - 25 = base - 25 + floor(draw·50/2^31)`.
    Same value (modulo a sub-ULP float-vs-integer floor edge — see UNCHECKED). The X-base and Y-base
    differ (X uses `+0xc`, Y uses `+0x10`) but both apply the same per-axis jitter; this is a
    visual sub-cell offset on a `WorldEffect`, not hashed.

11. **Map-editor early-out.** Binary bails entirely if `g_IsMapEditor != 0` (`0047dd78..0047dd7e`).
    Not applicable to the running game client (never in editor mode during a skirmish) — no Rust
    gap. TS/editor-only guard, correctly omitted.

12. **DropIn relayer removes while OnBridge==1, re-adds while OnBridge==0.** Binary `DropIn`
    (`0x005F4160`): vtable `+0x124(0)` (exit/remove) precedes `MOV [+0x8c],0`, then `+0x124(1)`
    (enter/add) follows the clear. Rust clears `on_bridge=false` then relayers via
    `occupancy.move_entity(.. Ground ..)` (`:1342`, `:1357-1368`). Net layer transition matches
    (deck→ground, same cell). (Doc §6 flags the exact selected-old-layer removal invariant as
    test-worthy; the observable end state — unit on ground layer, gone from bridge layer — matches.)

---

## UNCHECKED

- **Jitter float-vs-integer floor boundary (sub-ULP).** Rust uses u64 integer division
  `(draw·50)/2^31`; binary uses double `(draw·2^-31)·50` then `-25` then `ftol`. For draw values
  where `draw·50/2^31` is exactly an integer, double rounding could in principle nudge across the
  floor boundary by 1 lepton. Not exhaustively swept; the value is a purely visual sub-cell offset
  on a non-hashed `WorldEffect`, so lockstep is unaffected (draw count is what matters and it
  matches). Flagged per "1-lepton counts," not proven equal at every boundary.

- **D3 final rendered pixel delta.** The binary anim Z input is provably 0; I did not trace the
  Rust render path (bridge fudge / anim Z → screen) to quantify the on-screen pixel difference vs
  gamemd. The *input* Z differs (deck_level vs 0); the rendered magnitude is unverified.

- **D4 collapsed-cell queue consumer.** `DAT_0087f8c0`'s drain site was not decompiled; classified
  as an internal collapse-redraw queue with no observed sim-state effect. If it drives a distinct
  collapse animation/redraw cadence not covered by Rust's PathGrid/radar-dirty refresh, that would
  be a separate visual disparity — not resolved this slot.

- **Exotic non-Techno deck occupants.** The deck loop's vtable `+0xEC` is `DropIn` for normal
  Techno objects; whether any non-Techno object type can occupy `CellClass+0xE8` with a different
  `+0xEC` binding was not exhaustively classified (matches doc OQ-14, deferred). Not player-visible
  for normal units/infantry.
