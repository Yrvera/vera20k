# Chrono Miner Set_Destination Teleport-vs-Drive Gate — Ghidra Research Report

**Address(es):** `0x0073E5E0` (`UnitClass::Mission_Harvest`), `0x00741970` (`TechnoClass::Set_Destination`,
active UnitClass vtable+0x480 slot), `0x004D94B0` (`FootClass::Set_Destination_Internal`),
`0x0065AD30` (`FootClass::GetDestination`), `0x0047EBA0` (`CellClass::FindFirstUnit`, mislabeled
`FindFirstBuilding` in prior docs)

**Investigation Mode:** exhaustive-slice, read-only (Ghidra MCP, gamemd.exe, `testProsjekt`,
image base `0x400000`)

**Claimed Scope:** the exact boolean predicate in `TechnoClass::Set_Destination` (0x741970) that
decides whether a chrono miner (`CMIN`, `Teleporter=yes`) warps or drives to a new destination,
specifically as reached from `UnitClass::Mission_Harvest` (0x73E5E0). Does NOT re-derive the warp
visual/animation pipeline or the unload state machine.

**Trigger:** 2026-07-12 audit of `CHRONO_MINER_SYSTEM_OVERVIEW.md` §3 flagged the teleport-vs-drive
gate `UNVERIFIED-pending-reinvestigate` after finding `CellClass::FindFirstUnit` was mislabeled
`FindFirstBuilding` in `CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md` §14/§21.

---

## 1. Verified Predicate (asm-read, HIGH confidence)

All addresses below verified via `disassemble_function 0x00741970` (raw CMP/TEST/Jcc read
directly, not paraphrased decompile) plus `get_function_by_address` / `read_memory` for callee
and CLSID identity.

### 1.1 Outer gate — is this call eligible for the Teleporter block at all?

```
007423cd: MOV ECX,[EBP+0x6c4]        ; ECX = this->TechnoTypeClass
007423d3: MOV AL,[ECX+0xcd4]         ; AL = TechnoTypeClass+0xCD4 (Teleporter=)
007423d9: TEST AL,AL
007423db: JZ 0x007427c0              ; Teleporter==0 -> skip whole block
007423e1: MOV AL,[EBP+0x27c]         ; this+0x27C (ChronoInTransit)
007423e7: TEST AL,AL
007423e9: JNZ 0x007427c0             ; ChronoInTransit!=0 -> skip
007423ef: MOV EAX,[EBP+0x2b0]        ; this+0x2B0
007423f5: TEST EAX,EAX
007423f7: JNZ 0x007427c0             ; !=0 -> skip
007423fd: MOV AL,[EBP+0x6ad]         ; this+0x6AD (deploying)
00742403: TEST AL,AL
00742405: JNZ 0x007427c0             ; deploying!=0 -> skip
```

Gate: `Teleporter=yes AND ChronoInTransit==0 AND this+0x2B0==0 AND this+0x6AD==0`. Matches the
pre-existing claim in `CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md` §14 — re-verified fresh this
session via `disassemble_function 0x00741970`, not just trusted from the prior doc.

### 1.2 Fetch old destination (NavCom) and active-locomotor CLSID

```
0074240b: PUSH 0x0
0074240d: MOV ECX,EBP
0074240f: CALL 0x0065ad30            ; EDI = FootClass::GetDestination(this, 0)
00742414: LEA EBX,[EBP+0x674]        ; EBX = &this->active_locomotor
...
00742462: CALL [EDX+0xc]             ; GetClassID(active_locomotor) -> CLSID buffer at [ESP+0x60]
00742465: TEST EDI,EDI               ; EDI = old destination (NavCom), NULL?
00742467: MOV byte ptr [ESP+0x14],0x1  ; DEFAULT: need_drive_piggyback = TRUE
0074246c: JZ 0x007425e6              ; old dest NULL -> straight to Drive-vs-active-CLSID check
```

`FootClass::GetDestination(this, 0)` (`0x0065AD30`, `decompile_function 0x0065ad30`) is
`*(AbstractClass**)(*(int*)(this+0xE4) + 0*4)` — one level of indirection through a pointer table
at `this+0xE4` whose slot 0 holds `&this->NavCom`; net effect is `NavCom` (`FootClass+0x5A4`),
matching `NAVCOM_LIFECYCLE_GHIDRA_REPORT.md` §1-2 ("NavCom (+0x5A4 on every FootClass)").
`param_1[0x169]` in the decompile (`0x169*4 = 0x5A4`) is the *same field*.

