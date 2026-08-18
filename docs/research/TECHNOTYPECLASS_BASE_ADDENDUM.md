# TechnoTypeClass Base — Addendum (corrections + extensions)

**Companion to:** [TECHNOTYPECLASS_BASE_GHIDRA_REPORT.md](TECHNOTYPECLASS_BASE_GHIDRA_REPORT.md)
**Source:** Independent re-execution of the same investigation plan
(`docs/plans/2026-04-24-technotypeclass-base-investigation-plan.md`) by a
parallel research run. Findings below are the **delta** vs the canonical doc —
items that the canonical doc missed, got wrong, or left as open questions.

> Read the canonical report first; this addendum assumes its structure and
> field tables. Backport individual sections into the canonical doc only after
> spot-verifying each in Ghidra (the canonical doc was structured and well-cited
> — most of its content stands; this addendum corrects 8 specific points).

---

## A1. `GetSpeed @ 0x00717800` is mislabeled — it is `GetFlightLevel`

**Severity:** HIGH — implementing this against the canonical name will route
movement-speed lookups through an aircraft-altitude getter and break parity.

The canonical report's §7.3 describes `GetSpeed` as "Returns raw base speed
only" reading from `+0x618`. But the canonical report's own field table at
line 364 correctly labels offset `0x618` as **`FlightLevel`** (default `-1`,
fallback `Rules+0x7B4`). These two statements contradict each other.

### Evidence

The function is 5 instructions:

```asm
00717800: MOV   EAX, [ECX + 0x618]       ; FlightLevel
00717806: CMP   EAX, -1
00717809: JNZ   0x00717816
0071780b: MOV   EAX, [0x008871e0]        ; g_RulesClass_Instance
00717810: MOV   EAX, [EAX + 0x7b4]       ; Rules.FlightLevel
00717816: RET
```

- Field at `+0x618` is `FlightLevel` per the canonical doc itself (line 364) AND
  per `FLY_LOCOMOTION_CLASS_GHIDRA_REPORT.md` (which documents `0x618` as
  per-type aircraft cruise altitude in leptons, parsed from `[TypeName]
  FlightLevel=` with default `-1` and Rules-global fallback).
- `Rules+0x7B4` is also `FlightLevel` per the same doc (default 500, rulesmd
  override 1500).
- The string `"Speed"` at `0x0081d9cc` is parsed by `TechnoTypeClass::Read_INI`
  at `0x0071464C`, scaled `(raw * 256) / 100` clamped to `0xFF`, and stored at
  **`+0x678`** — NOT `+0x618`. The real unit-movement speed field is at
  `+0x678`.
- Caller xref: `FlyLocomotionClass::Begin_Takeoff @ 0x004CF950` invokes this
  function to set the aircraft's cruise altitude on takeoff. No
  movement-speed call site invokes `0x00717800`.

### Impact

The function should be referred to as **`TechnoTypeClass::GetFlightLevel`** in
all parity-critical contexts. Any Rust code routing skirmish unit speed
lookups through this function will read aircraft cruise altitude and produce
wildly wrong values for ground units. The Speed getter (real movement speed)
is inlined at every call site reading `+0x678` directly; there is no
`GetSpeed` accessor.

### Recommended action

1. Backport the rename in the canonical doc's §4.1, §7.3, and the field table
   line 364 caption.
2. Add a Ghidra rename request: `TechnoTypeClass__GetSpeed @ 0x00717800` →
   `TechnoTypeClass__GetFlightLevel`. Per CLAUDE.md, request user approval
   before relabeling.

Confidence: **HIGH** (~95%) — direct disassembly + matching FlightLevel
references in two independent reports.

---

## A2. Function at `0x00711AE0` is the destructor, NOT a constructor overload

**Severity:** MEDIUM — labeling impacts every reader of the doc; behavior
analysis already correct.

The canonical doc's §4.1 lists three "TechnoTypeClass::TechnoTypeClass"
constructor overloads at `0x00710AF0`, `0x00711840`, and `0x00711AE0`. Phase 2
re-decompilation provides 5-point evidence that `0x00711AE0` is actually
**`TechnoTypeClass::~TechnoTypeClass`** (the non-virtual / "complete-object"
destructor) — and `ObjectTypeClass__Constructor` called at its tail is
likewise the ObjectType destructor.

