# Tiberium Search Family — Search_For_Tiberium_And_Move (0x004dcfe0) / Scan_For_Tiberium (0x004dd0a0)

Investigation mode: **exhaustive-slice** (two bounded functions plus their direct callees: Is_Cell_Harvestable,
Get_Tiberium_Value, GetCoords, Can_Enter_Cell-gate citation). Read-only Ghidra session (project testProsjekt,
gamemd.exe, image base 0x400000). No renames/comments/labels were written — Ghidra MCP used strictly for
decompile/disassemble/read_memory/xrefs.

Non-scope (deliberately not re-investigated; covered by prior work cited inline): Mission_Harvest's own state
machine, cadence, Rate epilogue, NavCom field identity, Set_Destination_Internal internals, Queue_Mission,
Find_Docking_Bay, SetGhostCell, the weeder-variant scan (0x004ddb90). See
`docs/scans/trace-swarm-20260728/mission-harvest-cadence.md`.

Overall confidence: **HIGH** for both target functions' control flow, filter chain, scoring formula, and
return-value contract (full disassembly read, stack-offset arithmetic verified, vtable slots resolved via
read_memory). MEDIUM for two peripheral fields (own-cell fast-path's singleplayer/shroud gate flag identity,
Can_Enter_Cell's full internal semantics — out of scope, only its return-contract cited).

Active in YR: **Yes** for the whole call chain — both functions are reached from `UnitClass::Mission_Harvest`
(0x0073e5e0), which is the live mission-10 handler in stock YR (verified in the cited prior doc), with no
TS-legacy/dead-flag gating found on either function itself.

## 1. Overview

`Search_For_Tiberium_And_Move` (0x004dcfe0) is a thin **wrapper**: it gates on NavCom, calls
`Scan_For_Tiberium` through the **same vtable slot** (UnitClass vtable+0x338) it is itself sometimes bypassed
for, and — only if the scan found ore **not already under the harvester** — issues one `Set_Destination` call.
`Scan_For_Tiberium` (0x004dd0a0) is the actual **search**: own-cell fast path, then an expanding **square
(Chebyshev) ring** scan up to (but excluding) the caller-given cell radius, filtered per candidate by
`Is_Cell_Harvestable`, scored by `Get_Tiberium_Value` (tiberium-type base value × (density+1)), strict-greater
tie-break. `Scan_For_Tiberium` never calls Set_Destination itself — that's why Mission_Harvest's state-1
full-storage path can call it directly (via the vtable slot) purely to get a coordinate for `SetGhostCell`,
bypassing the wrapper entirely.

## 2. Relationship between the two functions (verified)

`get_function_callers 0x004dcfe0` → only `UnitClass__Mission_Harvest` (2 call sites: 0x0073e864, 0x0073eab9).
`get_function_callers 0x004dd0a0` → **no direct callers** — it is reached exclusively through the vtable, both
(a) internally from inside 0x004dcfe0's own body via `CALL dword ptr [EAX+0x338]` at 0x004dd008, and (b)
externally from Mission_Harvest's state-1 full-storage path via the same slot (per
`mission-harvest-cadence.md` §8, not re-verified this session — out of scope).
`read_memory 0x007f5fa8` (UnitClass vtable 0x007f5c70 + 0x338, 4 bytes) → `a0 d0 4d 00` = **0x004dd0a0**,
confirming the slot resolves to Scan_For_Tiberium (verified via read_memory 0x007f5fa8 + decompile_function
0x004dd0a0).

## 3. Search_For_Tiberium_And_Move (0x004dcfe0) — core logic

Signature (asm-verified, `disassemble_function 0x004dcfe0`): `__thiscall(FootClass* this, int radius,
int unusedFlag)`, `RET 0x8` (cleans 2 stack dwords — 2 explicit params beyond `this`).

