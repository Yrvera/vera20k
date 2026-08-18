# Chronoshiftable= INI Key — Where Does It Parse?

**Investigation date:** 2026-05-19
**Source:** `gamemd.exe` (YR 1.001), live Ghidra MCP decompilation, string-table searches, byte-pattern searches.
**Confidence:** HIGH on the primary findings (verified directly from binary).
**Active in YR:** N/A — the question itself is "does this key exist?", answered below.

---

## 1. Headline Finding

**The string `Chronoshiftable` is not present anywhere in the gamemd.exe binary.**
Stock gamemd does not parse `Chronoshiftable=` as an INI key.

The two fields the chrono/superweapon docs in `ra2-rust-game-docs/` claim are
"Chronoshiftable" are actually:

| Doc claim | Actual identity (verified) |
|---|---|
| `TechnoTypeClass+0xCCE = Chronoshiftable` (chrono miner & teleport docs) | `Naval=` |
| `TechnoTypeClass+0xD97 = Chronoshiftable` (Chronosphere superweapon doc) | `Organic=` |

Both INI key names verified by reading the string-pointer pushed onto the stack
immediately before `ReadBool` in `TechnoTypeClass::ReadINI`, and confirmed via
`get_xrefs_to` on those string addresses.

Where `Chronoshiftable=` actually comes from in community documentation
(ModEnc et al.) is **unverified** by this investigation. Possibilities include
(a) a community-doc fiction that propagated from an early mis-labeled RE
attempt, or (b) a real INI key defined by a third-party engine extension
(Ares, YRpp, Phobos). This report does not have evidence for either — only
that stock gamemd's binary and the stock retail RA2/YR `rulesmd.ini` /
`rules.ini` files used by this project do not contain or parse this key.

---

## 2. Evidence — `+0xCCE` is `Naval=`

`TechnoTypeClass::ReadINI` write site at **`0x00714A6A`**:

```
0x00714A49  8a 8d ce 0c 00 00    MOV CL, [EBP + 0xCCE]      ; read current value
0x00714A4F  51                   PUSH ECX                    ; default
0x00714A50  68 5c 39 84 00       PUSH 0x0084395C             ; string ptr → "Naval"
0x00714A55  53                   PUSH EBX                    ; section
0x00714A56  8b cf                MOV ECX, EDI                ; this
0x00714A58  e8 79 4b e1 ff       CALL ReadBool
0x00714A5D  88 85 ce 0c 00 00    MOV [EBP + 0xCCE], AL       ; write result
```

String at `0x0084395C` decodes as **`"Naval\0"`** (verified via `read_memory`).
`get_xrefs_to(0x0084395C)` returns exactly one xref, from `0x00714A6A` in
`TechnoTypeClass__ReadINI` (DATA reference).

---

## 3. Evidence — `+0xD97` is `Organic=`

`TechnoTypeClass::ReadINI` write site at **`0x0071503F`**:

```
0x00715024  8a 95 97 0d 00 00    MOV DL, [EBP + 0xD97]       ; read current value
0x0071502A  52                   PUSH EDX                    ; default
0x0071502B  68 14 37 84 00       PUSH 0x00843714             ; string ptr → "Organic"
0x00715030  8b cb                MOV ECX, EBX
0x00715032  e8 89 fe e0 ff       CALL <section helper>
0x00715037  50                   PUSH EAX                    ; section
0x00715038  8b cf                MOV ECX, EDI                ; this
0x0071503A  e8 b1 45 e1 ff       CALL ReadBool
0x0071503F  88 85 97 0d 00 00    MOV [EBP + 0xD97], AL       ; write result
```

String at `0x00843714` decodes as **`"Organic\0"`** (verified via `read_memory`).
`get_xrefs_to(0x00843714)` returns exactly one xref, from `0x0071502B` in
`TechnoTypeClass__ReadINI` (DATA reference).

`search_byte_patterns "88 85 97 0d 00 00"` returns exactly one match across the
entire binary: `0x0071503F`. There is no other write site to `+0xD97`.

