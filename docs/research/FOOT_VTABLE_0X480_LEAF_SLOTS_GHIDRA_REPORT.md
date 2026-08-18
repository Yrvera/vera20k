# Foot-Hierarchy vtable+0x480 Leaf Slots — Identity & Role (Ghidra-Verified)

**Session:** 2026-07-19, testProsjekt (canonical), gamemd.exe, image base `0x400000`.
**Trigger:** NAVCOM_LIFECYCLE_GHIDRA_REPORT.md §1/§11 flagged `FUN_0051aa40` (Infantry) and
`0x0041AA80` (labeled `UnitClass__EnterBuildingOrDock`, but proven to live in AircraftClass's
vtable) as `needs_reinvestigate` — their true owner/role was left UNVERIFIABLE in that doc's
2026-07-12 correction pass.

**Verdict: CONFIRMED.** Both `0x0041AA80` and `0x0051AA40` are leaf overrides of the
**same virtual slot** as `UnitClass`'s Set_Destination (vtable+0x480) — receivers are
**AircraftClass** and **InfantryClass** respectively, confirmed via RTTI/COL walk. The
existing Ghidra label `UnitClass__EnterBuildingOrDock` on `0x0041AA80` is **WRONG** — it
is exclusively AircraftClass's vtable slot, never UnitClass's.

---

## 1. Vtable slot bytes (read_memory, this session)

| Vtable (class, RTTI-verified §2) | Base | +0x480 addr | Bytes (LE) | Value |
|---|---|---|---|---|
| TechnoClass | `0x007F4960` | `0x007F4DE0` | `30 9a 70 00` | `0x00709A30` |
| FootClass | `0x007E8C94` | `0x007E9114` | `b0 94 4d 00` | `0x004D94B0` |
| UnitClass | `0x007F5C70` | `0x007F60F0` | `70 19 74 00` | `0x00741970` |
| AircraftClass | `0x007E22A4` | `0x007E2724` | `80 aa 41 00` | `0x0041AA80` |
| InfantryClass | `0x007EB058` | `0x007EB4D8` | `40 aa 51 00` | `0x0051AA40` |

Each vtable base was itself re-verified from the raw vtable-pointer bytes at the class
constructor's `this[0]` address before reading +0x480 (`read_memory` at each stated base,
matching the addresses given in the task).

## 2. Owner-class verification (RTTI COL walk)

Standard MSVC32 layout: `vtable-4` → `RTTICompleteObjectLocator` (20 bytes: signature,
offset, cdOffset, `pTypeDescriptor`@+0xC, `pClassHierarchyDescriptor`@+0x10) →
`TypeDescriptor` (`pVFTable`, `spare`, mangled `name` @+0x8).

| Vtable base | COL ptr (`base-4`) | `pTypeDescriptor` (COL+0xC) | Mangled name (TD+0x8) | Owner |
|---|---|---|---|---|
| `0x007F4960` | `0x0080C058` | `0x00817B58` | `.?AVTechnoClass@@` | **TechnoClass** |
| `0x007E8C94` | `0x00800948` | `0x00817B78` | `.?AVFootClass@@` | **FootClass** |
| `0x007E22A4` | `0x007FB4C0` | `0x00817B90` | `.?AVAircraftClass@@` | **AircraftClass** |
| `0x007EB058` | `0x008033B8` | `0x00825508` | `.?AVInfantryClass@@` | **InfantryClass** |
| `0x007F5C70` | `0x0080CC68` | `0x00842D80` | `.?AVUnitClass@@` | **UnitClass** |

All five owner attributions are RTTI-verified this session (`read_memory` chain above), not
inferred from labels.

## 3. Uniqueness check (`get_xrefs_to`) — no shared/ambiguous slot values

- `get_xrefs_to 0x0041AA80` → **exactly one** DATA xref: `0x007E2724` (AircraftClass
  vtable+0x480). No xref from `0x007F60F0` (UnitClass) or anywhere else.
- `get_xrefs_to 0x0051AA40` → **exactly one** DATA xref: `0x007EB4D8` (InfantryClass
  vtable+0x480).
- `get_xrefs_to 0x00741970` → **exactly one** DATA xref: `0x007F60F0` (UnitClass
  vtable+0x480). **Not** referenced by TechnoClass's own vtable slot (`0x007F4DE0`, which
  holds a different value — see §4). This corroborates the existing doc's 2026-07-12 note
  that `0x00741970` is UnitClass's live slot value, not a generic "TechnoClass-contributed"
  one — the current Ghidra function name `TechnoClass__Set_Destination` is a naming
  artifact, not a receiver claim; the only real vtable wiring is to UnitClass.