```
if (this->NavCom /* +0x5a4, param_1[0x169] */ != 0)
    return (NavCom_raw & 0xFFFFFF00);      // low byte forced 0 by convention below — effectively "not found"

foundCell = vtable[0x338](this, radius, unusedFlag);   // calls Scan_For_Tiberium; retptr reuses caller's
                                                        // own `radius` stack slot as scratch/output (verified
                                                        // via disassemble_function: LEA ECX,[ESP+0x1c] after
                                                        // the PUSH sequence resolves to that slot)
if (foundCell == (0,0))                    // sentinel "nothing found", both halves compared vs
                                            // DAT_008b3d88/DAT_008b3d8a (read_memory 0x008b3d88, len 4 = 0)
    return 0;                              // XOR AL,AL — "not found"

ownCell = CellFromLepton(this->GetCoords());  // vtable+0x48 = ObjectClass::GetCoords (own X/Y/Z, verified
                                               // via decompile_function 0x005f65a0 — a 3-field copy from
                                               // +0x9c/+0xa0/+0xa4, no cell/candidate argument)
if (ownCell == foundCell)
    return 1;                              // MOV AL,1 — already standing on it, no move issued

cellPtr = MapClass::Get_CellClass(foundCell);
Set_Destination(cellPtr, 1);               // vtable+0x480 = 0x00741970 = TechnoClass::Set_Destination
                                            // (verified via read_memory 007f60f0 + decompile_function
                                            // 0x00741970; plate comment already PROOFED under a different
                                            // investigation lane, "chrono-miner-mission-decision")
return 0;                                  // XOR AL,AL — SAME epilogue as the "nothing found" case
```

**Return-value contract (the key, non-obvious finding):** all three of (a) NavCom busy, (b) scan found
nothing, and (c) scan found ore elsewhere **and a new Set_Destination was just issued** converge on the
identical `XOR AL,AL` epilogue at 0x004dd08d. `MOV AL,0x1` at 0x004dd067 is reached **only** by the
already-standing-on-it short-circuit. Verified by disassembly control-flow trace: JNZ 0x004dcfef, JZ
0x004dd029, and the fallthrough after `CALL [EDI+0x480]` at 0x004dd086 all land on 0x004dd08c/0x004dd08d.
The function's "found" bit therefore means **"no travel needed, ore is already under me"**, not **"ore was
located."** This resolves cleanly against Mission_Harvest's own branch (already verified, not re-derived):
`!found && NavCom!=0` → "still driving, wait" vs `!found && NavCom==0` → "genuinely no ore, go idle" — the
caller relies on NavCom (freshly set by the Set_Destination call inside this very function) to distinguish
"driving to newly-found ore" from "truly nothing found," rather than trusting the boolean alone.