**The flag byte at `[ESP+0x14]` defaults to 1 ("prefer Drive") and is only overwritten to 0
("prefer/keep Teleport") by the RTTI/flag/occupancy chain below.**

### 1.3 The RTTI/flag/occupancy chain (only reached when old dest != NULL)

```
00742472: MOV EDX,[EDI]  MOV ECX,EDI  CALL [EDX+0x2c]   ; old_dest->What_Am_I()
00742479: CMP EAX,0x6
0074247c: JNZ 0x007425db             ; old_dest RTTI != BuildingClass(6) -> merge, flag stays 1
...                                    ; (branchless) ESI = old_dest if RTTI==6 else 0
00742496: MOV EDI,[ESP+0x94]         ; EDI = param_2 = new destination argument
0074249d: TEST EDI,EDI
0074249f: JNZ 0x007424a5
007424a1: XOR EAX,EAX                ; new_dest NULL -> EAX=0
007424a3: JMP 0x007424b7
007424a5: ... CALL [EDX+0x2c]        ; new_dest->What_Am_I()
007424ac: SUB EAX,0xb  NEG  SBB  NOT  AND EAX,EDI   ; EAX = new_dest if RTTI==0xB(CellClass) else 0
007424b7: MOV ECX,[ESI+0x520]        ; ECX = old_dest->TypeClass  (old_dest is BuildingClass here)
007424bd: MOV DL,[ECX+0x16b3]        ; DL = BuildingTypeClass+0x16B3 (DockUnload=)
007424c3: TEST DL,DL
007424c5: JZ 0x007425db              ; DockUnload==0 -> merge, flag stays 1
007424cb: TEST EAX,EAX
007424cd: JZ 0x007425db              ; new_dest not a valid CellClass -> merge, flag stays 1
007424d3: PUSH 0x0  MOV ECX,EAX
007424d7: CALL 0x0047eba0            ; CellClass::FindFirstUnit(new_dest_cell, list=0)
007424dc: TEST EAX,EAX
007424de: JNZ 0x007425db             ; a UNIT already occupies the dest cell -> merge, flag stays 1
007424e4: MOV ECX,0x4  MOV EDI,0x7e9a90  ; CLSID_TeleportLocomotion
007424ee: LEA ESI,[ESP+0x60]         ; queried CLSID of the currently-active locomotor
007424f4: CMPSD.REPE ES:EDI,ESI      ; 16-byte GUID compare
007424f6: MOV byte ptr [ESP+0x14],AL ; AL is a leftover 0 from the FindFirstUnit==NULL check above
                                      ;   -> this instruction sets the flag to 0 unconditionally
                                      ;   once all four conditions above have passed
007424fa: JZ 0x007425db              ; active CLSID == Teleport -> merge directly
00742500: ...                        ; active CLSID != Teleport -> attempt End_Piggyback (see 1.4)
```

`CellClass::FindFirstUnit` (`0x0047EBA0`) verified via `decompile_function 0x0047eba0`: iterates
the cell's occupant list at `cell+0xE4` (or `+0xE8` when the selector arg is nonzero) and returns
the first occupant whose `What_Am_I()==1`. RTTI `1` is `UnitClass`
(`0x00746E20`, cross-corroborated in `CELL_OCCUPANCY_ORDERING_FOLLOWUP_GHIDRA_REPORT.md` and
`TERRAIN_CLASS_GHIDRA_REPORT.md` §21.2, both citing direct decompiles of every `What_Am_I`).
**The current Ghidra label `CellClass__FindFirstUnit` is correct; the prior docs' "FindFirstBuilding"
identity for this address was wrong** (RTTI_LABEL_DRIFT, now corrected).