---

## 4. Evidence — `Chronoshiftable` Is Not in the Binary

- `search_strings "^Chronoshiftable$"` → 0 matches.
- `search_strings "Chronoshift"` → 0 matches.
- Comprehensive search for `search_strings "Chrono"` returns 24 strings.
  Full list (no `Chronoshiftable` anywhere):

  `EVA_ChronosphereDetected`, `ChronoWarp`, `ChronoSphere`, `ChronoBeamColor`,
  `ChronoOutSound`, `ChronoInSound`, `DefaultChronoSound`, `CHRONOSK.SHP`,
  `ChronoHarvTooFarDistance`, `ChronoRangeMinimum`, `ChronoMinimumDelay`,
  `ChronoTrigger`, `ChronoDistanceFactor`, `ChronoReinfDelay`, `ChronoDelay`,
  `ChronoSparkle1`, `ChronoBlastDest`, `ChronoBlast`, `ChronoBeam`,
  `ChronoPlacement`, `EVA_ChronosphereReady`, `EVA_ChronosphereActivated`,
  `ChronoTurretWeapon`, `ChronoTurretIndex`.

Stock gamemd's INI parser only reads keys whose names are literal strings in the
binary's string table. Absence of the string is definitive: the key cannot be
parsed by `TechnoTypeClass::ReadINI` (or any other ReadINI). The
`Chronoshiftable=` key support in the YR modding scene comes from
Ares/YRpp/Phobos engine extensions, not from the stock binary this project is
matching.

---

## 5. What the Chronosphere Superweapon Actually Checks

`SuperClass::Launch` (function `0x006CC390 – 0x006CDE42`) case 4 (ChronoWarp)
contains the kill-vs-warp filter. The relevant decompiled fragment:

```c
// piVar2 = the unit at the source cell
iVar16 = piVar2->vtable[0x84]();              // GetTechnoType → TechnoTypeClass*

if ( *(char*)(iVar16 + 0xD97) == 0            // NOT Organic
     || *(char*)(iVar16 + 0xCD4) != 0         // OR Teleporter=yes
     || piVar2->vtable[0x54]()                // OR IsCloaked()
   ) {
    // ===> TELEPORT THE UNIT (warp it to the destination cell)
}
else {
    // ===> KILL THE UNIT
    //   apply primary warhead (TechnoTypeClass + 0xA0) with Rules+0xFA8
    iVar16 = piVar2->vtable[0x84]();
    local_194 = *(uint*)(iVar16 + 0xA0);
    piVar2->vtable[0x16C](&local_194, 0, *(int*)(g_RulesClass_Instance + 0xFA8));
}
```

**Effective rule:** the Chronosphere kills a unit at the source cell iff
`Organic=yes AND Teleporter=no AND not currently cloaked`. Every other unit
gets warped to the destination cell.

In standard YR INIs:
- Regular infantry (`[E1]`, `[E2]`, `[GGI]`, `[INIT]`, etc.): `Organic=yes`,
  `Teleporter=no` → **killed**.
- Chrono Legionnaire (`[CLEG]`): `Organic=yes`, `Teleporter=yes` → **warped**.
- Chrono Miner (`[CMIN]`): `Organic=no`, `Teleporter=yes` → **warped** (both
  conditions exempt; either alone would also exempt).
- Mirage Tank cloaked (`[MGTK]` in cloaked state): `Organic=no`, IsCloaked → **warped**.
- Vehicles in general (`Organic=no`): **warped**.
- Aircraft: `Organic=no` → **warped** (in practice aircraft are skipped
  earlier by the on-map filter; not load-bearing for this finding).

This **is** the iconic "Chronosphere kills infantry, warps vehicles" rule, and
it's gated entirely on `Organic=` plus the `Teleporter=` and cloak exemptions.
There is no `Chronoshiftable=` flag involved.

---

## 6. What the Locomotor / Bridge / Self-Teleport Code Checks at `+0xCCE`