## 4. Role of each function (decompile_function, this session)

All three functions share the identical `__thiscall(this, AbstractClass* target, bool/char
initiator)` signature shape and **terminate by calling `FootClass__Set_Destination_Internal`
(`0x004D94B0`)** — non-virtually, as a direct call, not through the vtable.

- **`0x00741970` (UnitClass slot).** ~500-line preprocessing: locomotor piggyback/CLSID
  swap (Drive/Teleport/Hover locomotion), approach-cell finding via
  `FootClass__Find_Nearby_Passable_Cell`, bridge-adjacency checks (`MapClass__FindBridgeRecord`
  + `Sqrt_Approx`), refinery/helipad-adjacent building anim clearing, and a **self-call
  through the same virtual slot** on the *current destination object*:
  `(**(code**)(*piVar10 + 0x480))(0,1)` — i.e. `dest->Set_Destination(NULL, 1)`.
- **`0x0041AA80` (AircraftClass slot, mislabeled `UnitClass__EnterBuildingOrDock`).**
  Checks a target-cell flag (`+0x16cb`, gates only when set) and, when the aircraft has no
  valid path and mission is Guard(7): looks up a nearby object 1000 cells away via
  vtable+0x528, **recursively self-calls its own vtable+0x480 with `(0,1)`**
  (`Set_Destination(NULL, 1)`), then either paths to that lookup result or falls back to
  `AircraftClass__Find_Nearest_Friendly_Airfield()` (a name-verified AircraftClass-only
  helper, called directly with implicit `this`), then sets mission to Enter(2) or Guard(7)
  accordingly. This is airfield/return-to-base docking logic layered on top of the shared
  Set_Destination contract — confirmed as three **direct, non-virtual** calls to
  `FootClass__Set_Destination_Internal` (`get_xrefs_to 0x004D94B0` shows call sites
  `0x0041aaa6`, `0x0041ad06`, `0x0041adb4`, all inside this function).
- **`0x0051AA40` (InfantryClass slot, previously unnamed `FUN_0051aa40`).** Gates on
  `HouseClass__IsPlayerControl()` + current Mission field (`this+0x1b1`) ∈
  {`0x1b`,`0x1c`,`0x1d`,`0x1e`} → early-return no-op (blocks retargeting during those four
  special infantry mission states; exact mission names not identified this session — see
  §7 Unverified). Otherwise: cell-passability recheck, a player-feedback call
  (vtable+0x558) when the new target equals the already-set one, Guard-mission path
  re-evaluation matching the same `SetGhostCell`/RTTI-type-6/type-1 branching shape as
  `0x00741970`, and the same Drive-locomotion CLSID-swap idiom
  (`COM__CoCreateInstance_Locomotor(&CLSID_DriveLocomotion,...)`) seen in `0x00741970`. One
  direct call to `FootClass__Set_Destination_Internal` at `0x0051b1d2`
  (`get_xrefs_to 0x004D94B0`).

## 5. Base-class slot values confirm this is a progressive-override family

- **TechnoClass's own slot (`0x00709A30`)** decompiles to a **trivial `return;` stub** —
  a genuine, in-bounds no-op default (TechnoClass covers immobile Technos too, e.g.
  buildings, where "set destination" is meaningless by default).
- **FootClass's own slot (`0x004D94B0`)** *is* `FootClass__Set_Destination_Internal`
  itself — FootClass's base override for mobile Technos is the committer with no extra
  preprocessing.
- **UnitClass/InfantryClass/AircraftClass** each override again with class-specific
  preprocessing before calling the same committer directly.

This is the textbook shape of a **virtual-override family**: `TechnoClass` (no-op) →
`FootClass` (base implementation = committer) → leaf classes (type-specific preprocessing,
same committer). Combined with: (a) identical signature across all three leaf functions,
(b) all three are the **sole three callers** of `FootClass__Set_Destination_Internal`
(`get_function_callers 0x004D94B0` → exactly `FUN_0051aa40`, `TechnoClass__Set_Destination`,
`UnitClass__EnterBuildingOrDock`, no others), and (c) the shared `Set_Destination(NULL,1)`
self-recursion idiom through the same slot in both `0x00741970` and `0x0041AA80` — this
conclusively answers the task's central question: **yes, `0x0041AA80` and `0x0051AA40` are
Set_Destination-family overrides of the same virtual slot as UnitClass's**, not unrelated
functions.