`BuildingClass::What_Am_I` returns `6` (`0x00459EC0`, `CELL_OCCUPANCY_ORDERING_FOLLOWUP_GHIDRA_REPORT.md`).
`BuildingTypeClass+0x16B3` = `DockUnload=` (`BUILDINGTYPE_FLAGS_16B3_16BB_16BC_REFINERY_WEEDER_GHIDRA_REPORT.md`
§2-4, parser `0x0045FE50`; stock `[GAREFN]`/`[NAREFN]` both set `DockUnload=yes`,
`ini/rulesmd.ini:11726` and `:12519`). CellClass RTTI `0xB` corroborated via
`NAVCOM_LIFECYCLE_GHIDRA_REPORT.md` §3.1/§5.1 (`NavCom->RTTI == 0xB /* CellClass */`) — this
**refutes** an unrelated stale claim in `TECHNOCLASS_SET_DESTINATION_GHIDRA_REPORT.md`
("`FootClass__GetDestination(0)` RTTI == 0xF (CellClass)") which conflicts with the
independently double-verified fact that RTTI `0xF` is `InfantryClass`
(`0x00523340`, two independent docs). That contradiction is a different call site in the same
function (Stage 3, `~0x741ad1`) and is **out of this report's scope** — flagged for a separate
follow-up, not corrected here.

CLSID_TeleportLocomotion at `0x007e9a90` verified via `read_memory` = bytes
`47 27 58 4A 39 98 D1 11 B7 09 00 A0 24 DD AF D1` = `{4A582747-9839-11D1-B709-00A024DDAFD1}`,
matching `[CMIN] Locomotor=` in `ini/rulesmd.ini:7398` and `[CMIN] Teleporter=yes` at
`ini/rulesmd.ini:7396`. CLSID_DriveLocomotion at `0x007e9a30` = `{4A582741-9839-11D1-B709-00A024DDAFD1}`.

### 1.4 The merge point and the two outcomes

```
007425db: CMP byte ptr [ESP+0x14],0x1
007425e0: JNZ 0x007427b2             ; flag==0 -> SKIP Drive-piggyback creation entirely
007425e6: MOV ECX,0x4  MOV EDI,0x7e9a30  ; CLSID_DriveLocomotion
007425f6: CMPSD.REPE ES:EDI,ESI      ; compare active CLSID vs Drive
007425f8: JZ 0x007427b2              ; already Drive -> skip (nothing to change)
007425fe: ...                        ; NOT Drive -> CoCreateInstance(CLSID_DriveLocomotion),
                                      ;   IPiggyback-attach the old active locomotor under it,
                                      ;   set this->active_locomotor = new Drive instance
007427b2: ...                        ; merge: Release() the queried interface, continue normal flow
```

**Two outcomes, verified exactly:**

- **flag == 1 (default, or any of the four gating conditions in §1.3 failed):** if the active
  locomotor is not already `DriveLocomotionClass`, a new Drive instance is created, the current
  active locomotor (typically Teleport) is piggybacked underneath it, and Drive becomes active.
  `FootClass::Set_Destination_Internal` then calls `Head_To_Coord` on **Drive** — the unit drives.
- **flag == 0 (old destination was a `DockUnload=yes` building AND new destination is an empty,
  unit-free `CellClass`):** Drive-piggyback creation is skipped. If the active locomotor is
  already `TeleportLocomotionClass`, it stays active and `Head_To_Coord` fires on **Teleport** —
  the unit warps. If the active locomotor is *not* Teleport (e.g. still mid-Drive-piggyback from
  a previous order), the code at `0x742500-0x7425cd` attempts `IPiggyback::End_Piggyback`
  immediately: if the piggybacked Drive can end right now (`Is_Ok_To_End`-style check succeeds),
  Teleport is restored as active locomotor this same call; if not, it forces
  `Enter_Mission(7, 0)` and sets two pending-state flag bytes on the FootClass, deferring the
  restoration to `FootClass::AI`'s per-tick `IPiggyback::Is_Ok_To_End` check (already documented
  in `CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md` §14 "How the Locomotor Swap Works" step 3).

---

## 2. Remaining Uncertainty — this predicate does NOT by itself explain the classic long-range warp

This is the most important finding of this session and directly qualifies the answer to the
task's core question.