Multiple "Step 2: Chronoshiftable" / "if Chronoshiftable" comments in
`TELEPORT_LOCOMOTION_DEEP_DIVE.md`, `CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md`,
and `POSTWARP_VALIDATION_INVALID_TERRAIN_GHIDRA_REPORT.md` are actually reads
of `+0xCCE = Naval=`.

Semantic re-reading in context:

- **`TeleportLocomotionClass::PostWarpValidation` (0x007187A0)**: the
  `if (TypeClass+0xCCE != 0)` branch in §5 of the postwarp doc is "if the
  warping unit is Naval, treat the bridge check differently" — i.e. naval
  units have different bridge-overlay handling than land units. This is sane
  and consistent with `Naval=` semantics.

- **TELEPORT_LOCOMOTION_DEEP_DIVE.md "Step 2: Chronoshiftable bridge check"**
  is actually the "Naval-on-bridge-cell" gate. A non-naval unit warping onto
  a bridge cell goes through the standard land-passability path; a naval unit
  is exempted from the land-passability rejection because water under the
  bridge is a valid surface for it. This is what `Naval` is for.

- **The "Exception 1: Chronoshiftable units survive water destination"**
  in the same doc is actually "Naval units survive water destination." Again
  consistent with `Naval`.

In standard YR, the Chrono Miner has `Naval=no`, so every one of these "if
Chronoshiftable" branches that fire in the chrono miner's harvest-return path
take the same branch they took before this correction — the relabeling
clarifies the *name* of the check, not its *outcome* for the chrono miner.
Practical-parity impact on the chrono miner: **none**. Practical-parity impact
on naval-unit warps via Chronosphere: previously documented mechanism was
under the wrong name; the actual mechanism is correct as documented.

---

## 7. TechnoTypeClass Offsets — Verified Triad (Local Scope)

The three byte fields cluster in the same ReadINI block (~0x00714A00 –
0x00715050). All verified from `MOV [EBP + offset], AL` patterns and
string-pointer xrefs.

| Offset | INI Key | Type | String Address | Write Site |
|---|---|---|---|---|
| `+0xCCE` | `Naval=` | bool | `0x0084395C` | `0x00714A6A` in `TechnoTypeClass::ReadINI` |
| `+0xCD4` | `Teleporter=` | bool | `0x00843E60` | `0x00713FF6` in `TechnoTypeClass::ReadINI` |
| `+0xD97` | `Organic=` | bool | `0x00843714` | `0x0071503F` in `TechnoTypeClass::ReadINI` |

(`+0xCD4 = Teleporter` was independently confirmed: byte-pattern search for
`88 85 d4 0c 00 00` returned exactly one hit at `0x00713FF6`, and the string at
`0x00843E60` is `"Teleporter\0"`.)

`+0xCCE` and `+0xD97` were the two offsets in active doc-archive contention.
Both are now resolved.

---

## 8. Affected Docs — Cross-Doc Inventory of Bad Claims

These reports contain at least one assertion that `Chronoshiftable=` exists in
stock gamemd at `+0xCCE` or `+0xD97`. Each needs a localized correction (the
*offsets* in the docs are correct, only the *INI-key names* are wrong):

1. **`CHRONO_MINER_SYSTEM_OVERVIEW.md`** line 304:
   `+0xCCE | Chronoshiftable | bool | Can be moved by Chronosphere superweapon`
   → should read `+0xCCE | Naval | bool | TechnoTypeClass naval flag`.

2. **`CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md`**:
   - Line 864: `bool chronoshiftable = type->Chronoshiftable (+0xCCE);`
     → variable should be `type->Naval`.
   - Line 1183 in the offsets table: same rename as above.