## 6. Active in YR

**Yes**, for all three (Unit/Infantry/Aircraft), high confidence. `FootClass__GetDestination`
(the paired read-side accessor for the same NavCom state these functions write) is called
from `EventClass__Execute` (the player/network command dispatcher), `AircraftClass__Mission_Guard`,
`AircraftClass__Mission_Enter`, `FootClass__Mission_Enter`, `TechnoClass__ChangeOwner`,
`WarheadTypeClass__Detonate`, and 25+ other functions spanning ordinary move orders, guard
re-evaluation, radio/dock handshakes, and damage-response repositioning
(`get_function_callers FootClass__GetDestination`) — this is core, every-match movement
plumbing, not a conditional/rare path. Not independently re-traced: the exact call site(s)
that invoke the vtable+0x480 slot virtually (out of scope; the direct-call evidence above and
the shared committer's caller set are sufficient to establish the family and its liveness).

## 7. Unverified (YELLOW)

- Exact semantic meaning of InfantryClass's blocked Mission values `0x1b`–`0x1e`
  (`this+0x1b1` in `0x0051AA40`) — not resolved this session (would need MissionClass enum
  cross-reference); does not affect the identity/family verdict.
- Whether `BuildingClass` or `VesselClass` also carry a leaf override at this slot — out of
  scope per task instructions (no full vtable mapping performed).

## 8. Implementation Handoff

- Confirms `docs/plans` / Rust `sim/movement` should treat AircraftClass and InfantryClass
  as having their **own** Set_Destination preprocessing layers (airfield-docking gate for
  Aircraft; special-mission retarget-block for Infantry) distinct from UnitClass's — not a
  single shared "TechnoClass::Set_Destination" preprocessing function as the current doc
  wording could imply for non-Unit leaf classes.
- Aircraft-specific: any Rust move-order handler for aircraft must gate through an
  equivalent of `AircraftClass__Find_Nearest_Friendly_Airfield` fallback when the "airfield
  required" cell flag is set and no direct path exists.
- Infantry-specific: Rust move-order handling for infantry must no-op (reject retargeting)
  while the unit is in one of the four still-unidentified special missions (§7) — needs a
  follow-up mission-enum resolution before implementation, not a blind port.

## 9. Negative Facts / Do Not Do

- Do NOT keep calling `0x0041AA80` "UnitClass::EnterBuildingOrDock" — RTTI-proven
  AircraftClass-only, zero UnitClass vtable reference.
- Do NOT treat `0x0041AA80`/`0x0051AA40` as unrelated helper functions — both are
  Set_Destination virtual-slot overrides, same family as `0x00741970`.
- Do NOT assume TechnoClass's own vtable+0x480 is a copy of UnitClass's/FootClass's — it is
  a distinct no-op stub (`0x00709A30`), verified in-bounds via RTTI (not a bounds-overrun
  artifact).
- Do NOT treat FootClass's own vtable+0x480 (`0x004D94B0`) as reachable at runtime for a
  live Unit/Infantry/Aircraft object — per NAVCOM_LIFECYCLE §1's prior (still valid)
  finding, FootClass's raw vtable is only ever live between `FootClass::Constructor` and the
  leaf constructor's vtable overwrite.
- Do NOT expand this doc into a full vtable map — scope was strictly the +0x480 identity
  question.

## 10. Remaining Uncertainty

Infantry Mission `0x1b`–`0x1e` semantics (§7). No uncertainty remains on the core
identity/role/family question this report was dispatched to resolve.

## 11. Sources / verifying calls (inline-cited above)

`read_memory` ×13 (vtable bases, +0x480 slots, COL pointers, TypeDescriptor names),
`decompile_function` ×5 (`0x00741970`, `0x0041AA80`, `0x0051AA40`, `0x00709A30`, plus
signature check on `0x004D94B0`), `get_xrefs_to` ×5 (`0x0041AA80`, `0x0051AA40`,
`0x00741970`, `0x004D94B0`, implicit vtable-slot reads), `get_function_callers` ×3
(`0x004D94B0`/`FootClass__Set_Destination_Internal`, `UnitClass__EnterBuildingOrDock`,
`FUN_0051aa40`), `get_function_by_address` ×2, `search_functions` ×3
(`Set_Destination_Internal`, `EnterBuildingOrDock`, `Set_Destination`),
`get_function_signature` ×1 (`FootClass__Set_Destination_Internal`).