`UnitClass::Mission_Harvest` state 2 (RETURN) guards its entire dock-search body with
`if (param_1[0x169] != 0) goto default;` (`param_1[0x169]` = `this+0x5A4` = **NavCom**, same
field read by `FootClass::GetDestination(this,0)` in §1.2) — verified via
`decompile_function 0x0073E5E0`. This means the fallback branch that computes a validated cell
near the refinery's `DockOffset` and calls `Set_Destination(CellClass)` (the classic "far from
refinery, go home" call) is **only ever reached when NavCom is already NULL**.

But §1.2/1.3 show: when the *old* destination (NavCom) passed into `Set_Destination` is NULL,
the predicate takes the **default flag=1 branch** (§1.4, "prefer Drive"), not the flag=0
"stay/become Teleport" branch — because the RTTI==6 + `+0x16B3` + RTTI==0xB + FindFirstUnit chain
in §1.3 is skipped entirely (`TEST EDI,EDI; JZ 0x7425e6` at `0x742465-0x74246c`) when the old
destination is NULL.

**Consequence:** as traced byte-for-byte, the classic Mission_Harvest-state-2 long-range fallback
call does not hit the "keep Teleport" branch of this specific predicate. Either (a) the active
locomotor happens to already be `DriveLocomotionClass` at that moment and stays Drive, or (b) it
is Teleport and the code creates a Drive piggyback, handing `Head_To_Coord` to Drive. Neither
outcome, *within the bytes I traced*, produces an immediate Teleport-locomotor warp for this
specific call. This directly contradicts the framing in `CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md`
§14/§21 ("Harvest return (far from refinery)... Since FindFirstBuilding returns NULL for the
empty cell, the Teleporter check SKIPS the Drive piggyback... The miner warps"), which assumed
NavCom would be non-NULL and pointing at the refinery building at this call — not verified true.

I did **not** trace `DriveLocomotionClass::Head_To_Coord` (~`0x4AFD40`) or
`DriveLocomotionClass::Process` (~`0x4B0500`) far enough to know whether either internally
re-delegates to a piggybacked Teleport locomotor when the target is far away — that is out of
this report's bounded scope (anchors given were `0x73E5E0`, `0x4D94B0`,
`TechnoTypeClass+0x16B3`, RTTI values only). **This is the load-bearing open question**: the
actual mechanism producing the visible long-range warp on harvest return most likely lives in
Drive's own locomotion tick, not in this `Set_Destination` predicate. Flagging as
UNVERIFIED-pending-reinvestigate rather than guessing.

---

## 3. Inbound-only structural confirmation (separate finding, HIGH confidence)

This directly answers the project's standing fact
(`feedback_chrono_teleport_direction.md`: "Chrono miner warps ONLY inbound... Outbound is a
normal drive") independent of the open question in §2.

Verified via fresh `decompile_function 0x0073E5E0`:

- **State 0 (SEEK, outbound refinery→ore)** never calls the CellClass-fallback
  `Set_Destination` pattern used by state 2. It calls `FootClass__Search_For_Tiberium_Short_And_Move`
  or `FootClass__Search_For_Tiberium_And_Move` — pathfinding-based drive helpers. The only
  `Set_Destination` call visible in state 0's body is `Set_Destination(NULL, 1)` (a
  cancel/clear, gated on the active locomotor already being Teleport with a stale destination),
  not a move-to-cell call.
- **State 2 (RETURN, inbound ore→refinery)** is the only place `RulesClass+0xD78`
  (`HarvesterTooFarDistance`) and `RulesClass+0xD7C` (`ChronoHarvTooFarDistance`) are read
  (re-confirmed this session, matches `CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md` §16), and it is
  the only place the CellClass-fallback `Set_Destination` call exists.

**Conclusion:** whatever the exact downstream locomotor mechanism turns out to be (§2), it is
structurally impossible for Mission_Harvest's outbound leg to feed it — state 0 never presents
the code path required. This confirms the "inbound-only" project fact at the Mission_Harvest
orchestration level. It does not by itself rule out a warp triggered by some unrelated caller
(e.g. a manual player order mid-outbound-journey) — not evaluated here, believed low-relevance
in normal play.

---

## 4. Active in YR

**Yes, unconditionally for stock CMIN.** `[CMIN] Teleporter=yes` at `ini/rulesmd.ini:7396`;
`Locomotor={4A582747-9839-11d1-B709-00A024DDAFD1}` (CLSID_TeleportLocomotion) at
`ini/rulesmd.ini:7398`. Both stock refineries (`[GAREFN]`, `[NAREFN]`) have `DockUnload=yes`
(`ini/rulesmd.ini:11726`, `:12519`), so the `+0x16B3` condition in §1.3 is satisfiable by any
stock refinery. No `SpecialFlags` gate found on this code path.

