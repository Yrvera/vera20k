# Adversarial Verdicts — `blowup-fallout` (BlowUpBridge fallout ordering)

Audited against live `gamemd.exe` via Ghidra MCP this session and current Rust
(`src/sim/world/bridge_orchestrator.rs`, `src/sim/combat/mod.rs`, `src/sim/animation.rs`,
`src/sim/game_entity.rs`).

Live re-confirmations this session:
- `decompile_function 0x0047dd70` → resolves to `CellClass__BlowUpBridge` (named in decompile
  header). Body matches finder's reading.
- `disassemble_function 0x0047dd70` → ground loop `0047dd84` `MOV ECX,[ESI+0xe4]` →
  `CALL [EAX+0x16c]` with `PUSH 0,1,1,0,[Rules+0xfa8],0,&Health` (`0047dd99..0047ddae`); deck loop
  `0047ddba` `MOV ECX,[ESI+0xe8]` → `CALL [EAX+0xec]` (DropIn, no damage); queue append
  `0047de13..0047de2b` stores `[ESI+0x24]` into `[DAT_0087f8c0 + idx*4]`.
- `decompile_function 0x005F4160` (`ObjectClass__DropIn`) → sets +0x8d/+0x8f=1, vtable+0x124(0)
  remove, clears OnBridge byte (`param_1+0x23`=0), Submit_Object, vtable+0x124(1) re-add — no
  Take_Damage, no drown.
- `read_memory`: `0x007e4f58=0.95` (`666666666666ee3f`), `0x007e1738=0.5` (`000000000000e03f`),
  `0x007e3570=2^-31` (`000040000000003e`), `0x007e4f50=50.0` (`0000000000004940`),
  `0x0089e7c0=0.0`, `0x0089e7b4=0.0` (both all-zero doubles → anim Z term = `Level*0+0 = 0`).

---

## D1: Ground force-kill skips death-effect pipeline

**VERDICT = REAL** — finder's gamemd reading holds live and the Rust gap is confirmed.

- gamemd (`0x0047dd70`, asm `0047dd84..0047ddb8`): every object in the cell's ground list
  `+0xE4` gets `vtable[+0x16c](&Health, dmg=0, C4Warhead, 0, force=1, 1, 0)` =
  `ObjectClass::Take_Damage` with damage 0 + force flags → guaranteed-kill that runs the full
  death pipeline (death AnimList explosion, wreck/husk, smudge, passenger handling, score).
- Rust (`kill_ground_occupants_at` `bridge_orchestrator.rs:998-1024`): sets `health.current=0`,
  `dying=true`, clears targets/selection, switches death anim if animated. Independently confirmed
  this BYPASSES the death pipeline: the combat death pipeline (`handle_entity_deaths`,
  `combat/mod.rs:804`) is fed only by `dead_entities`, which is built EXCLUSIVELY from combat
  `damage_events` (`combat/mod.rs:2160-2186`, `if target.health.current==0 { dead_entities.push }`).
  Entities killed by `kill_ground_occupants_at` are never in `damage_events`, so they never get
  explosion AnimList / wreck / smudge / die_sound / death-weapon / passenger eject. Animated
  victims play a death anim; non-animated voxel units hit `animation.rs:403-406` (dying + no anim →
  `dying_finished`) and despawn silently with no FX.
- Corrected delta: Rust `health=0/dying=true` bespoke kill (anim-only, no death FX) → gamemd
  `Take_Damage(C4Warhead, dmg=0, force) full death pipeline` (explosion AnimList + wreck/husk +
  smudge + die_sound + death weapon + passenger eject + killer score), per cell, for every
  ground-layer occupant.

## D2: Ground-kill cell filter over-selects airborne units

**VERDICT = REAL** — confirmed live on both sides.

- gamemd ground loop walks ONLY `CellClass+0xE4` (`0047dd84 MOV ECX,[ESI+0xe4]`); no altitude/air
  test. In-flight aircraft (LAYER_TOP / air layer) are not marked down into the cell ground list,
  so an aircraft overflying the collapse cell is never touched.
- Rust `kill_ground_occupants_at` (`bridge_orchestrator.rs:1001-1011`) scans ALL entities
  (`sim.entities.iter_sorted()`), NOT the occupancy grid, filtering only
  `rx==rx && ry==ry && !is_on_bridge_layer() && health>0`. `is_on_bridge_layer()`
  (`game_entity.rs:604-606`) returns only `self.on_bridge` — no air-layer exclusion. The codebase
  HAS the correct exclusion (`occupancy_list_layer()` `game_entity.rs:582-601` returns `None` for
  `MovementLayer::Air|Underground`), but the kill filter does not use it. Aircraft keep a live
  `position.rx/ry` while airborne (used throughout `sim/aircraft/`) and have `on_bridge==false`, so
  an aircraft over a collapsing bridge cell matches the filter and is force-killed.
- Corrected delta: Rust kills any non-bridge-layer entity at the cell incl. air-layer aircraft →
  gamemd kills only ground-list `+0xE4` occupants (never in-flight aircraft). Fix = exclude
  air/underground layer (use `occupancy_list_layer().is_some()` or an `is_airborne` test), not just
  `!on_bridge`.

## D3: Debris/explosion anim Z uses deck level; gamemd Z = 0