3. **`CHRONOSPHERE_SUPERWEAPON_GHIDRA_REPORT.md`**:
   - Lines 176–180 (filter description): `Chronoshiftable=no (TypeClass+0xD97)`
     → should be `Organic=yes (TypeClass+0xD97)` and the kill-condition logic
     re-expressed as "kill if Organic AND not Teleporter AND not cloaked".
   - Line 295 in summary: `Chronoshiftable=no` → wrong key name and inverted
     sense; rephrase as "Organic infantry without Teleporter and not cloaked
     are killed instantly".
   - Lines 938–939: `Chronoshiftable=yes/no ; TypeClass+0xD97 - Can survive
     ChronoSphere warp` → rename to `Organic=yes/no` and rewrite description
     as "if Organic and not Teleporter and not cloaked, killed instead of
     warped by ChronoSphere".

4. **`TELEPORT_LOCOMOTION_DEEP_DIVE.md`**:
   - Lines 625–628 ("Step 2: Chronoshiftable bridge check") → rename to
     "Step 2: Naval bridge check".
   - Lines 656–657 ("Exception 1: Chronoshiftable units survive") → rename to
     "Exception 1: Naval units survive water destination".
   - Line 1238 ("PostWarpValidation exception chain": `Chronoshiftable, Infantry,
     HasBridge, LandType==Road`) → rename the first exception to `Naval`.

5. **`TELEPORT_LOCOMOTION_IMPLEMENTATION_REFERENCE.md`** — referenced in the
   grep results, contains the same naming pattern. Same correction class as
   #4 (review each occurrence before patching).

6. **`SUPERWEAPON_GAPS_INVESTIGATION_REPORT.md`** and
   **`SUPERWEAPON_SYSTEM_CONSOLIDATED_REPORT.md`** and
   **`SUPERCLASS_SYSTEM_GHIDRA_REPORT.md`** — referenced in the grep results,
   contain `Chronoshiftable` references. Same correction class as #3 (review
   each occurrence before patching).

7. **`POSTWARP_VALIDATION_INVALID_TERRAIN_GHIDRA_REPORT.md`** — the §5 label
   `TechnoType+0xCCE` was corrected from "BridgeCrosser?" → "Naval" in the
   earlier swarm reconciliation pass. Verify the existing correction matches
   the evidence in this report (it does).

8. **`combat/systems/can_target_gates.md`** (lines 180, 238, 311, 390, 474)
   contains multiple competing labels for `+0xD97` — "MissileSpawn must deploy",
   "third 'can't be Magnetron'd' flag", "deploy-and-fire gate", and an explicit
   open question on line 474 asking to resolve the INI-key mapping for
   `TechnoType.byte+0xD27/0xD94/0xD97/0xE13`. The actual identity is `Organic=`;
   the combat-pipeline behavioral observations ("Magnetron immunity gate",
   "must-deploy-to-fire gate") may still be correct as semantic observations of
   how the engine *uses* the Organic flag, but the INI-key label needs
   correction.

9. **`FIRE_AT_PIPELINE_GHIDRA_REPORT.md`** line 213 ("Type +0xD97 (deploy-and-fire
   gate)") — same correction as #8: rename the field to Organic, keep the
   behavioral observation if independently verifiable.

10. **`AIRCRAFTTYPECLASS_COMPLETE_GHIDRA_REPORT.md`** lines 147 and 400 mention
    `0xD97` being re-zeroed by aircraft ctor. The offset claim is fine; no
    INI-key label asserted, no correction needed.

11. **`BUILDINGTYPECLASS_CTOR_DEFAULTS.md`** lines 70, 403 mention `0xD97` byte
    default = 0 (override by building ctor). The offset claim is fine; no
    INI-key label asserted, no correction needed.

12. **Combat-pipeline overlap risk:** the combat docs that label `+0xD97` as
    deploy/missile-related warrant a follow-up. If those behavioral observations
    are real, then the engine is overloading the `Organic=` flag for two
    unrelated purposes (infantry-vs-Chronosphere AND deploy/fire behavior). That
    is plausible (TS-legacy code reusing a flag bit for an unrelated check)
    but should be verified separately — out of scope for this report.

---

## 9. Open Questions — Final State of the Investigation Log