---

## 5. Implementation Handoff

- **Verified behavior:** `TechnoClass::Set_Destination`'s Teleporter block (`0x7423CD-0x7427C0`)
  defaults to attempting a Drive-locomotor piggyback swap on every call for a Teleporter=yes
  unit, and only skips that swap (favoring/keeping Teleport active) when the *previous*
  destination (NavCom) was a `DockUnload=yes` building AND the *new* destination is an empty,
  unit-free cell.
  **Rust delta:** do not model "chrono miner instantly warps whenever ordered to an empty cell"
  as the general rule — that is refuted by §2 for the harvest-return case specifically (NavCom is
  NULL at that call). Model this predicate exactly as traced in §1, and treat the actual
  long-range-warp trigger as unresolved pending the Drive-locomotion follow-up in §2.
  **Surface:** `sim/miner` locomotion/destination-assignment logic.
  **Acceptance:** a fixture where a chrono miner's NavCom is a `DockUnload=yes` building and the
  new destination is a verified-empty cell must skip any drive-piggyback path and keep/restore
  Teleport as active locomotor within the same call (or defer via the Mission=7 retry flags if
  Drive is still mid-motion) — matching §1.4 flag==0 exactly.
  **Test name suggestion:** `set_destination_gate_skips_drive_when_leaving_dockunload_building`.
  **Risk:** medium — this is a narrow special case, not the primary warp trigger; do not build
  the primary Rust warp-trigger logic around it until §2 is resolved.

- **Verified behavior:** `CellClass::FindFirstUnit` (`0x47EBA0`) checks unit occupancy
  (`What_Am_I()==1`), not building occupancy.
  **Rust delta:** any Rust logic modeling this predicate must check "is a `UnitClass` present on
  the destination cell," not "is a building present."
  **Surface:** `sim/miner`, cell-occupancy queries.
  **Acceptance:** unit test with a cell containing only a building (no unit) must NOT trip this
  check; a cell containing a unit must trip it.
  **Test name suggestion:** `find_first_unit_ignores_buildings`.
  **Risk:** low-frequency in practice (harvest-return destination cells near a dock are rarely
  occupied by another unit) but changes the semantics of any doc/Rust code that copied the old
  "FindFirstBuilding" framing.

- **Verified behavior:** Mission_Harvest state 0 (outbound) structurally cannot reach the
  CellClass-fallback `Set_Destination` call that state 2 (inbound) uses.
  **Rust delta:** none required if the current Rust implementation already models "drive-only
  outbound, distance-gated fallback inbound" — this report only re-confirms the mechanism should
  not need change here; the open item is the *inbound* warp trigger internals (§2), not the
  inbound/outbound split itself.
  **Surface:** `sim/miner` harvest state machine.
  **Acceptance:** N/A (confirmation only).
  **Test name suggestion:** N/A.
  **Risk:** none — this section only confirms an existing, already-relied-upon fact.

---

## 6. Negative Facts / Do Not Do

- Do NOT add an outbound (refinery→ore) teleport. Mission_Harvest state 0 never presents the
  code path (§3); this is independently re-confirmed this session, not merely inherited from the
  project memory.
- Do NOT describe `0x0047EBA0` as `CellClass::FindFirstBuilding`. Verified via
  `decompile_function 0x0047eba0`: it checks `What_Am_I()==1` (UnitClass), not `6` (BuildingClass).
- Do NOT describe `0x007425DB` as "the Drive piggyback path" or `0x007427B2` as "the skip
  piggyback path" without qualification — both are **merge points** reached from multiple
  different upstream reasons (§1.3-1.4); `0x7425DB` is where the flag is actually tested,
  `0x7425FE` is where Drive-piggyback creation actually begins, `0x7427B2` is where the creation
  is actually skipped/cleaned up.