**VERDICT = REAL (input proven; rendered-pixel magnitude unverified)** — the binary Z INPUT is
provably 0; the Rust Z INPUT is `deck_level`. They differ.

- gamemd (asm `0047de78 MOVSX ECX,[ESI+0x11b]` (Level), `0047de83 IMUL ECX,[0x0089e7c0]`,
  `0047dea0 MOV EDX,[0x0089e7b4]`, `0047dea6 ADD ECX,EDX`): anim height = `Level*0.0 + 0.0 = 0`.
  Both constants read 0.0 live. (X/Y still get `coord*0x100+0x80` ±25-lepton jitter; only Z is 0.)
- Rust `spawn_bridge_debris` (`bridge_orchestrator.rs:1189-1194`, `z: deck_level` at `:1215` and
  `:1240`) sets metallic + explosion `WorldEffect.z = bridge_deck_level_if_any().unwrap_or(level)`.
- Per CLAUDE.md burden of proof, an unproven-equivalent rendered output defaults to DRIFT. The Z
  input is bit-different on every high-bridge collapse that passes the 95% gate. Whether the final
  on-screen pixel offset differs depends on the Rust render path's anim-Z + bridge-fudge mapping
  (not traced); flagged as render-magnitude UNVERIFIED, but the input is a confirmed disparity.
- Corrected delta: Rust debris/explosion `WorldEffect.z = deck_level` → gamemd anim Z = 0
  (regardless of bridge level).

## D4: Global collapsed-cell queue append not modelled

**VERDICT = UNCERTAIN** — the append is real in the binary; its observable effect is unverified.

- gamemd appends `[ESI+0x24]` (packed `MapCoord`) into `DAT_0087f8c0[DAT_0087f8cc++]` when
  capacity allows (asm `0047ddd5..0047de2b`, with a `vtable+8` grow attempt). Confirmed live.
- Rust models no such queue (`blow_up_bridge_cell_fallout` `:984-996` = kill→DropIn→debris only).
- The queue's consumer/drain site could not be decompiled this session (MCP `get_xrefs_to` /
  `decompile` of the consumer were unavailable — connection dropped). Without the consumer, I
  cannot confirm a player-visible effect distinct from Rust's existing `bridge_state_changed` →
  PathGrid rebuild + radar/terrain-dirty refresh. Stays UNCERTAIN (not REAL, not REFUTED) per the
  "cannot independently confirm → UNCERTAIN" rule. If the consumer drives a distinct
  collapse-redraw/animation cadence, it would become a real visual disparity.

---

## PARITY-CONFIRMED items — spot-checks held

Re-verified the load-bearing ones against the live binary; no misreads found:
- Per-cell order kill(`+0xE4`,Take_Damage) → DropIn(`+0xE8`,vtable+0xec) → debris — matches asm
  loop order `0047dd84` / `0047ddba` / `0047ddd5`; Rust `:991-995` same order. HELD.
- Deck occupants DropIn (survive, no damage/drown) — `DropIn @ 0x005F4160` has no Take_Damage.
  HELD.
- Next-object snapshot before mutate — `MOV EDI,[ECX+0x30]` at `0047dd96`/`0047ddc6`; Rust
  collects ID vec first. HELD.
- C4Warhead dmg=0 force-kill (warhead = `Rules+0xfa8`) — asm `0047dd9b`. HELD.
- Outer 95% gate (`0.95`), metallic 50% gate (`0.5`), jitter span `50.0`, 2^-31 normalizer — all
  four constants read live and match Rust constants (`:373-376`). HELD. Boundary integer math
  matches (`outer >= 2_040_109_466 skip`; `metallic < 0x4000_0000`).
- Debris RNG draw count/order (outer, jitter×2, metallic gate, metallic slot only on pass, delay
  1..5, explosion slot) — matches asm `0047de41..0047e02c` with the `JZ 0047dfbf` short-circuit at
  `0047df61`; Rust `:1179..1230`. HELD.
- Map-editor early-out (`g_IsMapEditor`, asm `0047dd78..0047dd7e`) — correctly omitted (never in
  editor during skirmish). HELD.

---

## NEW disparities the finder may have missed

- **MISS (LOW, sub-D1):** The gamemd ground loop calls `Take_Damage` on EVERY object in `+0xE4`,
  which can include non-Techno `ObjectClass` instances (e.g. a `TerrainClass` tree/object occupying
  the cell) — they get `Take_Damage(C4Warhead, force)` too. Rust's `kill_ground_occupants_at` only
  iterates `EntityStore` game entities and would not destroy a terrain object sitting on the
  collapse cell. Edge case (terrain objects on a bridge-deck/under-bridge cell are unusual), but per
  "no disparity too small" it is a scope mismatch in the kill set. UNCERTAIN on player-visibility;
  flagged for the finder's "exotic non-Techno occupants" UNCHECKED bucket as a kill-side (not just
  deck-side) gap.

- **MISS (verify, sub-D2):** Beyond aircraft, the Rust `!is_on_bridge_layer()` filter would also
  catch any entity using `MovementLayer::Underground` (off the cell ground list in gamemd, same as
  air) if such an entity ever shares the collapse cell with `on_bridge==false`. Underground is
  TS-legacy and not used in YR, so not player-visible — noted only to bound D2's exact fix
  (exclude air AND underground, i.e. gate on `occupancy_list_layer().is_some()`).