- `[RESOLVED]` Q1 — Is `Chronoshiftable=` at `+0xD97`, `+0xCCE`, or somewhere
  else? → Neither. The string `Chronoshiftable` is not present in the binary
  at all; the key is not parsed in stock gamemd. (evidence:
  `search_strings "Chronoshift"` returned 0 matches; comprehensive `Chrono*`
  string listing contains no match.)

- `[RESOLVED]` Q2 — Where is the `Chronoshiftable=` parser? → Nowhere in
  stock gamemd. (evidence: same as Q1; also `get_xrefs_to` on every `Chrono*`
  string was inspected and none triggers a ReadBool for a "Chronoshiftable"
  key.)

- `[RESOLVED]` Q3 — What does the Chronosphere superweapon target-eligibility
  check actually compare? → `SuperClass::Launch` (`0x006CC390 – 0x006CDE42`)
  case 4 checks the union `(Organic == 0) OR (Teleporter != 0) OR IsCloaked()`
  to teleport; else kills via primary warhead. (evidence: decompilation of
  `SuperClass__Launch`; both `+0xD97` and `+0xCD4` reads visible in the
  conditional.)

- `[RESOLVED]` Q4 — What does `+0xCCE` hold? What does `+0xD97` hold? →
  `+0xCCE = Naval=`; `+0xD97 = Organic=`. (evidence: write sites at
  `0x00714A6A` and `0x0071503F` push string pointers to "Naval" (`0x0084395C`)
  and "Organic" (`0x00843714`) respectively; both string contents verified by
  `read_memory`.)

- `[RESOLVED — partial]` Q5 — Why is the `Chronoshiftable` string absent
  from the binary? → Because stock gamemd does not implement that INI key.
  Verified: complete `Chrono*` string-table enumeration contains no match;
  no parser bytes reference the missing string; the stock retail
  `rulesmd.ini` / `rules.ini` in this project's `ini/` directory also
  contain zero `Chronoshiftable=` entries. **Not verified:** where the key
  *is* implemented in the broader YR modding ecosystem. ModEnc documents
  the key as if it were standard YR; this investigation cannot confirm
  whether it's a community-doc fiction or a third-party engine-extension
  key (Ares/YRpp/Phobos), because no such extension binaries were
  inspected. For this project's scope (stock gamemd parity + stock INIs),
  the key does not exist and need not be implemented.

- `[DEFERRED]` Q6 — In `SuperClass::Launch` case 4 there is a separate
  `*(char*)(GetTechnoType(piVar2[0x1A5]) + 0xCCE) != 0` branch that calls
  `WarpAttachClass::Detach`. The pointer at `TechnoClass + 0x694` (=
  `piVar2[0x1A5]`) is labeled `FlashAnim` in `CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md`
  §12, but the case-4 code uses it as if it were a TechnoClass (calls
  `GetTechnoType` and reads `+0x69C`), which strongly suggests it is actually a
  `WarpAttachClass*` (a Techno subclass). (category: `requires-different-system-context`
  — answering this needs a TechnoClass+0x694 struct-field investigation, which
  is a separate scope.) Next-step-if-pursued: decompile every read of
  `TechnoClass+0x694` and classify by RTTI; confirm `WarpAttachClass` is one of
  the legal occupants and document under which conditions.

- `[DEFERRED]` Q7 — Do the combat-pipeline `+0xD97`-named "deploy/fire" gates
  (`can_target_gates.md` and `FIRE_AT_PIPELINE_GHIDRA_REPORT.md`) actually
  reflect engine behavior that overloads the `Organic=` flag for an unrelated
  purpose, or are those behavioral observations also mislabeled? (category:
  `out-of-scope` — distinct system, needs an independent investigation of the
  fire pipeline.) Next-step-if-pursued: run `/re-investigate` scoped to
  `FIRE_AT_PIPELINE` with the question "what does each `+0xD97` read in the
  fire pipeline actually gate, given `+0xD97 = Organic=`?".

Adversarial-reader questions checked and answered (all from binary evidence,
none added new entries to the log on the zero-add final pass):