### Evidence

1. **Vtable rewrite first.** It re-installs the TechnoType vtables at offsets
   0/4/8/0xC at the *top* of the function. Constructors set vtables before
   field init (necessary so virtual dispatch works during base-class
   construction); destructors *also* re-install own vtable so virtual
   dispatch resolves to the correct owner during destruction. The dead
   giveaway is that THIS function does the rewrite *before* the embedded list
   teardowns — a ctor would not need to write vtables in the middle of init,
   only at the start.
2. **Guarded `free()` patterns.** Multiple instances of
   `if (ptr != 0 && flag != 0) { FUN_007c8b3d(ptr); ptr = 0; flag = 0; }` —
   the canonical "release allocated buffer" pattern. `FUN_007c8b3d` is
   `operator delete` / `free`. Constructors don't free.
3. **Unregisters from global vectors** `DAT_00b0f670` and `DAT_00a8eb00` via
   vtable-10 find-then-remove. The TechnoType ctor *appends* to these same
   vectors — the destructor unwinds it.
4. **Embedded list teardowns** at 18 offsets matching the Load/Save list
   inventory (`0x794, 0x778, 0x748, 0x72C, 0x654, 0x638, 0x5C4, 0x510, 0x4F4,
   0x4D8, 0x4BC, 0x4A0, 0x484, 0x468, 0x44C, 0x430, 0x414, 0x3E8, 0x330,
   0x314`). For each: re-assigns the slot's vtable then calls the slot's
   "clear" virtual. This is destruction (drain + release), not construction.
5. **Super-chain order.** The function calls `ObjectTypeClass__Constructor`
   (Ghidra label) **at the end**, after all field/list teardown. Constructors
   chain super-class init *before* their own work; destructors chain super
   *after*. Order is wrong for a ctor.

### Impact

Functional analysis in the canonical doc is unaffected (no behavior was
attributed to this function). Only the *name* is wrong. Future readers
seeing "Constructor overload" will misunderstand the call graph.

### Recommended action

1. Backport: §4.1 of the canonical doc — relabel `0x00711AE0` as
   `TechnoTypeClass::~TechnoTypeClass` (non-virtual dtor); the entry at
   `0x007179A0` is the **scalar-deleting** dtor (vtable slot 8) which calls
   `0x00711AE0` then optionally `free(this)`.
2. Ghidra rename request: `TechnoTypeClass__Constructor @ 0x00711AE0` →
   `TechnoTypeClass__Destructor` (and the same logic applies to whatever
   function is currently labeled `ObjectTypeClass__Constructor` and called at
   its tail — that is `ObjectTypeClass::~ObjectTypeClass`).

Confidence: **HIGH** (~95%).

---

## A3. Load/Save serialize 20 lists; Size accounts for only 18

**Severity:** LOW (savefile correctness) / MEDIUM (parity audit).

The canonical doc's §7.4–§7.6 says Load/Save/Size each cover **18**
variable-length lists. Re-decompilation finds:

- **Load** (`0x007162F0`): drains 18 lists in pre-phase, **deserializes 20
  lists** in main-phase.
- **Save** (`0x00716DC0`): **serializes 20 lists** mirroring Load.
- **Size** (`0x007170A0`): accounts for **18 lists** — omits the lists at
  `0x638` (count `0x648`) and `0x654` (count `0x664`).

### Evidence

Save loop count/buffer pairs (20 entries):

```
0x3F8/0x3EC, 0x324/0x318, 0x5D4/0x5C8, 0x424/0x418, 0x440/0x434,
0x45C/0x450, 0x478/0x46C, 0x494/0x488, 0x4B0/0x4A4, 0x4CC/0x4C0,
0x4E8/0x4DC, 0x504/0x4F8, 0x520/0x514, 0x73C/0x730, 0x758/0x74C,
0x340/0x334, 0x648/0x63C, 0x664/0x658, 0x788/0x77C, 0x7A4/0x798
```