- Do NOT treat "old destination is a `DockUnload` building + new destination is an empty cell"
  as THE explanation for why chrono miners visibly warp home from far ore patches. §2 shows
  NavCom is NULL at that exact call site, so this specific predicate branch is not what fires
  there. Treat the long-range-warp trigger as UNVERIFIED-pending-reinvestigate (likely inside
  `DriveLocomotionClass::Head_To_Coord`/`Process`) until a follow-up closes it.
- Do NOT reuse `TECHNOCLASS_SET_DESTINATION_GHIDRA_REPORT.md`'s "`FootClass__GetDestination(0)`
  RTTI == 0xF (CellClass)" claim — it conflicts with the independently double-verified fact that
  RTTI `0xF` is `InfantryClass`, and with `NAVCOM_LIFECYCLE_GHIDRA_REPORT.md`'s
  independently-verified `NavCom->RTTI == 0xB` for CellClass. That claim belongs to a different,
  unrelated call site in the same function and was not re-verified or corrected in this report
  (out of scope) — flag it separately.

---

## 7. Suggested Doc Patch — `CHRONO_MINER_SYSTEM_OVERVIEW.md` §3

Replace lines 97-152 (the "STRUCTURAL CAVEAT" flowchart and its caveat paragraph) with a summary
pointing at this report instead of re-deriving the flowchart inline, e.g.:

> **Old:** the entire `### The Teleport-vs-Drive Decision` flowchart block (lines 102-142) plus
> the "STRUCTURAL CAVEAT (2026-07-12)" paragraph (lines 144-150).
>
> **New:** "See `CHRONO_MINER_SET_DESTINATION_GATE_GHIDRA_REPORT.md` for the byte-verified
> predicate inside `TechnoClass::Set_Destination` (0x741970). Summary: the predicate defaults to
> a Drive-piggyback swap and only preserves/restores Teleport when the *previous* NavCom was a
> `DockUnload=yes` building and the *new* destination is an empty, unit-free cell — verified via
> `disassemble_function 0x00741970`. **Open question:** Mission_Harvest's classic long-range
> harvest-return fallback call always presents NavCom==NULL to this predicate, so this specific
> branch does not explain that warp; the actual trigger is suspected to live in
> `DriveLocomotionClass::Head_To_Coord`/`Process` and is UNVERIFIED-pending-reinvestigate. The
> inbound-only behavior itself (never outbound) is independently confirmed at the
   Mission_Harvest orchestration level (state 0 never reaches the fallback call state 2 uses)."

---

## Sources

- Ghidra (read-only, this session): `disassemble_function 0x00741970` (full function, primary
  evidence for §1); `decompile_function 0x00741970`, `0x0073E5E0`, `0x0047eba0`, `0x0065ad30`;
  `get_function_by_address 0x004D94B0`, `0x0047eba0`, `0x0065ad30`; `get_function_callers
  0x004D94B0`; `read_memory 0x007e9a90`, `0x007e9a30` (CLSID bytes).
- `ini/rulesmd.ini:7396` (`[CMIN] Teleporter=yes`), `:7398` (`Locomotor=`), `:11726`/`:12519`
  (`DockUnload=yes`).
- Prior docs consulted (not re-verified except where explicitly noted as corrected):
  `docs/research/miner/CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md` §14, §15, §16, §21;
  `docs/research/miner/CHRONO_MINER_SYSTEM_OVERVIEW.md` §3;
  `docs/research/NAVCOM_LIFECYCLE_GHIDRA_REPORT.md` §1-3, §5;
  `docs/research/BUILDINGTYPE_FLAGS_16B3_16BB_16BC_REFINERY_WEEDER_GHIDRA_REPORT.md`;
  `docs/research/TECHNOCLASS_SET_DESTINATION_GHIDRA_REPORT.md` (one claim flagged as
  contradicted, not corrected — out of scope);
  `docs/research/CELL_OCCUPANCY_ORDERING_FOLLOWUP_GHIDRA_REPORT.md`;
  `docs/research/TERRAIN_CLASS_GHIDRA_REPORT.md` §21.2.

**Status: teleport-vs-drive predicate for this specific `Set_Destination` block VERIFIED
(§1, HIGH confidence, asm-read). Long-range harvest-return trigger mechanism NOT resolved —
UNVERIFIED-pending-reinvestigate (§2). Inbound-only structural claim CONFIRMED (§3, HIGH
confidence, independent of §2).**