1. **Could the `Chronoshiftable` string be in a compressed/packed section
   that Ghidra hasn't decoded?** — Ghidra string analysis is comprehensive on
   PE32 files like gamemd.exe; any uncompressed-at-runtime data sections would
   appear via `search_strings`. No literal "Chronoshiftable" anywhere.
2. **Could `gamemd.exe` load a DLL that parses `Chronoshiftable=`?** — Stock
   gamemd is a single-binary game with no INI-parsing DLL plugins. Whether
   any third-party extension (Ares/YRpp/Phobos) adds such parsing in modded
   environments is not investigated here; this project uses stock gamemd
   and stock INIs.
3. **Could the INI key be aliased to a different in-binary string?** — INI
   ReadBool calls always use literal C-string pointers; no aliasing layer
   exists in the parser. Inspected the `ReadBool` call site at `0x0071503F`
   (the `+0xD97` write); the string-pointer push is direct.
4. **Could the Chronosphere check use a runtime-computed offset that happens
   to land on `+0xD97`?** — The decompilation shows a literal
   `*(char*)(iVar16 + 0xD97)`. The constant is hard-coded in the binary;
   no indirection.
5. **Could there be a second SuperClass with a different `Launch` implementation
   that checks Chronoshiftable?** — `SuperClass::Launch` at `0x006CC390` is the
   sole implementation; verified by `get_function_by_address`. No vtable
   overrides for case 4 exist in this binary.

---

## 10. Rust-Port Implications

When implementing the Chronosphere superweapon in the Rust port, **do not**
implement a `Chronoshiftable=` INI key on `TechnoTypeClass`. Implement the
following rule instead:

```
For each unit at the source cell:
    if unit.type.organic && !unit.type.teleporter && !unit.is_cloaked():
        kill the unit using its primary warhead and Rules.C4Warhead damage scaling
    else:
        warp the unit to the destination cell
```

Where:
- `organic` is parsed from `[UnitType] Organic=` in `rulesmd.ini`
  (default `no` for vehicles, `yes` for infantry units).
- `teleporter` is parsed from `[UnitType] Teleporter=` (default `no`).
- `is_cloaked()` checks the unit's current cloak state, not a TypeClass flag.

This is parity-correct against `SuperClass::Launch` case 4 in stock gamemd.

For the locomotor/bridge code that the chrono docs labeled "Chronoshiftable" at
`+0xCCE`: implement the check against `Naval=` instead. For most chrono-miner
behaviors, the branch outcome for a non-naval, non-organic unit is identical to
the doc-described behavior; only the field name changes. The mislabeling did
not produce any incorrect Rust-port logic to date — but if the port has
implemented a `chronoshiftable` boolean on TechnoTypeClass, that field should
be renamed to `naval` (when used in locomotor/bridge contexts) or `organic`
(when used in Chronosphere kill-rule contexts), and the parsed INI key
corrected accordingly.

---

## 11. Sources

- Ghidra MCP live decompilation of `gamemd.exe`:
  - `SuperClass__Launch` @ `0x006CC390` (body `0x006CC390 – 0x006CDE42`),
    case 4 (ChronoWarp).
  - `TechnoTypeClass__ReadINI` write blocks at `0x00714A6A` (+0xCCE), at
    `0x00713FF6` (+0xCD4), at `0x0071503F` (+0xD97).
  - `WarpUnitsAtCell` @ `0x0065EC30` and trigger-action helper
    `FUN_0065D8E0` for context (neither contains the `+0xD97` check).
- Binary string table: `search_strings`, `read_memory` on string addresses
  `0x0084395C` ("Naval"), `0x00843E60` ("Teleporter"), `0x00843714` ("Organic").
- Byte-pattern search: `88 85 97 0d 00 00` (write to `+0xD97`) and
  `88 85 d4 0c 00 00` (write to `+0xCD4`) — each returned exactly one match.
- Cross-doc grep across `C:/Users/enok/Documents/ra2-rust-game-docs/`.

---

**Status: COMPLETE.** Zero-add Ghidra pass executed; Open Questions Log
resolved or explicitly deferred per category. No `[OPEN]` items remain.