Size loop omits the entries at `0x648/0x63C` (Prerequisite list at `0x638`,
per the canonical doc field table) and `0x664/0x658` (PrerequisiteOverride
list at `0x654`). For runs where these lists are non-empty, `GetSizeMax`
underestimates the serialized footprint by `8 + 4*(count_at_0x648 +
count_at_0x664)` bytes.

### Likely cause

The two omitted lists are `Prerequisite` and `PrerequisiteOverride`. In stock
YR `rulesmd.ini`, every techno-type has at least one Prerequisite entry, so
the omission is observable. Either:

(a) GetSizeMax is buggy (TS-era oversight that survived);
(b) Prerequisite/PrerequisiteOverride are saved by `ObjectTypeClass::Save`'s
    super-call rather than by `TechnoTypeClass::Save`, so the total Size
    accounting still works out (Size's super-call would account for them).

Quick test: read `ObjectTypeClass::GetSizeMax @ 0x005F9970` and check whether
it walks any TechnoType-range lists. If yes, (b) is the answer; if no, (a).

### Impact

If the savefile actually stores fewer bytes than `GetSizeMax` predicts, no
breakage. If it stores MORE, the writer over-runs the buffer — but in
practice IStream::Write doesn't pre-allocate based on GetSizeMax for variable
streams. Probably benign in YR but worth flagging in the canonical doc.

### Recommended action

Add to canonical §7.6 a note that Size omits the Prerequisite and
PrerequisiteOverride lists relative to Save's 20-list inventory, with a
"likely covered by ObjectType super or benign omission" qualifier pending the
super-call check.

Confidence: **HIGH** on the count discrepancy; **MEDIUM** on the cause.

---

## A4. CRC skips list count at `0x45C`

**Severity:** LOW (replay determinism — already runs from same INI on all
peers, so per-element list contents are stable).

Of the 20 list-count fields covered by Load/Save, `Compute_CRC @ 0x007171A0`
hashes 19. Missing: **`0x45C`** (count for the list at `0x44C`, which uses
the `FUN_00477EC0` insert helper — `TypeList<int>`).

### Evidence

Decomp lists CRC inputs in this order: `0x424, 0x440, 0x478, 0x494, 0x4B0,
0x4CC, 0x4E8, 0x504, 0x520, 0x73C, 0x758, 0x788, 0x7A4, 0x324, 0x5D4, 0x3F8,
0x340, 0x648, 0x664`. Count `0x45C` is conspicuously absent from the sequence
between `0x440` and `0x478`.

### Cause

Almost certainly an oversight in the original CRC enumeration — the
surrounding entries form a regular stride (0x430, 0x44C, 0x468, 0x484, ...
list buffer offsets) and the omitted slot fits that pattern.

### Impact

For lockstep determinism, all peers parse the same `rulesmd.ini` so the list
content is identical pre-Save; CRC mismatch on this slot would only arise if
peers' parses diverged, which they shouldn't. Effectively benign in
practice but a real omission.

### Recommended action

Note in canonical §7.7 that CRC covers 19 of 20 list counts; the omitted
`0x45C` is likely an original-engine oversight.

Confidence: **HIGH**.

---

## A5. Save has 11 trailing scalar writes with no Load counterpart

**Severity:** LOW (likely TS-era dead code).

After the 20-list serialization, `Save @ 0x00716DC0` makes 11 additional
4-byte writes via wrapper helpers (`FUN_0067a520` ×9, `FUN_0067a4a0` ×2,
`FUN_00717b20` ×2). Each helper reads `*(this + 0x10)` of an embedded
list-header subobject and writes 4 bytes to the stream.

### Pattern

The `+0x10` field of each list header is a cached count or "max" value. These
writes are not mirrored in Load — `Load @ 0x007162F0`'s post-list phase only
does the 12 pointer-swizzle resolutions and then re-reads Cameo/AltCameo/
Palette from the INI. Nothing reads back these 11 scalar slots from the
stream.

### Impact

Bytes written to the stream that are then ignored on reload. Wastes 44 bytes
per saved TypeClass in the savefile. Effectively dead writes inherited from
TS. A from-scratch implementation can omit them; the savefile format will be
binary-incompatible with original gamemd.exe saves anyway because of internal
struct layout, so this is a "consider matching for round-trip parity"
question rather than a correctness one.

### Recommended action

Add to canonical §7.5 a note that 11 trailing scalar writes have no Load
counterpart — likely legacy/dead writes inherited from Tiberian Sun's
TypeClass save pattern. Safe to omit in any from-scratch implementation.

Confidence: **HIGH** on the count and asymmetry; **MEDIUM** on the
"TS legacy" attribution.

---

## A6. Pointer-swizzle mechanism: raw pointers + session-scoped scratch

**Severity:** MEDIUM (savefile / replay correctness).

The canonical doc §7.4 mentions "12 `FUN_006cf240` calls" for pointer fixups
but doesn't explain the mechanism. The full picture:

### How it works

1. **Save** writes pointer fields **as their raw 32-bit value** at save time
   (i.e., the literal `this`-style address from the saving process). No
   index-flattening pass is done; the stream just contains the pointer bits.
2. At Load time, `AbstractClass::Load @ 0x00410380` (called as the
   inheritance super-chain bottoms out for every object) registers an
   `{old_this → new_this}` mapping in a session-scoped scratch table at
   `&DAT_00b0c110`, via helper `FUN_006cf2c0(table, old_this, new_this)`.
3. For each pointer field that needs fixing up, `FUN_006cf240(table,
   &field)`:
   - Reads `iVar1 = *field` (the raw 4-byte value just deserialized).
   - If `iVar1 == 0`: returns 0 (NULL stays NULL).
   - Otherwise appends `{iVar1, &field}` to a "pending swizzle queue" inside
     the same table.
   - Sets `*field = 0` (the field is a pending promise).
4. After all objects have loaded, a resolver pass (in
   `FUN_006cf490`/`FUN_006cf4d0`/`FUN_006cf4e0` — the table's vtable methods)
   walks the pending-queue, looks each `old_pointer` up against the
   `{old → new}` registry, and writes the resolved `new_pointer` into each
   `&field` location.

### Structure of `DAT_00b0c110`

It is a composite swizzle-context object, not the global TypeClass registry
(which lives at `DAT_00b0f674` / `DAT_00a8eb04`). Layout (deduced from the
helper accessors):

| Offset | Purpose |
|---|---|
| 0x04 | swizzle-queue vtable |
| 0x08 | swizzle-queue data ptr |
| 0x0C | swizzle-queue count |
| 0x14 | swizzle-queue count (also) / append-cursor |
| 0x18 | swizzle-queue grow-step |
| 0x1C | registry (old→new) vtable |
| 0x20 | registry data ptr |
| 0x24 | registry count |
| 0x2C | registry count (also) / append-cursor |
| 0x30 | registry grow-step |

The registry stores `{old_this, new_this}` pairs (8 bytes each); the queue
stores `{old_pointer, &pending_field}` pairs (8 bytes each). Both grow
dynamically.

### Bootstrap registry entries

A few "bootstrap" registry entries are populated at rules-load time
(*before* any save exists), so that fields pointing to global rules-time
objects can be resolved immediately. Examples:
`WarheadTypeClass::Constructor`, `TiberiumClass::Constructor`,
`TagTypeClass`, `TaskForceClass` constructors all register
`{name_hash → this}` pairs via `FUN_00412610(name)` (the hex-hashed name
function).

### Recommended action

Replace canonical §7.4's "Pointer fixups: 12 `FUN_006cf240` calls" line with
a short paragraph documenting the swizzle mechanism. The 12 specific offsets
(`0x404, 0x408, 0x40C, 0x624, 0x628, 0x6B8, 0x6BC, 0x764, 0x774, 0xD18,
0xD40, 0xD58`) all hold 4-byte pointers to other TypeClass instances —
those are the fields swizzled.

Confidence: **HIGH** — verified via decomp of `FUN_006cf240`, `FUN_006cf2c0`,
and inspection of `DAT_00b0c110`'s memory layout.

---

## A7. Vtable extends past slot 31; GetBuildTime is at slot 34

**Severity:** MEDIUM — affects any code routing virtual calls through the
TechnoTypeClass vtable.

The canonical doc's §4 vtable table stops at slot 31 (byte 0x7C). §4.1 lists
`GetBuildTime @ 0x00711EE0` as a "non-vtable" method. This is incomplete:
**GetBuildTime is in the primary vtable at slot 34, byte offset 0x88.**

### Evidence

Raw read of `0x007F4ED8 + 0x80 .. + 0xA0`:

| Slot | Byte | Target | Identity |
|---|---|---|---|
| 32 | 0x80 | `0x004C9150` | Stub__ReturnZero |
| 33 | 0x84 | `0x00711F00` | Vtable-relative scalar compute (multiplies 2 floats; likely a coord helper) |
| **34** | **0x88** | **`0x00711EE0`** | **`TechnoTypeClass::GetBuildTime`** |
| 35 | 0x8C | `0x004C9150` | Stub__ReturnZero |
| 36 | 0x90 | `0x005F7640` | (no function defined; unresolved) |
| 37 | 0x94 | `0x005F7900` | (no function; unresolved) |
| 38 | 0x98 | `0x00712040` | (no function; unresolved) |
| 39 | 0x9C | `0x0041CFA0` | (no function; unresolved) |

Vtable size is **at least 40 slots (160 bytes)**, possibly longer — the read
was bounded at offset 0x80 + 0x80.

### Impact

Subclass virtual dispatches to `GetBuildTime` go through slot 34. The
canonical doc's "non-vtable" attribution would mislead an implementation that
tries to recreate the dispatch table.

### Recommended action

Extend canonical §4's vtable table to at least slot 39, move GetBuildTime
into the table (slot 34), and remove or recharacterize the "non-vtable"
section in §4.1. Slots 36–39 remain unresolved — Ghidra has not analyzed
those addresses as functions, so a `create_function` pass is needed before
they can be characterized.

Confidence: **HIGH** for the read; **MEDIUM** for slot identities (slots
36–39 are unanalyzed targets).

---

## A8. List-helper element types resolved via RTTI (canonical doc Open Q #7)

**Severity:** LOW — closes an open question.

The canonical doc's §12 Open Question #7 flags: "Phase 2b found that
`FUN_00525680` installs `0x007EB6F4` (not `0x007EB6D4` as an earlier note
suggested) and `FUN_0045AD80` installs `0x007E4424`."

All 5 list-helper element types are recoverable from the MSVC RTTI Class
Descriptor strings reachable via each helper's outer-class COL:

| Helper | Calls in TT::Ctor | Vtable written | RTTI class descriptor | Element type |
|---|---|---|---|---|
| `FUN_00525680` | 3× | `0x007EB6F4` | `.?AV?$TypeList@PBVAnimTypeClass@@@@` | `TypeList<AnimTypeClass*>` |
| `FUN_0045AD80` | 2× | `0x007E4424` | `.?AV?$TypeList@PBVParticleSystemTypeClass@@@@` | `TypeList<ParticleSystemTypeClass*>` |
| `FUN_00477BE0` | 13× | `0x007E4DB8` | `.?AV?$TypeList@H@@` | `TypeList<int>` |
| `FUN_0067C310` | 1× | `0x007F0D5C` | `.?AV?$TypeList@PBVVoxelAnimTypeClass@@@@` | `TypeList<VoxelAnimTypeClass*>` |
| `FUN_005105A0` | 1× | `0x007EAA08` | `.?AV?$TypeList@PBVBuildingTypeClass@@@@` | `TypeList<BuildingTypeClass*>` |

All five are structurally identical `VectorClass<T>::VectorClass(int capacity,
T* buffer)` constructors — MSVC emitted 5 instantiations differing only in
the vtable pointer written. The `count = 0; capacity = 10` two-phase init
is done by callers after the helper returns; the helper itself just installs
vtable + sets `IsValid=1; IsAllocated=0`.

### Recommended action

Replace canonical §12 Open Question #7 with a resolved entry citing this
RTTI lookup, and add a new "List helpers" subsection to §9 (Constructor
Notes) listing the 5 helpers with their element types.

Confidence: **HIGH** — RTTI class descriptors are authoritative.

---

## Cross-reference: open questions from the canonical doc that we did NOT resolve

For completeness, items in the canonical doc's §12 that this addendum does
*not* close:

- **Q1 / Q6** (Cost_Of vs Get_Ownable confusion at `0x00711EC0`) — the
  function reads `+0xC99` and `+0x6CC`, not `+0x610`. Our investigation also
  did not pin its true semantics. Still open.
- **Q2** (Unnamed ctor magic constants) — partially overlaps with our Phase 1
  finding of doubles at `[0xC0..0xC3]` (`~0.002`), `[0xC2..0xC3]` (`~0.030`),
  and the `~0.524` / `~0.349` radian defaults — but these are `Acceleration`/
  `Deacceleration` / `RollAngle` / `PitchAngle` defaults, all attributed in
  the canonical field table. Other miscellaneous int defaults remain
  unverified.
- **Q3** (18×7-DWORD inline weapon-slot tables) — not decoded; both runs
  confirmed the loop strides but neither extracted per-slot field semantics.
- **Q4** (`FUN_00717AE0`) — partially clarified in our Phase 1 (predicate
  used after Cameo/AltCameo to gate post-load voxel-hotspot computation); but
  exact semantics of the `[0x360]/[0x368]` writes still inferred.
- **Q5** (CDFileClass branch) — agreed dormant; both runs noted but did not
  trace.
- **Q8** (Subclass base-range patches) — both runs confirmed these are
  intentional default-overrides, not bugs (UnitType: SpeedType from JumpJet
  flag; BuildingType: SpeedType from WaterBound; InfantryType: derived
  Occupier flag at `0xC8F`).

---

## Sources

- Live Ghidra MCP decomp of:
  - `TechnoTypeClass::Read_INI @ 0x00712170` (verified key→offset map of ~260
    distinct keys; cross-checked against the canonical field table — full
    agreement except for the `+0x678` "Speed-from-INI scaled to byte"
    field, which the canonical doc lists as `Crushability(?)` at line 376
    but is actually the post-scaled `Speed` field reached by INI key
    `Speed=`)
  - `TechnoTypeClass::Constructor @ 0x00710AF0`
  - `TechnoTypeClass::Load @ 0x007162F0`, `Save @ 0x00716DC0`,
    `GetSizeMax @ 0x007170A0`, `Compute_CRC @ 0x007171A0`
  - `TechnoTypeClass::~TechnoTypeClass @ 0x00711AE0` (currently mislabeled in
    Ghidra)
  - `TechnoTypeClass::GetBuildTime @ 0x00711EE0` (5-instruction full trace)
  - `TechnoTypeClass::GetFlightLevel @ 0x00717800` (currently mislabeled
    `GetSpeed` in Ghidra)
  - `FUN_006cf240` swizzle-append, `FUN_006cf2c0` registry-append
  - 5 list-helper functions with RTTI class-name lookup
  - 4 subclass `Read_INI` super-call sites confirmed
  - TechnoTypeClass primary vtable @ `0x007F4ED8` extended to slot 39
- Cross-referenced research:
  - `FLY_LOCOMOTION_CLASS_GHIDRA_REPORT.md` — confirms `FlightLevel` field
    semantics
  - `BULLET_PROJECTILE_SYSTEM_CONSOLIDATED_REPORT.md`,
    `AIRCRAFTCLASS_GHIDRA_REPORT.md` — corroborate `Rules+0x7B4` =
    `FlightLevel`
  - `POWER_SYSTEM_GHIDRA_REPORT.md` — mentions the 0.9 BuildSpeed factor
    semantically
- **Investigation plan:**
  `docs/plans/2026-04-24-technotypeclass-base-investigation-plan.md` —
  executed in 3 phases (Phase 1: 6 fns, Phase 1.5 checkpoint, Phase 2: 10
  fns + TS-legacy traces + amendment helpers, Phase 3: 4 subclass ReadINI
  spot-checks + vtable extension + swizzle trace)

No `.rs` files modified. No Ghidra labels changed. No Rust written. The two
Ghidra rename recommendations (A1, A2) are flagged for user approval per
CLAUDE.md's ~90% confidence rule.