**The 3rd parameter ("no-archive flag" per the prior doc's framing) is dead inside this function.** Verified
by exhaustive disassembly scan: it is loaded (`MOV ECX,[ESP+0x1c]`), pushed once to forward to
Scan_For_Tiberium, and never read, tested, or branched on anywhere else in the 0x004dcfe0 body. Call-site
values confirmed via `get_assembly_context 0x0073e864,0x0073eab9`: state-1's re-search call pushes the literal
constant `0` for this argument; state-0's call pushes a value loaded from a Mission_Harvest stack slot whose
origin is outside this slot's scope (Mission_Harvest internals, already covered elsewhere).

## 4. Scan_For_Tiberium (0x004dd0a0) — core logic

Signature (asm-verified): `__thiscall(FootClass* this, int radius, int unusedArg3)`, hidden return pointer as
an implicit 1st stack arg; `RET 0xc` (cleans 3 stack dwords: retptr + radius + the 3rd arg). The 3rd argument
is **never read** anywhere in this function's body either (exhaustive scan of every `[ESP+K]` reference in the
full disassembly — no reference to the 3rd arg's stack slot at any code depth).

```
ownCoord = this->GetCoords();                 // vtable+0x48, same as above
ownCell  = CellFromLepton(ownCoord);          // >>8 with sign-adjust (ADD 0xFF if negative, then SAR 8)
cell = MapClass::Get_CellClass(ownCell);
if (cell->LandType == 5) {                    // Tiberium land type — no Is_Cell_Harvestable call here;
    *retptr = ownCell;                        // ring-0 is UNFILTERED (no zone/occupancy/shroud check)
    return;
}

if (radius <= 1) { *retptr = <uninitialized local>; return; }   // dead/garbage-return edge case — see §8

best = -1; bestCell = undefined;
for (ring = 1; ring < radius; ring++) {       // exclusive upper bound: ring runs 1 .. radius-1 inclusive,
                                               // radius itself is NEVER scanned (verified: post-increment
                                               // `CMP EBP,radius; JL` re-enters loop only while EBP<radius)
    for (offset = -ring; offset <= ring; offset++) {
        candidates = [ (ownX+offset, ownY-ring),   // top row
                       (ownX+offset, ownY+ring),   // bottom row
                       (ownX-ring,   ownY+offset), // left column
                       (ownX+ring,   ownY+offset) ]; // right column
        // NOTE: corners (offset == ±ring) are visited twice (once by a row arm, once by a column
        // arm) — harmless because of strict tie-break, but Is_Cell_Harvestable/Get_Tiberium_Value
        // run twice for those cells (verified: same offset range -ring..=ring drives both loops).
        for cand in candidates {
            if (!Is_Cell_Harvestable(this, cand)) continue;
            v = Get_Tiberium_Value(MapClass::Get_CellClass(cand));
            if (v > best) { best = v; bestCell = cand; }   // STRICT >, first-seen wins ties
        }
    }
    if (best != -1) break;    // ring-level early exit — nearer ring always wins over any farther ring
}
*retptr = (best != -1) ? bestCell : (0,0);   // sentinel on total failure
```

This is a **square/Chebyshev ring**, not a diamond (Manhattan) ring: at ring distance `r`, all four arms sweep
the perpendicular axis over the full `[-r, r]` range at the fixed axis-aligned offset `±r`, i.e. the entire
border of a `(2r+1)×(2r+1)` box. Verified via disassemble_function 0x004dd0a0, arithmetic at
0x004dd147–0x004dd2ac (LEA constructions building `(±r, offset)` and `(offset, ±r)` pairs) plus the decompile
cross-check (`decompile_function 0x004dd0a0`).

## 5. Is_Cell_Harvestable (0x004dce80) — the filter chain

`decompile_function 0x004dce80`, signature `__thiscall(FootClass* this, short* cellCoord)`. Gate order,
short-circuiting, AL=0 on any failure:

1. `MapClass__Is_Cell_In_Playfield(cellCoord)` — must be true (bounds check).
2. **Conditional** (singleplayer only): if `g_GameMode==0` AND `this[+0x41a]` byte is set, the candidate's
   center coordinate must not be shrouded (`IsShrouded`). Gated behind a campaign-only per-unit byte whose
   INI key was not traced this session (out of scope) — flagged Active in YR: **Conditional** (only fires for
   the subset of units/scenarios with that flag set, in singleplayer).
3. `MapClass__Can_Reach_Zone(0, cellCoord, this->MovementZone /* +0x5b4 off a resolved pointer */, zoneArg,
   0, 0)` — the unit's own movement-zone reachability test (the "passability/zone" gate).
4. `MapClass::Get_CellClass(cellCoord)->LandType == 5` (Tiberium) — must match; same LandType constant as the
   ring-0 fast path and as Mission_Harvest's own tick check (cross-verified against
   `mission-harvest-cadence.md`'s already-established `[cell+0xEC]!=5` usage).
5. `vtable+0x1ac(cellPtr, -1, -1, 0, 1) == 0` — occupancy/passability gate. Resolved via
   `read_memory 0x007f5e1c` (UnitClass vtable+0x1ac) → `0x0073f0a0` = `UnitClass::Can_Enter_Cell`
   (decompile_function 0x0073f0a0; function already carries a prior PROOFED plate comment: "Central
   passability check, returns 0-7… 0=OK/Clear"). Is_Cell_Harvestable requires the **exact** return value 0
   (fully clear) — not merely "passable-with-a-cost."

Only when all five pass does Is_Cell_Harvestable return true. **The ring-0 fast path in §4 skips this entire
chain** — it only checks LandType==5, not zone/occupancy/shroud — a real asymmetry between "am I already on
ore" and "is that other cell harvestable."

## 6. Get_Tiberium_Value (0x00485020) — scoring formula

`decompile_function 0x00485020`, `__fastcall(CellClass* cell)`:

```
idx = CellClass::OverlayToTiberiumIndex(cell);   // maps the cell's overlay to a TiberiumClass index
if (idx == -1) return 0;
return TiberiumClassArray[idx]->Value /* +0xb8 */ * (cell->densityByte /* +0x11e */ + 1);
```

Confirms: value = per-tiberium-type `Value=` INI field × (growth-stage/density byte + 1). No RNG. No other
terms (no distance falloff, no per-house scaling) inside this function.

## 7. INI keys (not re-derived this session — cited from prior verified work)

- `rules+0x1778` = TiberiumShortScan (stock rulesmd.ini 311: 6 cells) — verified prior session, re-confirmed
  this session via `get_assembly_context 0x0073eab9` (state-1 callsite reads `[EAX+0x1778]`, `<<8` at use).
- `rules+0x177c` = TiberiumLongScan (stock rulesmd.ini 312: 48 cells) — re-confirmed via
  `get_assembly_context 0x0073e864` (`[EDX+0x177c]`).
- `TiberiumClass+0xb8` = per-type `Value=` field — cited from decompile_function 0x00485020, not re-derived
  from RulesClass ReadINI this session (out of scope; the offset is load-bearing for the scoring formula so
  it is asm-verified at the *use* site here, even though its ReadINI store site was not walked).

## 8. Integration points / callers

- `UnitClass::Mission_Harvest` (0x0073e5e0) calls `Search_For_Tiberium_And_Move` at 0x0073e864 (state 0,
  radius=TiberiumLongScan/256, 3rd arg = non-literal stack value) and 0x0073eab9 (state 1 re-search,
  radius=TiberiumShortScan/256, 3rd arg = literal 0).
- Mission_Harvest's state-1 full-storage path calls `Scan_For_Tiberium` directly through vtable+0x338 (not
  through the wrapper) with TiberiumShortScan/256, to obtain a coordinate for SetGhostCell — not
  re-verified this session (covered in `mission-harvest-cadence.md` §8, marked UNCHECKED there for internals,
  now resolved for `Scan_For_Tiberium`'s own contract by this report).
- `Search_For_Tiberium_And_Move` → `Scan_For_Tiberium` (same vtable slot, internal call) → `Is_Cell_Harvestable`
  → `Can_Enter_Cell` (vtable+0x1ac) / `Can_Reach_Zone` / `Get_CellClass`; → `Get_Tiberium_Value`.
- `Search_For_Tiberium_And_Move` → `Set_Destination` (vtable+0x480, 0x00741970) → (eventually)
  `FootClass::Set_Destination_Internal` (0x004d9510/0x004d96bc, out of scope, writes NavCom) — chain
  confirmed only at the first hop (vtable+0x480 resolved and decompiled); the rest is cited from prior work.

## 9. Current Rust status (src/sim/miner/miner_system.rs)

`search_local_ore` (line 1453) — **matches the binary closely, better than the parent brief assumed**:

- Ring geometry: `for col in -ring..=ring { arms = [(cx+col,cy-ring),(cx+col,cy+ring),(cx-ring,cy+col),
  (cx+ring,cy+col)] }` — this is the **same square/Chebyshev ring** as gamemd, not a diamond, despite the
  code comment above it saying "diamond perimeter." **The comment is stale/mislabeled; the code is correct.**
- Ring bound: `for ring in 1..radius_i` (Rust exclusive range) — already matches gamemd's exclusive
  `ring < radius` bound exactly (radius itself never scanned).
- Scoring: `base * (remaining + 1)` — matches `Value * (density+1)` exactly.
- Tie-break: strict `value <= cur` skip (first-seen wins) — matches gamemd's strict `>` update.
- Ring-0 fast path: unfiltered node lookup — matches gamemd's unfiltered LandType==5 check.
- Corner double-visit: Rust's comment explicitly notes and accepts it, matching gamemd's actual behavior.
- Filters: `build_scan_filter` combines zone reachability + occupancy (`is_cell_path_clear_for_scan`,
  line 442) — a reasonable analog of Can_Reach_Zone + Can_Enter_Cell==0, though it degrades to *no* filter at
  all when the filter is unavailable (`filter_ref` is `None`), whereas gamemd's chain always runs (playfield,
  optional shroud, zone, landtype, occupancy) for every non-ring-0 candidate. The singleplayer-only shroud gate
  (§5 item 2) is not modeled — likely low priority (campaign-only, TS-legacy-adjacent).

**Real deltas:**

1. **No "found = already there" contract.** `search_local_ore` returns the target cell as `Option<(u16,u16)>`
   with no notion of "already standing on it, no move needed" vs "found, but a move is required." The caller
   (`handle_search_ore` etc.) issues its own Drive command uniformly. Whether the surrounding FSM correctly
   distinguishes "just arrived, start harvesting" from "still travelling" by other means (state transitions,
   arrival detection) is plausible but was not traced this session (Mission_Harvest cadence is out of scope
   here) — flagged as a verification gap, not a confirmed bug.
2. **Added whole-map fallback.** `handle_search_ore` falls through to `pick_best_resource_node` (global
   nearest-reachable-ore search) when the bounded ring scan finds nothing (line 533). Gamemd's
   `Scan_For_Tiberium`/`Search_For_Tiberium_And_Move` have **no such fallback** — if nothing is found within
   `radius-1` rings, the function returns "not found," full stop, and Mission_Harvest goes to the no-ore idle
   state (105-frame retry). This is a genuine behavioral divergence: Rust miners can find and drive to ore
   arbitrarily far away that gamemd would never look for from that function.
3. **The unused 3rd parameter** ("no-archive flag") has no equivalent in Rust and needs none — it does
   nothing in either binary function, so there's nothing to port.

## 10. Coverage Ledger

| Area | Status | Evidence | What remains |
|---|---|---|---|
| 0x004dcfe0 control flow / return contract | verified | disassemble_function 0x004dcfe0, decompile_function 0x004dcfe0 | none |
| 0x004dd0a0 control flow / ring geometry / bound | verified | disassemble_function 0x004dd0a0, decompile_function 0x004dd0a0 | none |
| vtable+0x338, +0x48, +0x480, +0x1ac resolution | verified | read_memory 0x007f5fa8/0x007f5cb8/0x007f60f0/0x007f5e1c + decompile of each target | none |
| Is_Cell_Harvestable filter chain | verified | decompile_function 0x004dce80 | shroud-gate INI key + `this+0x41a` byte's own key untraced (out of scope) |
| Get_Tiberium_Value formula | verified | decompile_function 0x00485020 | TiberiumClass+0xb8 ReadINI store site untraced (cited from use, not from parse) |
| 3rd parameter ("no-archive flag") liveness | verified-dead | disassemble_function 0x004dcfe0 + 0x004dd0a0 (exhaustive offset scan) | state-0 callsite's stack-value origin (Mission_Harvest-internal, out of scope) |
| Sentinel (0,0) "not found" value | verified | read_memory 0x008b3d88 (4 bytes = 0) | none |
| Set_Destination argument shape | touched-not-exhausted | decompile_function 0x00741970 (huge function, only entry/identity confirmed) | full internals out of scope by design |
| Rust `search_local_ore` comparison | verified | Read miner_system.rs:1453-1530 | none — direct line-by-line compare done |
| Rust filter wiring (`build_scan_filter`) | touched-not-exhausted | Read miner_system.rs:442-488 | did not trace zone-reachability's own correctness, only that it exists |

## 11. Open Questions — final log

- `[RESOLVED]` OQ1 — Does 0x004dcfe0 call 0x004dd0a0 via the vtable or directly? Via vtable+0x338, same slot
  external callers use (evidence: disassemble_function 0x004dcfe0 @0x004dd008; read_memory 0x007f5fa8).
- `[RESOLVED]` OQ2 — Ring shape: diamond or square? Square/Chebyshev (evidence: disassemble_function
  0x004dd0a0 @0x004dd147-0x004dd2ac).
- `[RESOLVED]` OQ3 — Ring bound inclusive or exclusive of radius? Exclusive — radius itself never scanned
  (evidence: disassemble_function 0x004dd0a0 @0x004dd2be-0x004dd2ca).
- `[RESOLVED]` OQ4 — Meaning of the 3rd parameter ("no-archive flag")? Dead/unread inside both target
  functions (evidence: exhaustive disassembly offset scan of both functions).
- `[RESOLVED]` OQ5 — Return-value contract of 0x004dcfe0? True only if already standing on found ore; false
  for "not found" AND for "found elsewhere + Set_Destination issued" (evidence: disassemble_function
  0x004dcfe0, control-flow trace to the shared epilogue at 0x004dd08c-0x004dd08d).
- `[RESOLVED]` OQ6 — Does Scan_For_Tiberium ever call Set_Destination itself? No — confirmed by full
  disassembly of 0x004dd0a0 (no vtable+0x480 call present).
- `[RESOLVED]` OQ7 — RNG consumption inside either function? None — no RandomRanged/RNG-instance calls appear
  in either disassembly.
- `[RESOLVED]` OQ8 — Filter chain gating candidates? Playfield bounds, conditional shroud (singleplayer +
  per-unit flag), zone reachability (Can_Reach_Zone), LandType==5, occupancy (Can_Enter_Cell==0) (evidence:
  decompile_function 0x004dce80, read_memory 0x007f5e1c + decompile_function 0x0073f0a0).
- `[RESOLVED]` OQ9 — Scoring formula? `Value(tiberium type) * (density+1)`, no RNG, no distance term
  (evidence: decompile_function 0x00485020).
- `[DEFERRED]` OQ10 — Radius<=1 edge case returns an uninitialized/garbage stack value as the found cell
  (evidence: disassemble_function 0x004dd0a0 @0x004dd121-0x004dd123, @0x004dd2d1-0x004dd2db path taken with
  the loop body never executed). Category: bounded-cost-too-high / low real-world relevance — stock
  TiberiumShortScan=6 and TiberiumLongScan=48 (cells) are both well above 1, so this only matters for a
  pathological mod setting a scan value under ~2 cells (512 leptons). Next step if pursued: confirm whether
  any caller ever passes such a small radius before treating this as dead code.
- `[DEFERRED]` OQ11 — Exact identity/INI key of the singleplayer-only shroud-check gate (`this+0x41a` byte).
  Category: out-of-scope (belongs to a general FootClass/campaign-AI flag investigation, not this slice).
  Next step: search_strings for candidate INI key names cross-referenced with FootClass constructor writes to
  +0x41a.
- `[DEFERRED]` OQ12 — Origin of the non-literal 3rd-argument value at Mission_Harvest's state-0 callsite
  (0x0073e84a, `MOV ECX,[ESP+0x14]`). Category: out-of-scope — belongs to Mission_Harvest's own state-0
  preamble, already the subject of a separate, larger investigation (`mission-harvest-cadence.md`). Moot for
  behavior anyway since OQ4 shows the value is never read by either target function.
- `[DEFERRED]` OQ13 — Full internals of `Set_Destination` (0x00741970, vtable+0x480) and its downstream call
  into `Set_Destination_Internal`. Category: out-of-scope by task constraint (explicitly listed as
  already-settled elsewhere / non-goal for this slice).

No `[OPEN]` entries remain.

## Negative Facts / Do Not Do

- Do not model 0x004dcfe0's boolean return as "ore was found" in any Rust port that tries to replicate its
  exact contract — it means "no travel needed" (verified via disassembly control-flow trace, §3).
- Do not port the 3rd parameter as a meaningful flag — it is provably dead in both functions (verified via
  exhaustive disassembly offset scan, §3-4).
- Do not model the ring scan as Manhattan/diamond distance — it is Chebyshev/square (verified via
  disassemble_function 0x004dd0a0, §4). (Rust's code is already correct here; only its comment is wrong.)
- Do not assume Scan_For_Tiberium's ring-0 fast path applies the same filter chain as rings 1+ — it does not
  (LandType==5 only, no zone/occupancy/shroud check) (verified via decompile_function 0x004dd0a0 vs
  0x004dce80, §4-5).
- Do not add a whole-map fallback search inside a from-scratch reimplementation of these two functions and
  call it "parity" — gamemd has none; Rust's existing `pick_best_resource_node` fallback (miner_system.rs:533)
  is an intentional-or-not deviation, not a gamemd behavior (verified via full disassembly of both functions —
  no calls beyond the bounded ring loop and the two helpers cited in §4-5).

## Remaining Uncertainty

- Whether Rust's overall Mission_Harvest-equivalent FSM correctly distinguishes "just arrived, start
  harvesting now" from "still driving toward newly found ore" given that `search_local_ore` doesn't surface
  gamemd's "already there" bit explicitly — plausible by construction (arrival is presumably detected by
  position/state elsewhere) but not traced this session; out of this slice's scope.
- The `this+0x41a` shrouded-check gate's INI key/purpose (OQ11) and the radius<=1 edge case's real-world
  reachability (OQ10) — both deferred, both low-impact for stock content.
- `Can_Reach_Zone`'s and `Can_Enter_Cell`'s own internal correctness were not re-verified (cited only for
  their call shape / return contract at the point Is_Cell_Harvestable uses them); `Can_Enter_Cell` already
  carries a prior PROOFED plate comment from a different investigation lane.

## Implementation Handoff

| # | Verified behavior | Binary evidence | Rust delta | Affected surface | Acceptance scenario | Proposed test name | Risk |
|---|---|---|---|---|---|---|---|
| 1 | 0x004dcfe0's "found" return is true only when the harvester is already standing on the discovered ore cell; false covers both "nothing found" and "found + Set_Destination issued" | disassemble_function 0x004dcfe0 @0x004dcfef/0x004dd029/0x004dd08c-0x004dd08d | `search_local_ore` has no equivalent tri-state signal; caller FSM must infer "arrived vs still driving" some other way | `src/sim/miner/miner_system.rs::handle_search_ore` and the state transition into harvesting | A miner scanning finds ore 3 cells away: the FSM should NOT enter a "harvesting" state until actual arrival, only "moving to ore" | `miner_search_does_not_harvest_before_arrival` | compounding — silent one-frame state skip would look correct in most traces but break edge timing under load |
| 2 | gamemd's bounded ring scan (both target functions) has no whole-map fallback; failure past radius-1 rings is a hard "not found" | full disassembly of 0x004dcfe0 and 0x004dd0a0 (no calls beyond the ring loop, Is_Cell_Harvestable, Get_Tiberium_Value, and the two Set_Destination-path helpers) | `handle_search_ore` (miner_system.rs:533) adds a `pick_best_resource_node` global fallback not present in gamemd | `src/sim/miner/miner_system.rs::handle_search_ore` | A map with ore only outside TiberiumLongScan radius from every miner: gamemd miners go idle (105-frame retry loop); confirm whether Rust intends to keep the global fallback as a deliberate design choice or gate it behind a flag | `miner_no_fallback_beyond_scan_radius_matches_gamemd` (or explicit doc note if the fallback is an intentional non-parity feature) | milestone-blocking if parity is the goal for this exact mechanic; harmless if the fallback is a deliberate gameplay-quality choice — needs a product decision, not just a code fix |
| 3 | Ring-0 (own cell) is checked with LandType==5 only, no zone/occupancy/shroud filter; rings 1+ go through the full 5-gate Is_Cell_Harvestable chain | decompile_function 0x004dd0a0 (fast path) vs decompile_function 0x004dce80 (ring path) | Rust's ring-0 fast path (miner_system.rs:1472-1476) is already unfiltered — matches. No action needed, but worth a regression test to keep it that way as filters evolve | `src/sim/miner/miner_system.rs::search_local_ore` | A miner standing on a cell with ore but zone-unreachable-from-elsewhere should still harvest it immediately (ring-0 bypass) | `miner_ring0_bypasses_zone_and_occupancy_filter` | exactification-residual — low risk, but a future "improve filter coverage" pass could accidentally apply the ring-1+ filter to ring-0 and silently change behavior |

## Sources

- Decompiled: 0x004dcfe0, 0x004dd0a0, 0x004dce80, 0x00485020, 0x005f65a0 (ObjectClass::GetCoords),
  0x00741970 (TechnoClass::Set_Destination, entry-identity only), 0x0073f0a0 (UnitClass::Can_Enter_Cell,
  entry-identity + return-contract only).
- Disassembled (full): 0x004dcfe0, 0x004dd0a0.
- read_memory: 0x007f5fa8 (UnitClass vtable+0x338), 0x007f60f0 (+0x480), 0x007f5cb8 (+0x48), 0x007f5e1c
  (+0x1ac), 0x008b3d88 (sentinel constant, 4 bytes).
- get_function_callers: 0x004dcfe0, 0x004dd0a0.
- get_xrefs_to: 0x004dcfe0.
- get_assembly_context: 0x0073e864, 0x0073eab9 (Mission_Harvest callsites).
- Docs referenced: `docs/scans/trace-swarm-20260728/mission-harvest-cadence.md` (prior session, cited for
  Mission_Harvest's own state machine, NavCom identity, and INI key bindings; not re-derived here).
- Rust: `src/sim/miner/miner_system.rs` (search_local_ore ~1453-1530; handle_search_ore ~462-551;
  is_cell_path_clear_for_scan ~442-460), read at
  `C:\Users\enok\Documents\ra2-rust-game\.claude\worktrees\quirky-brattain-fe0387\src\sim\miner\miner_system.rs`.
