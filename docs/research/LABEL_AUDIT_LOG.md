# Ghidra label audit log — gamemd.exe

Running log of labeling decisions, mislabel fixes, and significant rename events from
the iterative Ghidra labeling pass. Cross-reference with `ghidra_loop_state.md` for
the live focus tracks and pending candidates.

---

## 2026-05-17 — BuildingClass destructor mislabel + track close (round 39)

**Focus track:** `combat-instance-arrays` — CLOSED.

### Mislabel fix (6th of session)

- **`0x0043bcf0`** `BuildingClass__Constructor` → `BuildingClass__Destructor`

  Smoking gun signals:
  - Sets vtable at top (MSVC destructor convention)
  - Calls `Detach_From_All_Lists` (the master observer dispatcher,
    renamed from FUN_007258d0 in round 32)
  - Calls `BuildingClass__Limbo` (removes from game world)
  - DECREMENTS counts on multiple arrays via vtable[+0x10]:
    `g_BuildingClass_Array`, `DAT_008b41e0` (unknown secondary),
    `g_AnimClass_RemoveListeners`, `g_FactoryClass_RemoveListeners`,
    tagged-pointer heap `DAT_00b0e840`
  - Ends with chained call to `MissionClass__Constructor` — LIKELY
    also a destructor mislabel (the chained parent destructor).

### Real BuildingClass full constructor at 0x0043b740

Verified bindings:
- Pushes self into `g_BuildingClass_Array` (already labeled in prior work)
- Registers as observer of `g_FactoryClass_RemoveListeners` (Factory owns
  building production; when Factory dies, in-progress buildings need
  cleanup)
- Registers as observer of `g_AnimClass_RemoveListeners` (buildings have
  attached animations like construction effects that need notification)
- Registers in tagged-pointer heap `DAT_00b0e840`

This confirms BuildingClass's role in the observer subscription topology.

### Combat-instance-arrays track close-out

All 5 combat-instance class arrays VERIFIED:
- BulletClass → g_BulletClass_Array
- InfantryClass → g_InfantryClass_Array
- UnitClass → g_UnitClass_Array
- AircraftClass → g_AircraftClass_Array
- BuildingClass → g_BuildingClass_Array

Plus AnimClass (`g_AnimClass_Array`) from prior work.

Plus 1 mislabel fix (BuildingClass destructor).

Plus methodology lesson #10: FootClass-derived construction pattern.

Plus architectural findings:
- Locomotor is COM-based (`CoCreateInstance` with rules.ini CLSIDs)
- Cross-class tagged-pointer heap at `DAT_00b0e840`
- BuildingClass is the only combat-instance class that's NOT
  FootClass-derived (no Locomotor needed)

### HYPOTHESIS: MissionClass destructor mislabel

The BuildingClass destructor ends with a call labeled
`MissionClass__Constructor`. In MSVC destructor chains, the parent
destructor is invoked. So `MissionClass__Constructor` at that address
is LIKELY actually `MissionClass__Destructor` (the 7th destructor
mislabel candidate of the session).

Investigate in a future iteration.

---

## 2026-05-17 — Unit + Aircraft instance arrays + FootClass pattern confirmed (round 38)

**Focus track:** `combat-instance-arrays` (continuing).

### 4 new labels

- `0x008b410c` → `g_UnitClass_Array`
- `0x008b4118` → `g_UnitClass_Array_Count`
- `0x00a8e394` → `g_AircraftClass_Array`
- `0x00a8e3a0` → `g_AircraftClass_Array_Count`

Both verified via vtable-as-ground-truth + the matching FootClass-derived
construction pattern.

### FootClass-derived construction pattern (now verified 3 times)

All three classes (InfantryClass, UnitClass, AircraftClass) follow the
exact same construction sequence:

1. `FootClass__Constructor(param_3)` — parent
2. Initialize class-specific state
3. Set leaf vtable to `vtable__<X>Class`
4. `AbstractClass__AssignUniqueID`
5. Push self into per-class array (`g_<X>Class_Array`)
6. `CoCreateInstance` Locomotor COM object from `type+0x34c` CLSID
7. Push self into tagged-pointer heap (`DAT_00b0e840`)
8. Call `<X>Class__InitFromType` (or inline equivalent)

This is the **FootClass-derived instance construction pattern** — locked
in as methodology lesson #10. Any future FootClass-derived instance
constructor can be identified by this signature.

### BuildingClass is the exception

BuildingClass has 3 constructor variants:
- 0x0043b680 — minimal/load (calls TechnoClass__Constructor, sets vtable,
  doesn't push to any per-class array directly)
- 0x0043b740 — TBD
- 0x0043bcf0 — TBD

The first variant is clearly NOT the full constructor. Buildings don't
move, so no Locomotor CoCreateInstance is needed — but they should
still have a per-class instance array. The full constructor is at one
of the other addresses.

### Combat-instance subsystem near complete

Labeled: BulletClass, InfantryClass, UnitClass, AircraftClass.
Plus prior work: AnimClass (`g_AnimClass_Array`).
Pending: BuildingClass.

---

## 2026-05-17 — Bullet + Infantry instance arrays + Locomotor-is-COM finding (round 37)

**Focus track:** `combat-instance-arrays` (new active track).

### 4 new labels

- `0x00a8ed44` → `g_BulletClass_Array`
- `0x00a8ed50` → `g_BulletClass_Array_Count`
- `0x00a83dec` → `g_InfantryClass_Array`
- `0x00a83df8` → `g_InfantryClass_Array_Count`

Verification via vtable-as-ground-truth.

### Architectural finding 1: Locomotor is COM

InfantryClass__Constructor @ 0x00517a50 instantiates the unit's
Locomotor (movement controller) via the Windows COM API:
- `CoCreateInstance` with the Locomotor CLSID from InfantryType+0x34c
- `OleRun` to start the object
- `QueryInterface` for the actual ILocomotor pointer
- Stored at infantry+0x19d

This means **Locomotors are real OLE/COM objects** in gamemd.exe, with
the CLSID-based dispatch documented in rules.ini (e.g.,
`Locomotor={4A582746-9839-11d1-B709-00A024DDAFD1}`). The Rust port can:
- Mirror exactly: use a trait + dynamic dispatch with CLSID lookup
- Simplify: use an enum of known locomotor variants (sufficient for
  YR's fixed locomotor set)

Same likely applies to UnitClass and AircraftClass (also have
Locomotor= INI fields).

### Architectural finding 2: Secondary tagged-pointer heap

InfantryClass also registers self in a secondary structure at
`DAT_00b0e840` (size at `DAT_00b0e844`, capacity at `DAT_00b0e848`,
free-slot index at `DAT_00b0e84c`). Entries are 8 bytes — RTTI (4) +
pointer (4). Uses a growth-doubling pattern (capacity doubles when
full, copying old entries).

This is a different DV-like structure from the standard 6-field
DynamicVectorClass. Likely role: cross-class RTTI-keyed lookup table.
Need to investigate which other classes push here.

### Combat-instance subsystem now in progress

Labeled so far in this track: BulletClass, InfantryClass.
Pending: UnitClass, AircraftClass, BuildingClass.

Plus AnimClass was labeled in earlier work (`g_AnimClass_Array`).

---

## 2026-05-17 — TriggerClass + ScriptClass arrays (round 36)

**Focus track:** `mission-scripting-instance-arrays` — substantially closing.

### 4 new labels

- `0x00a8eaec` → `g_TriggerClass_Array`
- `0x00a8eaf8` → `g_TriggerClass_Array_Count`
- `0x008872b4` → `g_ScriptClass_Array` (FIRST instance array for RTTI 0x1a class)
- `0x008872c0` → `g_ScriptClass_Array_Count`

### Architectural finding: Isolated Subsystem Pattern

ScriptClass__Constructor @ 0x006913c0 pushes self ONLY into
`g_ScriptClass_Array`. Does NOT push to:
- Master Abstract registry (`DAT_00b0f674`)
- Any observer-list array
- Any RTTI-keyed dispatch list

This matches the AI type exclusion pattern (AITriggerType also opts out
of the master registry). The two known isolated classes:
- **ScriptClass** — owned by TeamClass (TeamClass__Constructor creates
  ScriptClass via `operator_new(0x30)` + `ScriptClass__Constructor`)
- **AITriggerTypeClass** — AI subsystem

Both classes are owned by parent objects and rely on parent lifecycle
for cleanup. They don't need to participate in the engine-wide
observer/notification system.

### Significance

The engine has **at least 3 lifecycle-management patterns** for classes:
1. **Per-class destructor only** — simple game-world entities (Unit,
   Building, Bullet, Terrain, etc.). Cleanup is local.
2. **Observer-subscriber pattern** — classes with complex
   relationships (House, Anim, Tag, Team, Trigger, Factory, Neuron).
   They register in others' RemoveListeners arrays.
3. **Isolated subsystem** — owned by parent, no registry participation
   (ScriptClass, AITriggerTypeClass).

Each pattern has clear semantic boundaries. The Rust port can model
these explicitly with different ownership/borrowing strategies for
each category.

### Mission-scripting instance arrays now complete (6)

- TActionClass (RTTI 0x2f)
- TEventClass (RTTI 0x30)
- TagClass (RTTI 0x2c)
- TeamClass (RTTI 0x22)
- TriggerClass (RTTI 0x26)
- ScriptClass (RTTI 0x1a)

All bound via vtable-as-ground-truth across rounds 33-36.

---

## 2026-05-17 — Tag + Team instance arrays + observer subscription topology (round 35)

**Focus track:** `mission-scripting-instance-arrays` (new active track).

### 4 new labels

- `0x00b0e724` → `g_TagClass_Array`
- `0x00b0e730` → `g_TagClass_Array_Count`
- `0x008b40ec` → `g_TeamClass_Array`
- `0x008b40f8` → `g_TeamClass_Array_Count`

Verification via vtable-as-ground-truth (TagClass→vtable__TagClass→
DAT_00b0e724, TeamClass→vtable__TeamClass→DAT_008b40ec).

### Architectural breakthrough: Observer Subscription Topology

Decompiling the TagClass/TeamClass/TriggerClass constructors revealed the
observer subscription topology — which classes register to receive
removal notifications from which other classes:

| Subscriber  | Watches for removal of | Source                   |
|-------------|------------------------|--------------------------|
| TagClass    | TriggerClass           | TagClass__Constructor @ 0x006e4de0 |
| TeamClass   | TagClass, NeuronClass  | TeamClass__Constructor @ 0x006e8a90 |
| TriggerClass| TriggerClass (self/sibling) | TriggerClass__Constructor @ 0x00725fa0 |
| SuperClass  | HouseClass             | (round 26)                |
| TechnoClass | HouseClass             | (round 26)                |

This is the **engine's encoded game-logic dependency graph**:
- Tags depend on Triggers (tag = trigger + condition + state)
- Teams depend on Tags AND Neurons (team = AI-controlled unit group)
- Supers/Technos depend on Houses (everything ownable belongs to a house)

When a parent dies, all dependent subscribers get a `NotifyOfRemoval`
callback (vtable[+0x28]) and can detach safely.

### Significance for parity

This subscription topology is the SOURCE OF TRUTH for how gamemd.exe
handles object-lifecycle relationships. A Rust port mirroring this
exactly would inherit the same cleanup semantics. The pattern is also a
useful debugging tool — any time the Rust port has a dangling-pointer
bug in mission scripting, the answer is "who needed to subscribe but
didn't?" — and the binary tells us.

### Verified-but-DEFERRED next iteration

- `0x00a8eaec` → `g_TriggerClass_Array`
- `0x00a8eaf8` → `g_TriggerClass_Array_Count`

Verified via TriggerClass__Constructor @ 0x00725fa0 setting
`vtable__TriggerClass` and pushing into `DAT_00a8eaec`.

---

## 2026-05-17 — TActionClass + TEventClass instance arrays (round 34)

**Focus track:** `tactionclass-teventclass-arrays` (new active track).

### 4 new labels (vtable-as-ground-truth VERIFIED)

- `0x00b0e65c` → `g_TActionClass_Array`
- `0x00b0e668` → `g_TActionClass_Array_Count`
- `0x00b0f1a4` → `g_TEventClass_Array`
- `0x00b0f1b0` → `g_TEventClass_Array_Count`

### Verification chain

**TActionClass__Constructor @ 0x006dd000:**
- Sets `*param_1 = &vtable__TActionClass`
- Pushes self into 3 arrays:
  1. `g_TActionClass_Array` (primary)
  2. `DAT_00b0f674` (master Abstract registry)
  3. `g_TriggerTypeClass_Array` (shared trigger storage)

**TEventClass__Constructor @ 0x0071e6a0:**
- Sets `*param_1 = &vtable__TEventClass`
- Pushes self into 3 arrays:
  1. `g_TEventClass_Array` (primary)
  2. `g_TriggerTypeClass_Array` (shared trigger storage)
  3. `DAT_00b0f674` (master Abstract registry)

### Architectural finding

`g_TriggerTypeClass_Array` is **shared storage** — it holds:
- TriggerTypeClass instances (the rules-defined trigger types)
- TActionClass instances (mission script actions)
- TEventClass instances (mission script events)

This explains why FUN_007258d0's dispatch table maps both RTTI 0x2f
(TAction) and 0x30 (TEvent) to this same array (round 26 finding now
fully understood).

The array's name is somewhat misleading — it's broader than "trigger
types". A more accurate name might be `g_TriggerStorage_Array` or
`g_TriggerSubsystem_Heap`. But renaming an established label would be
disruptive — leaving it and documenting.

---

## 2026-05-17 — Deferred-list cleanup + BeaconClass closed (round 33)

**Focus track:** `deferred-list-cleanup` (new active track).

### 3 new labels — long-deferred items committed

- `0x00673e80` `FUN_00673e80` → `RulesClass__ReadPowerups`
  - Role VERIFIED since round 6. Reads `[Powerups]` INI section, parses
    crate-bonus types (Money/Heal/Cloak/etc.).

- `0x0066d3a0` `FUN_0066d3a0` → `Init_Color_Schemes_INI`
  - Role VERIFIED since round 7. Reads `[Colors]` INI section, calls
    palette/remap helpers.

- `0x00674650` `FUN_00674650` → `RulesClass__ReadCommandBar`
  - Role VERIFIED since round 7. Reads `[AdvancedCommandBar]` /
    `[MultiplayerAdvancedCommandBar]` based on game-mode flag. Calls
    button-setup helpers 25× then parses ButtonList= INI override.

These had been deferred only because the EXACT canonical Westwood name
was uncertain. The committed names are descriptive and capture the role
clearly. If a more canonical name surfaces, one-step rename.

### BeaconClass vtable track — closed as THIN

Started this round; closed immediately. BeaconClass has no labeled
vtable in the binary — RA2/YR beacons are helper functions around
RadarClass + SHP rendering, not a separate class with vtable hierarchy.

The only Beacon-related labels are `Init_BeaconArt @ 0x004309d0`
and `RadarClass__PlaceBeacon @ 0x00430ba0`. Plus
`BeaconPlacementCommandClass` (the input command, separate concept).

Track closed as unfruitful with 2 labeled items.

### Verified-but-DEFERRED list shrunk significantly

After this round, the deferred list is down to:
- AI items (deferred per `feedback_no_ai_yet.md` — intentional skip)
- `DAT_00b0f674` master Abstract registry name (still MEDIUM
  confidence — `g_AbstractClass_Heap` is the leading candidate)

Both of these are acceptable deferrals; not blocking other work.

---

## 2026-05-17 — Track close: tracker-arrays-investigation (round 32)

**Focus track:** `tracker-arrays-investigation` — CLOSING.

### 3 new labels — closing the track

- `0x00b0f6f4` → `g_NeuronClass_RemoveListeners`
- `0x00b0f700` → `g_NeuronClass_RemoveListeners_Count`
- `0x007258d0` `FUN_007258d0` → `Detach_From_All_Lists`

NeuronClass binding verified in round 28 (RTTI 0x3c → DAT_00b0f6f4).

The `Detach_From_All_Lists` rename has been deferred since round 24.
Going with this name now: short, descriptive, matches Westwood verb-
first style, no class prefix (correct since it's a free helper). If a
more canonical name surfaces later, one-step rename.

### Track close-out summary

The tracker-arrays-investigation track ran from rounds 22-32 and produced:

**Architectural findings:**
- The RTTI-keyed observer-pattern dispatch in gamemd.exe
- 3 inferred AbstractClass vtable slots (+0x28 NotifyOfRemoval, +0x2c
  GetRTTIType, +0x64 Read_INI)
- Pattern: observer dispatch is for mission/scripting types + complex-
  relationship instances. Simple game-world entities (Unit/Building/
  Bullet/Terrain) use per-class destructor cleanup only.

**Concrete labels:**
- 31 RTTI values cataloged via the vtable[+0x2c] inspection technique
- 7 of 8 observer-list array pairs labeled (Anim/Factory/House/Tag/Team/
  Trigger/Neuron — only RTTI 0x33 class unknown)
- `Detach_From_All_Lists` named with comprehensive plate comment

**Methodology breakthroughs (3):**
- vtable-as-ground-truth (now 18+ applications)
- destructor pattern recognition
- vtable[+0x2c] raw-byte inspection for RTTI extraction

**Hypothesis revisions (5):**
- Round 10 position-based array guesses systematically wrong
- Round 14 effect-only registry was actually the universal Abstract registry
- Round 22 "Logics array" guess corrected to TagClass-removal observers
- Round 23 multiple-Constructor-labels doesn't imply destructor mislabel
- "Observer arrays" interpretation refined: arrays hold listeners, not
  instances of the dispatched class

### Track productivity assessment

This was the highest-yield track of the session:
- 31 RTTI mappings (~50% of all session work)
- 9 observer-list labels
- 2 major-architectural revelations (observer pattern, vtable slot map)

Most productive iteration block: rounds 26-28 (RTTI catalog expansion
via vtable inspection). Lesson: when a mechanical technique works well,
run it to completion before moving on.

### Outstanding work (deferred)

- RTTI 0x33 array (`DAT_00b0f5f4 / 600`) — class unknown, left as DAT_*
- Opportunistic catch when another class vtable surfaces

---

## 2026-05-17 — Team + Trigger observer-list renames (round 31)

**Focus track:** `tracker-arrays-investigation` (continuing).

### 4 new labels

- `0x00b0f5dc` → `g_TeamClass_RemoveListeners`
- `0x00b0f5e8` → `g_TeamClass_RemoveListeners_Count`
- `0x00b0f70c` → `g_TriggerClass_RemoveListeners`
- `0x00b0f718` → `g_TriggerClass_RemoveListeners_Count`

Bindings verified across rounds 26-27:
- TeamClass = RTTI 0x22 → DAT_00b0f5dc
- TriggerClass = RTTI 0x26 → DAT_00b0f70c

### Observer-list array tally: 6 of 7 labeled

| Class       | Array | Status |
|-------------|-------|--------|
| HouseClass  | g_HouseClass_RemoveListeners | ✓ |
| TagClass    | g_TagClass_RemoveListeners | ✓ |
| AnimClass   | g_AnimClass_RemoveListeners | ✓ |
| FactoryClass| g_FactoryClass_RemoveListeners | ✓ |
| TeamClass   | g_TeamClass_RemoveListeners | ✓ |
| TriggerClass| g_TriggerClass_RemoveListeners | ✓ |
| NeuronClass | DAT_00b0f6f4 (pending) | — |
| ? (RTTI 0x33) | DAT_00b0f5f4 (class unknown) | — |

One more iteration will close NeuronClass. The RTTI 0x33 array can be
renamed generically (e.g., `g_RTTI33_RemoveListeners`) or held until
the class identity surfaces.

---

## 2026-05-17 — Anim + Factory observer-list renames (round 30)

**Focus track:** `tracker-arrays-investigation` (continuing).

### 4 new labels

- `0x00b0f5bc` → `g_AnimClass_RemoveListeners`
- `0x00b0f5c8` → `g_AnimClass_RemoveListeners_Count`
- `0x00b0f644` → `g_FactoryClass_RemoveListeners`
- `0x00b0f650` → `g_FactoryClass_RemoveListeners_Count`

Both verified via the round-26+27 RTTI mapping:
- AnimClass = RTTI 0x04 → dispatch target DAT_00b0f5bc
- FactoryClass = RTTI 0x0c → dispatch target DAT_00b0f644

### RTTI 0x33 hunt — final non-result this iteration

SwizzleManagerClass and BaseClass vtable inspection didn't follow the
clean MOV/RET pattern — these are not AbstractClass-derived and have
different vtable layouts.

**Decision: ACCEPT RTTI 0x33 as unmapped.** Future iterations can pick
this up opportunistically. The dispatch entry's role is clear (observer
list for class with RTTI 0x33), only the class identity is unknown.

### Cumulative progress on the tracker investigation

- 4 of 7 observer-list array pairs labeled (HouseClass, TagClass,
  AnimClass, FactoryClass)
- 3 pending pairs (TeamClass, TriggerClass, NeuronClass)
- 1 unresolved (RTTI 0x33 class identity)

At 4 renames per iteration, the remaining 3 pairs will close in 1-2
more iterations.

---

## 2026-05-17 — First observer-list renames (round 29)

**Focus track:** `tracker-arrays-investigation` (continuing).

### 4 new labels (first observer-list array renames)

- `0x00b0f6cc` → `g_HouseClass_RemoveListeners`
- `0x00b0f6d8` → `g_HouseClass_RemoveListeners_Count`
- `0x00b0f61c` → `g_TagClass_RemoveListeners`
- `0x00b0f628` → `g_TagClass_RemoveListeners_Count`

Naming convention adopted: `g_<X>Class_RemoveListeners` — descriptive
and semantic. The exact canonical Westwood name is uncertain (could be
`g_<X>_DependantList`, `g_<X>_Watchers`, etc.) but the
"RemoveListeners" name accurately captures the semantic role: objects
registered to receive notification when an instance of class X is
removed.

### Bindings verified across rounds 24-26:

| RTTI | Class | Array | Plus action |
|------|-------|-------|-------------|
| 0xd | HouseClass | g_HouseClass_RemoveListeners ✓ | — |
| 0x2c | TagClass | g_TagClass_RemoveListeners ✓ | + UnregisterBridgeRepairHut |

### 3 more RTTI values this round

- AITriggerTypeClass = 0x3b
- TubeClass = 0x35
- BombClass = 0x44

None are RTTI 0x33 (the last unknown dispatch entry).

### Renames pending — 5 more observer-list array pairs

| Array | Class binding | Status |
|-------|---------------|--------|
| `DAT_00b0f5bc / 5c8` | AnimClass | pending |
| `DAT_00b0f644 / 650` | FactoryClass | pending |
| `DAT_00b0f5dc / 5e8` | TeamClass | pending |
| `DAT_00b0f70c / 718` | TriggerClass | pending |
| `DAT_00b0f6f4 / 700` | NeuronClass | pending |
| `DAT_00b0f5f4 / 600` | ? (RTTI 0x33 unknown) | pending |

Will apply at 4 renames per iteration over the next ~2 rounds.

---

## 2026-05-17 — RTTI catalog reaches 28 values (round 28)

**Focus track:** `tracker-arrays-investigation` (continuing).

### 12 more RTTI values mapped this round

| RTTI    | Class                |
|---------|----------------------|
| 0       | TechnoClass (base — returns 0; RadioClass inherits) |
| 2       | AircraftClass        |
| 0xc     | **FactoryClass** (dispatch match) |
| 0x1a    | ScriptClass          |
| 0x29    | VoxelAnimClass       |
| 0x2b    | WaveClass            |
| 0x36    | LightSourceClass     |
| 0x39    | SuperClass           |
| 0x3c    | **NeuronClass** (dispatch match) |
| 0x3d    | FoggedObjectClass    |
| 0x3e    | AlphaShapeClass      |
| 0x49    | DiskLaserClass       |

### Dispatch table near-complete

After this round, only 1 of 10 dispatch RTTI values is still unmapped
(RTTI 0x33 → `DAT_00b0f5f4`). All others are now bound to a class:

| RTTI | Class              | Tracker array               |
|------|--------------------|-----------------------------|
| 0x04 | AnimClass          | DAT_00b0f5bc                |
| 0x0c | **FactoryClass**   | DAT_00b0f644 + bridge       |
| 0x0d | HouseClass         | DAT_00b0f6cc                |
| 0x18 | ParticleSystemClass| DAT_00a8ed78 (singleton)    |
| 0x22 | TeamClass          | DAT_00b0f5dc                |
| 0x26 | TriggerClass       | DAT_00b0f70c                |
| 0x2c | TagClass           | DAT_00b0f61c + bridge       |
| 0x2f | TActionClass       | g_TriggerTypeClass_Array    |
| 0x30 | TEventClass        | g_TriggerTypeClass_Array    |
| 0x33 | ? (last unknown)   | DAT_00b0f5f4                |
| 0x3c | **NeuronClass**    | DAT_00b0f6f4                |

The bridge-repair-hut handling now makes perfect sense:
- **Factory removal (RTTI 0xc):** interrupts any in-progress building
  production including bridge repair huts
- **Tag removal (RTTI 0x2c):** clears tags on bridges

### Cumulative RTTI catalog (~28 values)

The catalog is now comprehensive enough to be a useful project reference.
Documented in state file.

### Hypothesis filter result (architectural clarity)

The observer-pattern dispatch covers exactly these class categories:
- **Mission/scripting types:** Tag, Team, Trigger, TAction, TEvent
- **Complex-relationship instances:** House, Anim, ParticleSystem, Factory, Neuron

NOT in dispatch:
- **Simple game-world entities:** Unit (1), Aircraft (2), Anim (4 — wait
  Anim IS in dispatch), Building (6), Bullet (8), Infantry (0xf), etc.
  Wait — AnimClass IS in dispatch (0x04). Let me re-classify:
  - AnimClass is "complex relationship" because anims attach to objects
    (puffs above buildings, wake trails, etc.)
- **Truly simple entities:** Unit, Aircraft, Building, Bullet, Infantry,
  Overlay, Terrain, Tiberium, Bullet — these use per-class destructor only

This is the architectural cleanliness signal: only objects with
non-trivial lifecycle relationships need observer notification.

### Pending: 1 unknown + renames

Next iteration target: find RTTI 0x33 by trying more class vtables
(AITriggerTypeClass / DropPodClass / TubeClass / BeaconClass etc.). Once
all 10 dispatch entries are bound, begin renaming the observer-list
arrays with the now-confirmed bindings.

---

## 2026-05-17 — RTTI catalog completed to 16 (round 27)

**Focus track:** `tracker-arrays-investigation` (continuing).

### 7 more RTTI values mapped this round

Using the vtable[+0x2c] inspection technique. All via raw-byte reading,
no decompilation needed:

| RTTI    | Class                | Vtable address                     |
|---------|----------------------|------------------------------------|
| 4       | AnimClass            | 0x007e3354                         |
| 6       | BuildingClass        | 0x007e3ebc (inferred from secondary)|
| 8       | BulletClass          | 0x007e46e4                         |
| 0x18    | ParticleSystemClass  | 0x007efb9c                         |
| 0x22    | TeamClass            | 0x007f4730                         |
| 0x2f    | TActionClass         | 0x007f443c                         |
| 0x30    | TEventClass          | 0x007f5578                         |

6 of 7 match dispatch entries:
- 0x04 (AnimClass) → `DAT_00b0f5bc`
- 0x18 (ParticleSystem) → `DAT_00a8ed78` (singleton)
- 0x22 (TeamClass) → `DAT_00b0f5dc`
- 0x2f (TActionClass) + 0x30 (TEventClass) → both → `g_TriggerTypeClass_Array`

BulletClass (0x08) is NOT in dispatch — consistent with the "simple
game-world entity" pattern (like Unit, Building, Terrain).

### Cumulative RTTI catalog (16 confirmed)

| RTTI   | Class                |
|--------|----------------------|
| 1      | UnitClass            |
| 4      | AnimClass            |
| 6      | BuildingClass        |
| 8      | BulletClass          |
| 0xd    | HouseClass           |
| 0xf    | InfantryClass        |
| 0x14   | OverlayClass         |
| 0x15   | OverlayTypeClass     |
| 0x18   | ParticleSystemClass  |
| 0x22   | TeamClass            |
| 0x24   | TerrainClass         |
| 0x26   | TriggerClass         |
| 0x2c   | TagClass             |
| 0x2e   | TiberiumClass        |
| 0x2f   | TActionClass         |
| 0x30   | TEventClass          |

3 RTTI values in the dispatch table still unknown: 0xc, 0x33, 0x3c.

### Pattern: what gets observer dispatch vs what doesn't

**Has observer dispatch (notification on removal):**
- House, Tag, Team, Trigger, TAction, TEvent — mission/scripting types
- Anim, ParticleSystem — complex-relationship game objects

**No observer dispatch (per-class destructor cleanup only):**
- Unit, Infantry, Building, Bullet — combat-unit types
- Overlay, OverlayType, Terrain, Tiberium — map terrain/decoration

The pattern reveals architectural intent: the observer system is for
types that participate in complex relationships (houses owning units,
triggers firing actions, anims attached to objects) where references
must be cleaned up coordinated. Simple game-world entities don't need
this — when they die, their owners just check `if (ptr == this) ptr = NULL`.

### Methodology success: vtable[+0x2c] inspection

The technique has now produced 10 RTTI mappings (3 from round 26 + 7 this
round) with zero failures. Each takes ~3 tool calls:
1. `list_globals` for `vtable__<Class>`
2. `read_memory` at vtable+0x2c (4 bytes → GetRTTI address)
3. `read_memory` at that address (8 bytes → decode b8 XX...)

Time per mapping: <30 seconds. Scales linearly. Easily 20+ more values
mappable in one focused iteration.

---

## 2026-05-17 — RTTI catalog expanded + observer pattern verified (round 26)

**Focus track:** `tracker-arrays-investigation` (continuing).

### 3 more RTTI values mapped via vtable inspection

Extracted by reading raw bytes at `vtable[+0x2c]`:

| RTTI    | Class           | Extraction                              |
|---------|-----------------|------------------------------------------|
| 0xd (13)| HouseClass      | `vtable__HouseClass @ 0x007ea8a0` → `0x0050e360`: `b8 0d 00 00 00 c3` |
| 0x26    | TriggerClass    | `vtable__TriggerClass @ 0x007f5858` → `0x00726940`: `b8 26 00 00 00 c3` |
| 0x2c    | TagClass        | `vtable__TagClass @ 0x007f44e0` → `0x006e58a0`: `b8 2c 00 00 00 c3` |

All three match the FUN_007258d0 dispatch table entries.

### Methodology checkpoint: vtable[+0x2c] inspection technique

Read 4 bytes at `vtable_addr + 0x2c` to get the GetRTTI function pointer.
The function body is always a 6-byte `b8 XX 00 00 00 c3` pattern
(MOV EAX, RTTI; RET). XX is the RTTI value as a single byte.

This is the FASTEST way to extract RTTI values:
1. Find the class's primary vtable via global symbol search
2. Read 4 bytes at `vtable + 0x2c`
3. Read 6-8 bytes at that address
4. Decode the RTTI from byte 1

No function decompilation needed once the technique is established.

### Observer pattern VERIFIED empirically (not just inferred)

xref scan of `DAT_00b0f6cc` (RTTI 0xd / HouseClass-removal dispatch):

| Function                  | Action |
|---------------------------|--------|
| SuperClass__Constructor   | PUSH self into the array |
| TechnoClass__Constructor  | PUSH self into the array |
| FUN_007258d0              | Iterates and notifies   |

SuperClass and TechnoClass instances register themselves to receive
notification when a HouseClass is removed. This is exactly the observer
pattern with concrete subscribers verified.

### 5th hypothesis revision of session

Round 22's plate comment on ObjectClass__Destructor hypothesized that
`DAT_00b0f61c` was `g_Logics_Array` (Westwood per-frame update queue).
Round 26 vtable evidence: RTTI 0x2c = TagClass. So `DAT_00b0f61c` is
the **TagClass removal-observer list**, NOT the Logics array.

The "Logics" hypothesis is RETRACTED. Update the ObjectClass destructor
plate comment in a future round.

Pattern: this is the 5th major hypothesis revision in the session.
Each one strengthened the methodology. Keep challenging hypotheses.

### Updated RTTI catalog (9 confirmed, 5 still UNKNOWN in dispatch)

Confirmed: Unit=1, HouseClass=0xd, Infantry=0xf, Overlay=0x14, OverlayType=0x15,
Terrain=0x24, TriggerClass=0x26, TagClass=0x2c, Tiberium=0x2e.

Still unknown in dispatch: 4, 0xc (bridge-related), 0x22, 0x33, 0x3c.
Plus 0x2f/0x30 which map to TriggerType (already labeled).

### Pending renames (verified-but-deferred)

The 3 observer arrays are role-VERIFIED but rename DEFERRED. Canonical
Westwood naming is uncertain:
- `g_<X>OwnedObjects` (semantic: things owned by/dependent on X)
- `g_<X>_RemoveObservers` (semantic: things that want notification)
- `g_<X>_DependantsList` (semantic: same)

Each pattern is plausible. Defer until canonical name surfaces, which is
likely to happen when more functions in the cleanup path get labeled.

---

## 2026-05-17 — RTTI catalog seeded + FUN_007258d0 plate comment (round 25)

**Focus track:** `tracker-arrays-investigation` (continuing).

### RTTI value catalog (6 confirmed via `What_Am_I` decompilation)

| RTTI    | Class                | Source                                |
|---------|----------------------|---------------------------------------|
| 1       | UnitClass            | `UnitClass__What_Am_I @ 0x00746e20`   |
| 0xf (15)| InfantryClass        | `InfantryClass__What_Am_I @ 0x00523340` |
| 0x14    | OverlayClass         | `OverlayClass__What_Am_I @ 0x005fdf50` |
| 0x15    | OverlayTypeClass     | `OverlayTypeClass__What_Am_I @ 0x005fef00` |
| 0x24    | TerrainClass         | `TerrainClass__What_Am_I @ 0x0071d300` |
| 0x2e    | TiberiumClass        | `TiberiumClass__GetRTTI @ 0x007236f0`  |

Each function is a trivial 1-line `return <constant>;` — easy to extract
values from once located.

### Key negative finding

NONE of these 6 RTTI values appear in the FUN_007258d0 dispatch table.
So removal of a Unit, Infantry, Overlay, Terrain, or Tiberium instance
does NOT trigger observer-list notification. Their cleanup is handled
purely by per-class array decrement (in their destructors).

The dispatch table covers RTTI 4, 0xc, 0xd, 0x22, 0x26, 0x2c, 0x2f, 0x30,
0x33, 0x3c — these map to OTHER classes (Building, House, Tag, Team,
Trigger types are the leading candidates based on context).

### Plate comment applied: `FUN_007258d0`

Comprehensive documentation of the function's role applied as plate
comment. Canonical Westwood name still DEFERRED — the role is HIGH
confidence (RTTI-keyed observer dispatch) but the exact name is MEDIUM
(`Detach_From_All_Lists` / `Notify_Observers_Of_Removal` /
`Dispatch_Removal_Notification` are all plausible).

The plate comment preserves the full analysis for future iterations
without committing to a speculative function name. When the canonical
name surfaces (likely via finding more callers or a debug-string
reference), one-step rename is straightforward.

### AbstractClass vtable slots inferred so far (3)

| Offset | Method                                  |
|--------|-----------------------------------------|
| `+0x28` | `NotifyOfRemoval(target, ?)` virtual    |
| `+0x2c` | `GetRTTIType()` virtual                 |
| `+0x64` | `Read_INI(CCINIClass*)` virtual         |

The `+0x64` slot was identified earlier in round 10 (RulesClass__ReadTypeData
calls vtable[+0x64] uniformly across all TypeClass arrays). The other two
emerged from the round-24 observer-pattern analysis.

### Strategy for unknown RTTI values

The 10+ unknown RTTI values in the dispatch table block deeper analysis
of the observer pattern. Cleanest path to mapping them:
1. Find each Class's vtable address (typically labeled `vtable__<X>`)
2. Read vtable[+0x2c] to get the GetRTTI function address (typically a
   1-line `return constant`)
3. Decompile that function

This is mechanical work that should yield 10+ new RTTI mappings in a
single iteration. Worth doing as a focused pass.

---

## 2026-05-17 — RTTI-keyed observer pattern discovered (round 24)

**Focus track:** `tracker-arrays-investigation` (promoted/renamed from
`instance-class-investigation`).

### Architectural breakthrough — no renames this iteration

`FUN_007258d0` is the **master "Detach From All Trackers"** dispatch
function. Its body:
1. Calls vtable[+0x2c] on target → gets RTTI type
2. Switches on RTTI type
3. For each branch: iterates a SPECIFIC tracker array and calls
   vtable[+0x28] on each entry (the listener callback)

**This means the "instance class arrays" I've been investigating are
actually OBSERVER LISTS**, not the actual instance arrays:

| Old interpretation                         | Corrected interpretation                  |
|--------------------------------------------|-------------------------------------------|
| g_HouseClass_Listeners or similar         | RTTI-keyed observer list — holds listeners that want notification when objects of that RTTI are removed |
| `DAT_00b0f724` = "secondary registry"      | "is_registered" observer dispatch         |
| `DAT_00b0f61c` = "Logics" array            | RTTI 0x2c observer list                   |

This was a major hypothesis revision: the Logics-array guess was wrong.
The structure is an Observer pattern, not a per-frame update queue.

### RTTI-keyed tracker arrays surfaced by the dispatch

| RTTI ID | Array            | Count            | Extra dispatch                    |
|---------|------------------|------------------|-----------------------------------|
| 4       | `DAT_00b0f5bc`   | `DAT_00b0f5c8`   | —                                 |
| 0xc     | `DAT_00b0f644`   | `DAT_00b0f650`   | UnregisterBridgeRepairHut         |
| 0xd     | `DAT_00b0f6cc`   | `DAT_00b0f6d8`   | FUN_0055b880                      |
| 0x22    | `DAT_00b0f5dc`   | `DAT_00b0f5e8`   | —                                 |
| 0x26    | `DAT_00b0f70c`   | `DAT_00b0f718`   | —                                 |
| 0x2c    | `DAT_00b0f61c`   | `DAT_00b0f628`   | UnregisterBridgeRepairHut         |
| 0x2f,30 | `g_TriggerTypeClass_Array` (labeled) | — | —                       |
| 0x33    | `DAT_00b0f5f4`   | `DAT_00b0f600`   | —                                 |
| 0x3c    | `DAT_00b0f6f4`   | `DAT_00b0f700`   | —                                 |

Plus unconditional dispatch through:
- `DAT_00b0f724/30` for "is_registered" objects (bit 1 of byte +5)
- `DAT_00b0f674/80` (master Abstract registry)

### Newly inferred vtable slots

- **+0x28 on AbstractClass vtable** = virtual `NotifyOfRemoval(target)`
  callback. Inferred from the dispatch pattern across all RTTI branches.
- **+0x2c on AbstractClass vtable** = virtual `GetRTTIType()`.

Worth labeling these vtable slots in a future round once their canonical
names are confirmed.

### Other learning: `BulletAnimTracker__Register` shows the push pattern

The function pushes a tracker into `DAT_00b0f69c` (count `DAT_00b0f6a8`)
when target attachment can't proceed normally. So this 5th array (the
one that appeared in ObjectClass destructor but not constructor) IS an
observer/tracker registry. The push happens from many places, not just
ObjectClass.

### Pending: canonical name for FUN_007258d0

Strong candidates: `Detach_From_All_Trackers`, `Notify_All_Of_Removal`,
`ObjectClass__Detach_From_All`. Rename DEFERRED until canonical Westwood
name is identified (likely surfaces when more callers are labeled).

### Hypothesis revisions this session: 4

1. Round 10 → corrected by round 12: position-based TypeClass array guesses
   were systematically wrong
2. Round 14 → corrected by round 15: "effect-only registry" was actually
   the universal Abstract registry
3. Round 22 → corrected by round 23: "multiple Constructor labels"
   doesn't imply destructor mislabel — usually just constructor variants
4. Round 21+22 (Logics hypothesis) → corrected by round 24: the "Logics"
   guess was wrong, the arrays are RTTI-keyed observers

Each correction strengthened the methodology. Pattern: high-information
investigations should challenge their own hypotheses before committing.

---

## 2026-05-17 — Destructor suspect-list audit (round 23, methodology refinement)

**Focus track:** `instance-class-investigation` (continuing).

### No mislabels found this iteration — but heuristic refined

Audited 6 candidate "destructor mislabel" addresses identified by the
round-22 suspect list. ALL 6 turned out to be legitimate constructor
variants. None decrement arrays. None call destructor markers. Outcome:

| Address      | Class                  | Actual role                |
|--------------|------------------------|----------------------------|
| `0x0075e0c0` | WarheadTypeClass       | Load-from-save constructor |
| `0x006ce800` | SuperWeaponTypeClass   | Load-from-save constructor |
| `0x00771f00` | WeaponTypeClass        | Default/minimal constructor |
| `0x00523980` | InfantryTypeClass      | Default/minimal constructor |
| `0x004f54a0` | HouseClass             | Full constructor (heavy)   |
| `0x00422720` | AnimClass              | Default constructor        |

### Refined methodology: destructor vs constructor-variant

**Heuristic**: "multiple `__Constructor` labels" is NOT sufficient evidence
of a destructor mislabel. Most TypeClasses have 1-3 legitimate constructor
variants. The destructor cases I caught earlier (AnimClass round 21,
ObjectClass round 22) had specific tell-tale bodies.

**Decision matrix** for any "extra Constructor" candidate:

| Signal                            | Destructor      | Constructor variant |
|-----------------------------------|-----------------|---------------------|
| Array operations                  | DECREMENT count | INCREMENT or none   |
| Resource handles                  | *__Release      | *__Init             |
| Object handle detachment          | *__Detach       | — (or *__Attach)    |
| Calls AssignUniqueID              | No              | Often yes           |
| `BombClass__Defuse` / similar     | Yes (object cleanup) | No             |
| `AbstractClass__Destructor_*`     | YES (smoking gun) | No                |
| Parent chain destination          | Destructor      | Constructor         |

A class with multiple Constructor variants typically has:
- 1 Full constructor (heavy state setup)
- 1 Load constructor (calls `AbstractClass__Load`, deserializes)
- 0-1 Default constructor (minimal init for stubs)
- 0-1 Destructor (only if cleanup logic is in there)

### New architectural finding: HouseClass push pattern

The HouseClass full constructor (0x004f54a0) pushes self into 5+
arrays:

- `g_HouseClass_Array` (pre-labeled, primary HouseClass instance array)
- `DAT_00b0f674` (master Abstract registry)
- `DAT_00b0f644` (count `DAT_00b0f650`) — UNKNOWN secondary
- `DAT_00b0f5f4` (count `DAT_00b0f600`) — UNKNOWN secondary
- `DAT_00b0f61c` (count `DAT_00b0f628`) — LIKELY g_Logics_Array
- `DAT_00b0f724` (count `DAT_00b0f730`) — same secondary as ObjectClass

Plus a House-to-House registration cycle that adds the new house to
each existing house's relation list (likely for alliance/enemy
tracking).

HouseClass also initializes SuperClass entries for every loaded
SuperWeaponType — a known startup-phase pattern from the engine.

### Cluster patterns now characterized

**3-variant clusters (full / default / destructor):**
- AnimClass: 0x00421ea0 / 0x00422720 / 0x004228e0 (destructor fixed R21)
- ObjectClass: 0x005f3900 / 0x005f3b50 / 0x005f3b80 (destructor fixed R22)

**2-variant clusters (full + load OR full + default):**
- HouseClass: 0x004f5190 (default) + 0x004f54a0 (full)
- WarheadTypeClass: 0x0075cec0 (full) + 0x0075e0c0 (load)
- SuperWeaponTypeClass: 0x006ce5b0 (full) + 0x006ce800 (load)
- WeaponTypeClass: 0x00771c70 (full) + 0x00771f00 (default)
- InfantryTypeClass: 0x005236a0 (full) + 0x00523980 (default)

### Investigation done — track refocus

The destructor-hunt has yielded its initial mislabel fixes. Further
hunting on the existing suspect list would have low yield. Closing this
sub-investigation; will pick up destructor mislabels OPPORTUNISTICALLY
when other work surfaces them (e.g., a function that looks like a
constructor but obviously does teardown).

---

## 2026-05-17 — ObjectClass destructor + g_ObjectClass_Array (round 22)

**Focus track:** `instance-class-investigation` (continuing).

### Mislabel fix (CONFIRMED via body inspection + smoking-gun marker)

- **`0x005f3b80`** `ObjectClass__Constructor` → `ObjectClass__Destructor`

  Pattern signals match destructor:
  - Leaf vtable set at top
  - DECREMENTS counts on 5 arrays (remove-self pattern)
  - Multiple `*__Detach` / `*__Release` calls
  - Calls `BombClass__Defuse` for attached-bomb cleanup

  **Smoking gun:** the function ends with a call to
  `AbstractClass__Destructor_ResetVtables` — a function whose label
  explicitly contains "Destructor". A constructor would NEVER call a
  function whose role is to reset vtables on destruction.

### Verified labels for ObjectClass instance array

- **`0x00a8e364`** → `g_ObjectClass_Array`
- **`0x00a8e370`** → `g_ObjectClass_Array_Count`

  Verified from BOTH sides:
  - REAL constructor (0x005f3900) pushes self into this array
  - DESTRUCTOR (0x005f3b80) removes self from it
  - Both reference the same address pair

### ObjectClass constructor cluster fully characterized

After this round, the three functions labeled `ObjectClass__Constructor`
are now resolved:

| Address      | Real role                                  |
|--------------|--------------------------------------------|
| `0x005f3900` | **Full constructor** (pushes to 4 arrays) |
| `0x005f3b50` | **VtablesOnly constructor** (load-from-save) |
| `0x005f3b80` | **Destructor** (FIXED this round) |

### Architectural finding: 5th array in destructor's cleanup

The destructor removes from FIVE arrays, but the full constructor only
pushes to FOUR. The 5th array (`DAT_00b0f69c` / count `DAT_00b0f6a8`) is
populated by some OTHER constructor in the chain — possibly the
AbstractClass-level part. Worth tracing in a future round.

This is the second case of "destructor cleans up more than constructor
visibly pushes" (first was AnimClass in round 21). Both cases reveal
arrays that are populated elsewhere in the inheritance chain. Useful
debugging technique: destructors tell you the full set of registries a
class participates in.

### Methodology: destructor pattern recognition now battle-tested

Pattern has caught 2 mislabels this session:
- Round 21: AnimClass destructor at 0x004228e0
- Round 22: ObjectClass destructor at 0x005f3b80

Both cases the body had:
- Leaf vtable set at top (looked like a constructor)
- Teardown body (DECREMENT, *Release, *Detach)
- Chained call to parent destructor (often itself mislabeled, OR named
  with explicit "Destructor" suffix as in round 22)

**Heuristic for future passes:**
- Any class with 2+ functions labeled `__Constructor` is suspicious.
- Decompile the candidates, look for the DECREMENT pattern on a known
  per-class array.
- If found: it's a destructor.

### Suspect list for future destructor hunting

Classes with multiple `__Constructor` variants that should be audited:
- TechnoTypeClass (3 variants: 0x00710af0, 0x00711840, 0x00711ae0)
- BuildingTypeClass (3 variants)
- InfantryTypeClass (2 variants: 0x005236a0, 0x00523980)
- WarheadTypeClass (2 variants: 0x0075cec0, 0x0075e0c0)
- SuperWeaponTypeClass (2 variants: 0x006ce5b0, 0x006ce800)
- WeaponTypeClass (2 variants: 0x00771c70, 0x00771f00)
- HouseClass (2 variants: 0x004f5190, 0x004f54a0)
- AnimClass (3 variants — round 21 fixed one; 0x00422720 still unchecked)

Plus all the Sidebar/Power/Display classes likely have destructors hiding.

---

## 2026-05-17 — AnimClass destructor mislabel + new methodology (round 21)

**Focus track:** `instance-class-investigation` (promoted from
`abstract-typeclass-array-globals`).

### Mislabel fix (CONFIRMED via body inspection)

- **`0x004228e0`** `AnimClass__Constructor` → `AnimClass__Destructor`

  The body is unambiguously a destructor:
  - Sets `vtable__AnimClass` at top (MSVC destructor convention: leaf
    vtable is restored before teardown so virtual calls during cleanup
    resolve to the correct class)
  - Decrements/removes self from `g_AnimClass_Array` via vtable[+0x10]
  - Conditionally decrements `DAT_00a83e04` array (when RTTI ID == -2)
  - Releases sound handles (SoundEvent__Release ×2, VocHandle__Detach ×2)
  - Chains to ObjectClass destructor at end

  The REAL AnimClass constructor is at `0x00421ea0` — it takes 7+this
  params, initializes ~0x1C8 bytes of state, sets vtable, calls
  `AbstractClass__AssignUniqueID`, and pushes self only into
  `g_AnimClass_Array` (NOT directly to the master Abstract registry —
  the master push happens via the parent `ObjectClass__Constructor`).

### New methodology: destructor pattern recognition

MSVC destructors in Ghidra often look like constructors at first glance:
- Both set the leaf vtable at top of body
- Both can have lots of state mutation
- Both end with a chained call to a parent function

**Distinguishing pattern:**

| Signal                          | Constructor   | Destructor          |
|---------------------------------|---------------|---------------------|
| Pushes to per-class array       | INCREMENT count | DECREMENT count   |
| State setup vs teardown         | Sets defaults | Clears/zeros state  |
| Sound/resource handles          | `*__Init`     | `*__Release/Detach` |
| Parent call at end              | Constructor   | Destructor (often   |
|                                 |               | mislabeled)         |
| Frame counter / unique-ID assign| Yes (init)    | No                  |

**Action:** any class with 2-3 functions labeled `__Constructor` is
suspicious — at least one of them is likely the destructor. Check via
body inspection before relying on the label.

### Follow-up: ObjectClass has 3 constructor variants

ObjectClass has functions at 0x005f3900, 0x005f3b50, 0x005f3b80 all
labeled `__Constructor`. Round-21 verification:
- `0x005f3900` is a REAL constructor (verified — pushes self into 4
  arrays: `DAT_00a8e364`, `DAT_00b0f724`, `DAT_00b0f674`, `DAT_00b0f61c`)
- The other two are UNVERIFIED — at least one is likely the
  ObjectClass destructor

Next iteration should verify.

### Architectural finding: ObjectClass pushes into 4 arrays

This is the most heavily-registered constructor seen so far. The 4
arrays:

1. **`DAT_00a8e364`** (count `DAT_00a8e370`) — LIKELY primary
   `g_ObjectClass_Array` (per-instance tracking)
2. **`DAT_00b0f724`** (count `DAT_00b0f730`) — UNKNOWN secondary
3. **`DAT_00b0f674`** (count `DAT_00b0f680`) — master Abstract registry
4. **`DAT_00b0f61c`** (count `DAT_00b0f628`) — LIKELY the "Logics"
   array (Westwood's per-frame update queue — objects that need
   `Per_Tick` calls)

Every ObjectClass-derived instance (BuildingClass, UnitClass,
AnimClass, etc.) pushes into ALL FOUR arrays via the inherited
constructor call. So the actual master "all live objects" picture in
the engine is more complex than a single registry — it's a 4-way
partition.

### Cumulative session: 4 mislabels fixed

1. AnimTypeClass: `__FindByName` → `__FindOrAllocate` (round 5)
2. Aircraft/Infantry array global swap (round 6, paired)
3. SidebarClass→SideClass constructor (round 11)
4. AnimClass destructor (round 21) — first DESTRUCTOR mislabel of
   the session

---

## 2026-05-17 — g_ArtINI label (round 20)

**Focus track:** `abstract-typeclass-array-globals` (continuing).

### New label (via `create_label` — was undefined data)

- **`0x00887180`** → `g_ArtINI`

  Identity VERIFIED via xref pattern:
  - `BuildingTypeClass__LoadVisualAssets` reads it 9 times (cameos, SHP
    files, building art parameters)
  - `ObjectTypeClass__ReadINI` reads it 6 times (per-object visual data)
  - `RulesClass__ReadTypeData` reads it during the AnimType 2nd-pass
    dispatch
  - `CCFileClass__Constructor`, `CDFileClass__Constructor` reference it

  This is the global CCINIClass instance for `artmd.ini` — the secondary
  INI file holding all per-type visual/animation data that doesn't fit
  in rules.ini.

### Method note: `create_label` vs `rename_data`

`rename_data` requires defined data (a memory region with an assigned
data type). For undefined-but-referenced addresses, use `create_label`.

This is the first address in the session that was undefined data with a
strong xref pattern. Worth keeping in mind for similar future findings —
the engine has many global INI pointers and state-machine globals that
exist at fixed addresses but were never tagged with a data type by
prior labelers.

### Verified-but-DEFERRED snapshot

The state file now maintains a "recovery list" of renames that are
proven but held back. Currently parked:
- All AI-related renames (per `feedback_no_ai_yet.md`)
- `DAT_00b0f674` master registry naming (name MEDIUM, evidence
  inconclusive on canonical Westwood term)
- 4 RulesClass sub-readers with identity HIGH but name MEDIUM

---

## 2026-05-17 — TriggerType array + AI exclusion finding (round 19)

**Focus track:** `abstract-typeclass-array-globals` (continuing).

### New labels

- **`0x00b0f65c`** → `g_TriggerTypeClass_Array`
- **`0x00b0f668`** → `g_TriggerTypeClass_Array_Count`

  Verified in round 18 via TriggerTypeClass__Constructor.

### Investigation: third Trigger-related registry at `DAT_008b417c`

xref scan revealed narrow scope:
- 2 writers: TriggerTypeClass__Constructor and one unlabeled writer at
  `0x004e6567` (outside any function)
- ~10 readers, mostly TriggerType-related code in 0x00727xxx

Conclusion: not a broad scenario-data registry as the round-18
hypothesis suggested. Likely a TriggerType-specific subset (e.g.
"currently-active triggers"). Investigation parked.

### AITriggerTypeClass — verified-but-rename-DEFERRED

`AITriggerTypeClass__Constructor @ 0x0041e350` sets
`vtable__AITriggerTypeClass` and pushes self into `DAT_00a8b204`
(count `DAT_00a8b210`).

**Key structural finding:** AITriggerType does NOT push into the master
registry `DAT_00b0f674`. AI types are EXCLUDED from the universal
Abstract tracking array. This is consistent with AI being a separate
subsystem, possibly because:
- AI logic runs at a different frame rate than core sim
- AI types don't need to participate in save/load via the master heap
- AI types are scenario-bound, not rules-bound, so they have a separate
  lifetime model

Rename **DEFERRED per `feedback_no_ai_yet.md`**. The verified mapping
is documented in the state file for one-step recovery when AI work
resumes:
- `DAT_00a8b204` → `g_AITriggerTypeClass_Array`
- `DAT_00a8b210` → `g_AITriggerTypeClass_Array_Count`

### Updated cumulative TypeClass array catalog (23 labeled this session)

Adding TriggerType to the list:

| Class                       | Array global                      |
|-----------------------------|-----------------------------------|
| TriggerTypeClass (NEW)      | g_TriggerTypeClass_Array          |

Plus TagType, TeamType (round 18); House, Terrain, SuperWeapon, Warhead,
Weapon, Bullet, VoxelAnim, TechnoType, Tiberium, IsometricTileType (rounds
10–17); plus prior swap-fixes and pre-existing labels.

---

## 2026-05-17 — TagType + TeamType arrays (round 18)

**Focus track:** `abstract-typeclass-array-globals` (continuing).

### New labels (vtable-as-ground-truth VERIFIED)

- **`0x00b0e784`** → `g_TagTypeClass_Array`
- **`0x00b0e790`** → `g_TagTypeClass_Array_Count`

  Verified via `TagTypeClass__Constructor @ 0x006e5b60`: sets
  `*param_1 = &vtable__TagTypeClass` and pushes self into `DAT_00b0e784`.

- **`0x00a8eca4`** → `g_TeamTypeClass_Array`
- **`0x00a8ecb0`** → `g_TeamTypeClass_Array_Count`

  Verified via `TeamTypeClass__Constructor @ 0x006f06e0`: sets
  `*param_1 = &vtable__TeamTypeClass` and pushes self into `DAT_00a8eca4`.

### Verified-but-deferred (next iteration)

- `DAT_00b0f65c` is `g_TriggerTypeClass_Array` — proven by
  TriggerTypeClass__Constructor @ 0x00726c80 setting
  `vtable__TriggerTypeClass`. Held back to bundle with its structural
  finding (see below).

### Structural discovery: third shared registry

**TriggerTypeClass pushes into THREE arrays**, not two:

1. Primary: `DAT_00b0f65c` (count `DAT_00b0f668`)
2. Master registry: `DAT_00b0f674` (the universal Abstract registry)
3. **NEW: `DAT_008b417c` (count `DAT_008b4188`)** — a third shared registry

The third registry is adjacent in memory to `DAT_008b4124` (the SideClass
array identified in round 11). Both share the prefix `0x008b41...`. This
suggests `0x008b41xx` is a region of "scenario/campaign related" globals.

Worth a separate investigation pass to identify what else pushes into
`DAT_008b417c`. Candidates to check: TActionClass, TEventClass,
TriggerClass, ScenarioClass-related globals.

### Session catalog: 22 TypeClass arrays now labeled

Continuing from round 17:

| Class                       | Array global                    |
|-----------------------------|---------------------------------|
| TagTypeClass (NEW)          | g_TagTypeClass_Array            |
| TeamTypeClass (NEW)         | g_TeamTypeClass_Array           |
| TriggerTypeClass (verified, defer) | g_TriggerTypeClass_Array (next round) |

Plus the 20 from prior rounds.

---

## 2026-05-17 — Tiberium + IsometricTileType arrays (round 17)

**Focus track:** `abstract-typeclass-array-globals` (new active track).

### New labels (vtable-as-ground-truth VERIFIED)

- **`0x00b0f4ec`** → `g_TiberiumClass_Array`
- **`0x00b0f4f8`** → `g_TiberiumClass_Array_Count`

  Verified via `TiberiumClass__Constructor @ 0x007216c0`: sets
  `*param_1 = &vtable__TiberiumClass` and pushes self into `DAT_00b0f4ec`
  with count `DAT_00b0f4f8`. Also pushes into the master registry
  `DAT_00b0f674` (consistent with round-15 finding).

  Note: TiberiumClass is unusual in Westwood code — it's named "Class"
  but functions as a TypeClass (one instance per ore color/type, no
  separate `TiberiumTypeClass`). The `*Class` naming is historical.

- **`0x00a8ed2c`** → `g_IsometricTileTypeClass_Array`
- **`0x00a8ed38`** → `g_IsometricTileTypeClass_Array_Count`

  Verified via `IsometricTileTypeClass__Constructor @ 0x005447c0`: sets
  `*param_1 = &vtable__IsometricTileTypeClass`. Has a conditional
  per-class push (gated by `param_6`) and an unconditional push to
  `DAT_00b0f674`.

  IsometricTileTypeClass is the tile-graphic descriptor used by the map
  renderer — each is one terrain tile definition (Grass1, Shore1A, etc.).

### Session progress: 20 TypeClass arrays labeled

The comprehensive set of TypeClass arrays now properly labeled:

| Class                       | Array global                              |
|-----------------------------|-------------------------------------------|
| HouseTypeClass              | g_HouseTypeClass_Array                    |
| TerrainTypeClass            | g_TerrainTypeClass_Array                  |
| SuperWeaponTypeClass        | g_SuperWeaponTypeClass_Array              |
| WarheadTypeClass            | g_WarheadTypeClass_Array                  |
| WeaponTypeClass             | g_WeaponTypeClass_Array                   |
| BulletTypeClass             | g_BulletTypeClass_Array                   |
| VoxelAnimTypeClass          | g_VoxelAnimTypeClass_Array                |
| TechnoTypeClass             | g_TechnoTypeClass_Array (aggregate)       |
| TiberiumClass               | g_TiberiumClass_Array (NEW)               |
| IsometricTileTypeClass      | g_IsometricTileTypeClass_Array (NEW)      |
| SmudgeTypeClass             | g_SmudgeTypeClass_Array                   |
| BuildingTypeClass           | g_BuildingTypeClass_Array (was pre-existing)|
| UnitTypeClass               | g_UnitTypeClass_Array (was pre-existing)  |
| AircraftTypeClass           | g_AircraftTypeClass_Array (swap fixed)    |
| InfantryTypeClass           | g_InfantryTypeClass_Array (swap fixed)    |
| AnimTypeClass               | g_AnimTypes_Array (was pre-existing)      |
| ParticleTypeClass           | g_ParticleTypeClass_Array (pre-existing)  |
| ParticleSystemTypeClass     | g_ParticleSystemTypeClass_Array (pre-existing) |
| OverlayTypeClass            | g_OverlayTypeClass_Array (pre-existing)   |
| SideClass                   | (DAT_008b4124 identified, not yet labeled)|

Remaining TypeClass-style targets (mission/scripting subsystem): TagType,
TeamType, TriggerType, plus instance-class arrays (Object, Anim, Team,
TActionClass, TEventClass, TriggerClass).

---

## 2026-05-17 — Sidebar mislabel suspicions cleared + AbstractClass investigation closed (round 16)

**Focus track:** `abstractclass-master-registry-investigation` — closing.

### Mislabel investigations CLOSED (no renames needed)

- **`0x006a4e60` `SidebarClass__Constructor`** — verified CORRECT. Body
  sets `*param_1 = &vtable_SidebarClass` (note: single underscore, the
  non-TypeClass vtable naming convention). Calls PowerClass__constructor
  first (deep inheritance chain: AbstractClass → DisplayClass →
  PowerClass → SidebarClass). Initializes 4 cameo strip arrays via
  `FUN_004068f0(..., StripClass__CameoEntry__SortCompare)`. Identity
  verified — this is the real Sidebar UI constructor.

- **`0x006a4f20` `SidebarClass__constructor` (lowercase)`** — also
  CORRECT. A variant of the constructor (probably load/copy
  constructor). Body sets `vtable_SidebarClass` at the end. Calls
  `DisplayClass__Constructor` followed by SidebarClass-specific init.

Both labels were correct from a prior labeling pass. The only Sidebar
cluster mislabel was `0x006a4550` → `SideClass__Constructor` (fixed in
round 11). Track item closed.

### AbstractClass constructor variants discovered

- **`AbstractClass__Constructor_Full @ 0x00410170`** — body only sets
  vtables. Does NOT push to `DAT_00b0f674`. So registry-push is the
  responsibility of each derived class's own constructor body, not
  inherited from AbstractClass.

- **`AbstractClass__Constructor_VtablesOnly @ 0x004101c0`** — already
  labeled. Used by `HouseClass__Constructor` to deliberately bypass the
  full constructor. This is the mechanism by which some classes opt out
  of the master registry.

### Final characterization of `DAT_00b0f674`

The registry is the **master Westwood AbstractClass tracking array** —
holds all Abstract-derived TypeClass objects (and many but not all
instance objects). Push behavior is per-class:

- TypeClasses with direct push: Anim, VoxelAnim, Particle, Bullet,
  Warhead, IsometricTile, SuperWeapon, TagType, TeamType, TriggerType,
  Tiberium
- TypeClasses with inherited push (via TechnoTypeClass): Building, Unit,
  Aircraft, Infantry
- Instance classes that push directly: Object, AnimClass, ParticleSystem,
  Super, Tag, Team, TAction, TEvent, Trigger, Neuron
- Instance classes that DON'T push (use VtablesOnly): HouseClass

**Identity HIGH; canonical Westwood name MEDIUM.** Candidate names:
`g_AbstractClass_Heap`, `g_All_Abstract_Objects`, `g_Logics_All`. Rename
DEFERRED — too speculative without a function name that explicitly
references the registry's purpose.

### Methodology checkpoints

- Vtable-as-ground-truth: 12+ uses, 0 false positives.
- Constructor inheritance trace: 1 use, caught a false-negative.
- Mislabel investigations: 5 (3 confirmed mislabels fixed; 2 confirmed
  CORRECTLY labeled and cleared).

### Track closure

`abstractclass-master-registry-investigation` is PARKED with a complete
technical writeup. The remaining uncertainty (canonical name) requires
evidence from a function that explicitly works with the registry by name
— that evidence will surface incidentally when other Westwood subsystems
are labeled (likely the save/load machinery or the Map class). Future
iterations should not actively chase this; pick it up only if the right
function surfaces.

---

## 2026-05-17 — TechnoType aggregate + round-14 hypothesis correction (round 15)

**Focus track:** `abstractclass-master-registry-investigation` (renamed
from `shared-effect-typeclass-registry`).

### New labels

- **`0x00a8eb04`** → `g_TechnoTypeClass_Array`
- **`0x00a8eb10`** → `g_TechnoTypeClass_Array_Count`

  Verified via TechnoTypeClass__Constructor @ 0x00710af0. The function
  sets `*param_1 = &vtable__TechnoTypeClass` and pushes self into BOTH
  `g_TechnoTypeClass_Array` AND `DAT_00b0f674` (the master Abstract
  registry).

  This is the TechnoTypeClass aggregate — holds all Building/Unit/
  Aircraft/Infantry types combined, viewed as their common Techno parent.
  Useful for global techno-type lookup-by-name.

### MAJOR hypothesis CORRECTION

**Round 14 claim:** `DAT_00b0f674` is a "combat-effect TypeClass registry"
because Anim/VoxelAnim/Particle/Bullet/Warhead push there directly,
while BuildingType does NOT push there in its constructor body.

**Round 15 reality (corrected via xref + parent-constructor inspection):**

Building DOES push to `DAT_00b0f674` — INDIRECTLY through its parent
`TechnoTypeClass__Constructor`. The round-14 analysis only looked at the
direct body of `BuildingTypeClass__constructor` and missed the inherited
push.

xref scan of `0x00b0f674` shows ~20 constructors reference it, spanning:
- TypeClasses: TechnoType (parent of Building/Unit/Aircraft/Infantry),
  Bullet, Warhead, Anim, VoxelAnim, Particle, Tiberium, IsometricTile,
  TagType, TeamType, TriggerType, SuperWeapon
- Game-instance classes: Object, House, Anim, Neuron, ParticleSystem,
  Super, Tag, Team, TAction, TEvent, Trigger

**Conclusion:** This is the master Westwood **Abstract-class tracking
array** — every Abstract-derived object/type pushes here. Used for
save/load and possibly global iteration.

### Lesson logged (METHODOLOGY)

**Constructor-body inspection must account for INHERITED registrations via
parent constructor calls.**

When a TypeClass body doesn't appear to push into a registry, this is
NOT proof of non-membership. The parent constructor (called at the top
of the function) may perform the push. ALWAYS:

1. Identify the parent constructor call (e.g., `TechnoTypeClass__Constructor`).
2. Decompile the parent constructor.
3. Check whether the parent pushes into the registry.

This caught a false negative in round 14. Adding to the methodology
checklist alongside vtable-as-ground-truth.

### Methodology applications (running total)

- Vtable-as-ground-truth: 12+ uses, 0 false positives
- Constructor-inheritance trace: 1 use this round, 1 false-negative caught

### Pending registry naming

`DAT_00b0f674` is HIGHLY likely the canonical Westwood AbstractClass
registry. Candidate names: `g_AbstractClass_All`, `g_Logics_All`,
`g_All_AbstractClass_Array`.

Rename DEFERRED until:
- A function name reveals the canonical role (e.g.,
  `AbstractClass__Get_All` or `Map__Logics_Array`).
- Confidence reaches HIGH on the canonical name (currently MEDIUM-HIGH on
  role, MEDIUM on name).

---

## 2026-05-17 — VoxelAnimType array + shared-registry characterization (round 14)

**Focus track:** `shared-effect-typeclass-registry` (promoted from
`typeclass-globals-audit-via-vtable`).

### New labels

- **`0x00a8eb2c`** → `g_VoxelAnimTypeClass_Array`
- **`0x00a8eb38`** → `g_VoxelAnimTypeClass_Array_Count`

  Verified previously (round 13) via VoxelAnimTypeClass__Constructor.
  Rename completed this round.

### Major characterization: shared effect-TypeClass registry at `DAT_00b0f674`

Five constructors decompiled this round to map the registry membership:

| Constructor                              | Pushes into `DAT_00b0f674`? |
|------------------------------------------|------------------------------|
| AnimTypeClass @ 0x00427530               | YES                          |
| VoxelAnimTypeClass @ 0x0074ad80          | YES (round 13)               |
| ParticleTypeClass @ 0x00644be0           | YES                          |
| BulletTypeClass @ 0x0046bbc0             | YES (round 13)               |
| WarheadTypeClass @ 0x0075cec0            | YES (round 12)               |
| BuildingTypeClass @ 0x0045dd90           | **NO**                       |

**Conclusion:** `DAT_00b0f674` is NOT a global TypeClass registry. It is
specifically a **combat-effect / projectile / animation TypeClass
registry**. All five members are types that can be referenced from
rules.ini combat fields:

- Warhead = damage profile
- Bullet = projectile fired
- AnimType = explosion/effect animation
- VoxelAnimType = voxel-based debris/explosion
- ParticleType = smoke/sparks/particles

**Identity HIGH; canonical Westwood name MEDIUM-HIGH.** Rename deferred —
candidates are `g_EffectTypeClass_Registry`, `g_AnimEffectClass_Array`,
`g_CombatEffectType_Array`. Path forward: trace `get_xrefs_to` 0x00b0f674
to find reader functions; the readers should reveal the canonical name via
function names or debug strings.

### DynamicVectorClass layout at the registry (canonical 6-field)

- `0x00b0f670`: vtable pointer (DV class itself)
- `0x00b0f674`: array pointer (T**)
- `0x00b0f678`: capacity
- `0x00b0f67d`: byte flag (heap-owned / auto-resize)
- `0x00b0f680`: count
- `0x00b0f684`: growth increment

Identical layout to the per-class array globals — confirming the
hypothesis from round 6 that all TypeClass arrays in gamemd.exe use the
same DynamicVectorClass struct.

### Session cumulative state

- Functions renamed: ~21 (including 3 mislabel fixes)
- Globals renamed: 20 (paired with their counts where applicable)
- TypeClass arrays now labeled: 17 (House, Terrain, SuperWeapon, Warhead,
  Weapon, Bullet, VoxelAnim newly added; rest pre-existing)
- Constructors decompiled but unmodified: ~15
- Methodology applications: vtable-as-ground-truth used 11+ times, all
  successful

---

## 2026-05-17 — Weapon + Bullet arrays + shared-registry discovery (round 13)

**Focus track:** `typeclass-globals-audit-via-vtable` — continuing.

### New labels (vtable-as-ground-truth VERIFIED)

- **`0x0088756c`** → `g_WeaponTypeClass_Array`
- **`0x00887578`** → `g_WeaponTypeClass_Array_Count`

  Verified via `WeaponTypeClass__Constructor @ 0x00771c70`:
  sets `*param_1 = &vtable__WeaponTypeClass` and pushes self into
  `DAT_0088756c`. (Verification done in round 12; rename completed now.)

- **`0x00a83c84`** → `g_BulletTypeClass_Array`
- **`0x00a83c90`** → `g_BulletTypeClass_Array_Count`

  Verified via `BulletTypeClass__Constructor @ 0x0046bbc0`:
  sets `*param_1 = &vtable__BulletTypeClass` and pushes self into
  `DAT_00a83c84`. ALSO pushes into the secondary registry at
  `DAT_00b0f674` (see structural discovery below).

  This resolves the "UNCLEAR" entry from round 11 — `DAT_00a83c84` is the
  BulletTypeClass array, not (as round 10 had guessed) SuperWeapon.

### Verified-but-deferred (next iteration)

- `0x00a8eb2c` is `g_VoxelAnimTypeClass_Array` — proven by
  VoxelAnimTypeClass__Constructor @ 0x0074ad80 setting
  `vtable__VoxelAnimTypeClass` and pushing into it. Will rename next round.

### Major structural discovery: SHARED registry at `DAT_00b0f674`

Decompiling Warhead, Bullet, and VoxelAnimType constructors revealed that
**at least 3 TypeClasses push self into a shared secondary registry at
`DAT_00b0f674` (count `DAT_00b0f680`)** in addition to their primary array.

The push always happens after the primary push, gated by the same kind of
capacity/flag/growth check as the primary array. The shared registry has
the same 6-field DynamicVectorClass layout (`DAT_00b0f670` vtable,
`DAT_00b0f674` array, `DAT_00b0f678` capacity, `DAT_00b0f67d` flag byte,
`DAT_00b0f680` count, `DAT_00b0f684` growth).

Common thread between Warhead, Bullet, VoxelAnim: all three are referenced
from combat-dispatch and impact-effect code paths. **Hypothesis:**
`DAT_00b0f674` is a unified "effect-type lookup" table used by combat
dispatch for indirect type-by-name lookup.

**Open question for next iteration:** Do AnimType and ParticleType also
push here? If yes, this is the global "ObjectTypeClass" registry (all
ObjectTypeClass-derived types). If only Bullet/Warhead/VoxelAnim push,
it's specifically the combat-effect dispatch table.

Worth labeling `DAT_00b0f674` once the membership is fully characterized.

### Methodology checkpoint: 9+ vtable-as-ground-truth applications

Zero false positives. This continues to be the gold standard for any
TypeClass identification question.

### Cumulative TypeClass array catalog (post round 13)

| Class                       | Array global              | Status this session |
|-----------------------------|---------------------------|---------------------|
| HouseType                   | `g_HouseTypeClass_Array`  | NEW (round 10)      |
| TerrainType                 | `g_TerrainTypeClass_Array`| NEW (round 11)      |
| SuperWeaponType             | `g_SuperWeaponTypeClass_Array` | NEW (round 12) |
| WarheadType                 | `g_WarheadTypeClass_Array`| NEW (round 12)      |
| WeaponType                  | `g_WeaponTypeClass_Array` | NEW (round 13)      |
| BulletType                  | `g_BulletTypeClass_Array` | NEW (round 13)      |
| AnimType                    | `g_AnimTypes_Array`       | pre-existing        |
| VoxelAnimType               | `g_VoxelAnimTypeClass_Array` (next round) | LIKELY  |
| BuildingType                | `g_BuildingTypeClass_Array`| pre-existing       |
| UnitType                    | `g_UnitTypeClass_Array`   | pre-existing        |
| AircraftType                | `g_AircraftTypeClass_Array` | swap-fixed (R6)   |
| InfantryType                | `g_InfantryTypeClass_Array` | swap-fixed (R6)   |
| SmudgeType                  | `g_SmudgeTypeClass_Array` | labeled (R8)        |
| ParticleType                | `g_ParticleTypeClass_Array` | pre-existing      |
| ParticleSystemType          | `g_ParticleSystemTypeClass_Array` | pre-existing|
| OverlayType                 | `g_OverlayTypeClass_Array` | pre-existing       |
| SideClass                   | `DAT_008b4124` (unlabeled, identified) | LIKELY |

---

## 2026-05-17 — TypeClass globals: SuperWeapon + Warhead arrays (round 12)

**Focus track:** `typeclass-globals-audit-via-vtable` — continuing.

### New labels (vtable-as-ground-truth VERIFIED)

- **`0x00a8e334`** → `g_SuperWeaponTypeClass_Array`
- **`0x00a8e340`** → `g_SuperWeaponTypeClass_Array_Count`

  Verified via `SuperWeaponTypeClass__Constructor @ 0x006ce5b0`:
  sets `*param_1 = &vtable__SuperWeaponTypeClass` and pushes self into
  `DAT_00a8e334`.

- **`0x008874c4`** → `g_WarheadTypeClass_Array`
- **`0x008874d0`** → `g_WarheadTypeClass_Array_Count`

  Verified via `WarheadTypeClass__Constructor @ 0x0075cec0`:
  sets `*param_1 = &vtable__WarheadTypeClass` and pushes self into
  `DAT_008874c4`. ALSO pushes into a secondary registry at `DAT_00b0f674`
  — Warhead has DUAL registration (parked for separate audit).

### Verified-but-deferred (renames held back due to budget)

- `0x0088756c` is the `g_WeaponTypeClass_Array` — proven by
  `WeaponTypeClass__Constructor @ 0x00771c70` setting
  `vtable__WeaponTypeClass` and pushing into it. Will rename next iteration.

### LIKELY corrections to round-10 guesses (LESSON)

Round 10 used dispatch-position in `RulesClass__ReadTypeData` to LIKELY-identify
4 TypeClass arrays. The vtable verification this round shows 3 of those
guesses were WRONG:

| Round-10 LIKELY                       | Vtable proof says                |
|---------------------------------------|----------------------------------|
| `DAT_00a8e334` = TerrainType          | SuperWeaponType                  |
| `DAT_0088756c` = WarheadType          | WeaponType                       |
| `DAT_008874c4` = WeaponType           | WarheadType                      |
| `DAT_00a83c84` = SuperWeaponType      | UNCLEAR — not yet verified       |

The Warhead/Weapon arrays were SWAPPED in my position-based guesses — same
exact failure mode as the Aircraft/Infantry array swap mislabel discovered
in round 5. Lesson: position-based interpretations of TypeClass dispatch
order are systematically unreliable. The actual order in
`RulesClass__ReadTypeData` does NOT correspond to a clean
"declaration-order" or "alphabetical" arrangement.

**Policy locked in:** never rename a TypeClass global based on dispatch
position alone. ALWAYS verify with the corresponding `<TC>__Constructor`'s
vtable assignment first. The constructor decompilation is fast (one MCP
call), and the failure rate of position-based guesses is too high to risk
compounding mislabels into downstream rounds.

### Methodology checkpoint: vtable-as-ground-truth applications to date

- Round 5: AnimType FindByName → FindOrAllocate (mislabel fix, partial)
- Round 6: Aircraft/Infantry global swap (2 globals corrected)
- Round 7: SmudgeTypeClass FindOrAllocate verification (new label)
- Round 11: SidebarClass → SideClass constructor mislabel fix
- Round 11: TerrainTypeClass array (new label)
- Round 12: SuperWeapon/Warhead/Weapon arrays (3 new + 2 LIKELY corrections)

Zero false positives across 6 applications. This is now the
**single most important verification method** in the labeling pass.

### Discovery: WarheadTypeClass has DUAL registration

WarheadTypeClass__Constructor pushes self into TWO arrays:
- `g_WarheadTypeClass_Array` (primary, name-keyed for INI lookup)
- `DAT_00b0f674` (secondary registry, count `DAT_00b0f680`)

The secondary push happens after the primary one, conditionally. Suspected
purpose: a separate warhead lookup table — possibly for combat-damage
dispatch (the warhead-vs-armor matrix). Investigation deferred to a future
combat-system audit track. Worth labeling `DAT_00b0f674` as
`g_WarheadTypeClass_Combat_Registry` or similar once its purpose is
verified.

---

## 2026-05-17 — SideClass mislabel + TerrainType array (round 11)

**Focus track promotion:** `typeclass-globals-audit-via-vtable` promoted
from the broader inline-dispatchers track.

### Mislabel fix (CONFIRMED via vtable-as-ground-truth)

- **`0x006a4550`** `SidebarClass__Constructor` → `SideClass__Constructor`

  Verification:
  - Body sets `*param_1 = &vtable__SideClass` (NOT `vtable__SidebarClass`)
  - Pushes self into `DAT_008b4124` (the SideClass array, count `DAT_008b4130`)
  - Allocates 0xb4 bytes (matches SideClass; far too small for SidebarClass
    which has full UI/HUD state)
  - Called from FUN_00672440 (`[Sides]` INI reader)

  Note: there are 2 OTHER functions still named `SidebarClass__Constructor`
  at `0x006a4e60` and `0x006a4f20` (the latter with lowercase 'constructor').
  Those are NOT addressed this iteration — they may be real SidebarClass
  constructors or additional mislabels. Pending verification.

### New labels (vtable-as-ground-truth)

- **`0x00a8e31c`** → `g_TerrainTypeClass_Array`
- **`0x00a8e328`** → `g_TerrainTypeClass_Array_Count`

  Verified via `TerrainTypeClass__Constructor @ 0x0071da80`:
  - Sets `*param_1 = &vtable__TerrainTypeClass`
  - Pushes self into `DAT_00a8e31c` with count `DAT_00a8e328`
  - Same canonical 6-field DynamicVectorClass layout
    (`DAT_00a8e318` vtable, `DAT_00a8e320` capacity, `DAT_00a8e325` flag,
    `DAT_00a8e328` count, `DAT_00a8e32c` growth).

### Hypothesis CORRECTION

In round 10 I LIKELY-identified `DAT_00a8e334` as the TerrainTypeClass array
based on dispatch position in `RulesClass__ReadTypeData`. **This was wrong.**
The actual terrain array is `DAT_00a8e31c`. The identity of `DAT_00a8e334`
is now UNKNOWN — re-park as a pending candidate.

Lesson: dispatch-order LIKELY identifications are heuristic only; always
verify with vtable-as-ground-truth before committing. The session has now
seen TWO position-based misguesses (AnimType FindByName→FindOrAllocate
implication, and now this Terrain array hypothesis) — confidence in
position-based naming is MEDIUM at best.

### Methodology checkpoint: vtable-as-ground-truth now battle-tested

The "vtable assignment in constructor body" method has resolved three
class-identity questions this session with zero false positives:

1. Aircraft/Infantry array swap (round 6)
2. AnimType FindByName→FindOrAllocate confirmation (round 5 — partial)
3. SidebarClass → SideClass constructor (round 11)

This is now the **gold standard** for any class-identity question in
gamemd.exe. Use it FIRST whenever a label is in doubt. Faster, more
reliable, and zero risk of compounding earlier mislabels.

---

## 2026-05-17 — HouseTypeClass factory + RulesClass__ReadTypeData (round 10)

**Focus track:** `rules-class-process-inline-dispatchers` — major progress.

### New labels

- **`0x00512680`** `FUN_00512680` → `HouseTypeClass__FindOrAllocate`
  - Canonical pattern (two-sentinel + search + `operator_new(0x1b0)` +
    `HouseTypeClass__Constructor`).
  - Called from `RulesClass__Process` at the `[Countries]` section dispatch.
  - Westwood naming: INI side says "Countries", code side says "HouseType".
  - sizeof(HouseTypeClass) = 0x1b0.

- **`0x00a83c9c`** → `g_HouseTypeClass_Array`
- **`0x00a83ca8`** → `g_HouseTypeClass_Array_Count`

- **`0x00679a10`** `FUN_00679a10` → `RulesClass__ReadTypeData`
  - The TypeClass second-pass INI reader. After all TypeClass objects are
    registered (in section-list reads — round 1), this iterates every
    TypeClass array and calls each instance's vtable[+0x64] (the virtual
    `Read_INI` method) to populate per-type gameplay data.
  - Dispatches across 18+ TypeClass arrays, then ends with
    `MissionClass__Read_INI` for the 0x100-slot mission table.
  - Identity HIGH (master 2nd-pass invoker). Name MEDIUM-HIGH — the
    function is unambiguously this role but the exact Westwood method name
    could also be `ReadTypeINI` or `Process_All_Types`. Going with
    `RulesClass__ReadTypeData` per Westwood convention.

### Key structural discovery

**The +0x64 vtable slot on every TypeClass is the virtual `Read_INI(CCINIClass*)`
method.** This was discovered via the body of `RulesClass__ReadTypeData`,
which calls `(**(code **)(*type_ptr + 0x64))(ini)` uniformly across all
type arrays. Useful for:
- Future TypeClass vtable-layout pass
- Cross-referencing every TypeClass's INI key set (just decompile each
  Read_INI override)
- Identifying mislabeled TypeClass arrays (find what's at +0x64 of the
  vtable and trace which class it belongs to)

### Six unlabeled TypeClass arrays surfaced

`RulesClass__ReadTypeData` iterates these arrays — their identities are
HIGH-confidence LIKELY based on dispatch position vs already-labeled
neighbors:

| Array global       | Count global       | LIKELY class                     |
|--------------------|--------------------|----------------------------------|
| `DAT_00a8e334`     | `DAT_00a8e340`     | `g_TerrainTypeClass_Array`       |
| `DAT_0088756c`     | `DAT_00887578`     | `g_WarheadTypeClass_Array`       |
| `DAT_00a83c84`     | `DAT_00a83c90`     | `g_SuperWeaponTypeClass_Array`   |
| `DAT_008874c4`     | `DAT_008874d0`     | `g_WeaponTypeClass_Array`        |
| `DAT_00a8e31c`     | `DAT_00a8e328`     | UNCLEAR — TiberiumClass?         |
| `DAT_00a8eb2c`     | `DAT_00a8eb38`     | `g_VoxelAnimTypeClass_Array` (confirmed earlier) |

Verification path for next iteration: decompile each TypeClass constructor
and confirm which global it pushes into (vtable-as-ground-truth method).

### Mislabel suspicion (PENDING verification)

`SidebarClass__Constructor` — called from FUN_00672440 (`[Sides]` reader)
after `operator_new(0xb4)`. But sizeof(SidebarClass) should be much larger
(it includes UI/HUD state). 0xb4 = 180 bytes is more consistent with a
`SideClass` struct.

This is the third potential TypeClass naming-confusion mislabel in the
session (after `AnimTypeClass__FindByName` and the Aircraft/Infantry global
swap). A broader audit of "similarly-named class" mislabels is warranted.

---

## 2026-05-17 — ElevationModel + WallModel sub-readers (round 9)

**Focus track:** `rules-class-process-inline-dispatchers` — partial progress.

### New labels (HIGH confidence, both name and identity)

- **`0x0066d150`** `FUN_0066d150` → `RulesClass__ReadElevationModel`

  Reads 3 keys from the `[ElevationModel]` INI section into RulesClass
  members: ElevationIncrement (int) → this+0x1838, ElevationIncrementBonus
  (double) → this+0x1840, ElevationBonusCap (double) → this+0x1848.
  Identity VERIFIED via INI section pointer name + exact key set match.

- **`0x0066d1f0`** `FUN_0066d1f0` → `RulesClass__ReadWallModel`

  Reads 2 keys from the `[WallModel]` INI section: AlliedWallTransparency
  (bool) → this+0x1850, WallPenetratorThreshold (double) → this+0x1858.
  Identity VERIFIED similarly.

### Identification with rename DEFERRED (per user feedback memory)

- `0x00672ae0` — **LIKELY** `RulesClass__ReadAI`. Massive INI reader covering
  the entire `[AI]` section: BuildConst, BuildPower, BuildRefinery,
  BuildBarracks, BuildTech, BuildWeapons, Allied/Soviet/Third-BaseDefenses,
  AIForcePredictionFudge, BuildDefense, BuildPDefense, BuildAA,
  BuildHelipad, BuildRadar, ConcreteWalls, NSGates, EWGates, BuildNavalYard,
  BuildDummy, NeutralTechBuildings, AttackInterval, AttackDelay, PatrolScan,
  CreditReserve, PathDelay, BlockagePathDelay, AutocreateTime,
  InfantryReserve, InfantryBaseMult, PowerSurplus, BaseSizeAdd,
  RefineryRatio/Limit, BarracksRatio/Limit, WarRatio/Limit,
  DefenseRatio/Limit, AARatio/Limit, TeslaRatio/Limit, HelipadRatio/Limit,
  AirstripRatio/Limit, CompEasyBonus, Paranoid, PowerEmergency,
  AIBaseSpacing, GDIWallDefense, GDIWallDefenseCoefficient,
  NodBaseDefenseCoefficient, GDIBaseDefenseCoefficient,
  MaximumBaseDefenseValue, ComputerBaseDefenseResponse — 50+ keys.

  Identity HIGH confidence; rename DEFERRED per `feedback_no_ai_yet.md` —
  the user has asked to skip AI work at the current stage of the Rust port.
  Once AI is re-prioritized, this is a one-step rename.

### RulesClass member offset map (cumulative across rounds)

Partial map discovered to date:
- `+0x8ac..+0xadc`  AI build lists (DynamicVector<BuildingTypeClass*>)
- `+0x10a0..+0x1768` AI tuning doubles/ints
- `+0x17e0..+0x17e3` AI strategic bool flags (Paranoid, CompEasyBonus)
- `+0x1838..+0x184c` ElevationModel (1 int + 2 doubles)
- `+0x1850..+0x185c` WallModel (1 bool + 1 double)
- `+0x1874`          ColorAdd array start

This will be useful for any future RulesClass struct-definition pass.

---

## 2026-05-17 — RulesClass__ReadDifficulty + Smudge globals (round 8)

**Focus track:** `rules-class-process-inline-dispatchers` — partial progress.

### New label (HIGH confidence)

- **`0x0066d270`** `FUN_0066d270` → `RulesClass__ReadDifficulty`

  Reads DifficultyClass settings from an INI section into a `double[]`.
  Identity VERIFIED via the exact INI key set matching the canonical
  DifficultyClass schema: FirePower, Groundspeed, Airspeed, Armor, ROF, Cost,
  BuildTime, RepairDelay, BuildDelay, BuildSlowdown, DestroyWalls, ContentScan.

  Called 3× from `RulesClass__Process` with section names `[Easy]`,
  `[Normal]`, `[Difficult]` to populate the three DifficultyClass slots on
  the RulesClass instance.

  **DifficultyClass struct layout fully reverse-engineered**:
  - `+0x00` FirePower (double, default 1.0)
  - `+0x08` Groundspeed (double, default 1.0)
  - `+0x10` Airspeed (double, default 1.0)
  - `+0x18` Armor (double, default 1.0)
  - `+0x20` ROF (double, default 1.0)
  - `+0x28` Cost (double, default 1.0)
  - `+0x30` BuildTime (double, default 1.0)
  - `+0x38` RepairDelay (double, default 0.02)
  - `+0x40` BuildDelay (double, default 0.03)
  - `+0x48` BuildSlowdown (bool, default false)
  - `+0x49` DestroyWalls (bool, default true)
  - `+0x4a` ContentScan (bool, default false)
  - Total size: 0x4c bytes.

  This is the canonical RA2/YR difficulty-modifier struct. Useful for any
  later RuleClass struct-definition pass or for verifying the Rust port's
  DifficultyClass equivalent.

### Hypothesis correction

In round 7, the plate comment on `RulesClass__Process` noted `FUN_00674650`
as LIKELY `RulesClass__ReadDifficultyAdjustments`. **This was wrong.**
Decompilation this round shows `FUN_00674650` is actually the Advanced
Command Bar reader — it reads `[AdvancedCommandBar]` or
`[MultiplayerAdvancedCommandBar]` based on the boolean flag, calls
`FUN_006cfd20` 25× for default buttons, then parses an INI `ButtonList=`
override. Total identity unrelated to difficulty.

Lesson: hypotheses written into plate comments before decompilation rot.
Don't speculate on names in plate comments — note "LIKELY <name> per pattern
X" with the pattern explicit, so a later verifier knows what was assumed.

### New labels (count globals from prior iteration)

- **`0x00a8ec1c`** → `g_SmudgeTypeClass_Array` (the SmudgeType array global)
- **`0x00a8ec28`** → `g_SmudgeTypeClass_Array_Count` (count)

  Both identified in round 7 during the SmudgeTypeClass__FindOrAllocate
  rename; deferred to this round to stay within the renames-per-iteration
  budget.

### Identification (parked — name uncertain)

- `0x0066d3a0` — Color scheme INI initializer. Reads `[Colors]` section,
  calls `FUN_00474c70` (LIKELY ColorScheme/RGB parser) and `FUN_0068c9c0`
  twice (LIKELY palette/remap ops). Candidate names: `Init_Color_Schemes`,
  `Read_Colors_INI`, `ColorScheme__Init_All`. Identity HIGH, name MEDIUM.

- `0x00674650` — Advanced Command Bar reader (see hypothesis correction
  above). Candidate names: `Init_Command_Bar`, `RulesClass__ReadCommandBar`,
  `SidebarClass__Init_Command_Bar`. Identity HIGH, name MEDIUM.

---

## 2026-05-17 — RulesClass__Process master dispatcher + bonus factory (round 7)

**Focus track:** `rules-class-read-all-sections-rename` — CLOSED.

### Master dispatcher rename

- **`0x00668bf0`** `FUN_00668bf0` → `RulesClass__Process`

  Identity VERIFIED via multiple independent signals:
  - `__thiscall` calling convention: `param_1` = RulesClass*, `param_2` = CCINIClass*
  - Writes member `this+0x1874` (ColorAdd array)
  - Dispatches inline to TypeClass list readers for `[Countries]`,
    `[OverlayTypes]`, `[SuperWeaponTypes]`, `[Warheads]`, `[SmudgeTypes]`,
    `[TerrainTypes]`, then calls the 8 already-labeled section-reader siblings
    (Building/Vehicle/Aircraft/Infantry/Animation/VoxelAnim/ParticleType/
    ParticleSystem), then 15+ already-labeled `RulesClass__Read<X>` subreaders
    (Jumpjet, MultiplayerDialog, SpeedTypeLandTypeTable, IQ, General,
    CrateRules, CombatDamage, Radiation, AudioVisual, SpecialWeapons),
    finally `TiberiumClass__ReadINI_All`.
  - Final dispatch `FUN_00674650(this, mode_flag)` is gated by `g_GameMode`
    (skipped when mode is 0 or 5 — i.e. campaign/skirmish only).
  - Caller: `ScenarioClass__Full_Init` @ 0x00686b20 (also called from two
    functions currently mislabeled as `CDFileClass__Constructor` — worth a
    future spot check; CDFileClass shouldn't be invoking rules processing).

  Naming choice: `RulesClass__Process` is the canonical Westwood name for
  top-level INI processors of this shape. Alternative candidates
  (`__Read_INI`, `__ReadAllSections`) were less likely on prior-base-rate
  grounds. Identity is rock-solid; the only uncertainty is the exact
  Westwood method name. Confidence: HIGH on identity, MEDIUM-HIGH on name.

### Bonus rename (canonical pattern catch)

- **`0x006b5910`** `FUN_006b5910` → `SmudgeTypeClass__FindOrAllocate`

  Discovered via decompilation of `RulesClass__Process`, which calls it
  inline from the `[SmudgeTypes]` section dispatch. Matches the now-locked
  canonical FindOrAllocate template (two-sentinel guard, search by
  `name@+0x24`, fall-through to `operator_new + Constructor`). Calls
  `SmudgeTypeClass__Constructor` (already labeled).

  Pattern body: `sizeof(SmudgeTypeClass) = 0x2a4`. Array at `DAT_00a8ec1c`,
  count at `DAT_00a8ec28` — rename deferred to next iteration.

### New label (count global)

- **`0x00a83cf0`** → `g_UnitTypeClass_Array_Count`

  Identified during round 5 factory verification; deferred to this round.

### Aggregated progress (rounds 1–7)

- Functions renamed/fixed (this session): 17
- Globals/data renamed (this session): 6
- Canonical Westwood FindOrAllocate pattern locked in (7+ confirmed instances)
- TypeClass sizeofs catalogued: 8 confirmed (Anim, VoxelAnim, Building,
  Unit, Aircraft, Infantry, Smudge, ParticleType)
- 2 mislabels resolved (`AnimTypeClass__FindByName`, Aircraft/Infantry
  array global swap)
- 1 ground-truth verification method established (vtable assignment in
  constructor bodies as strongest identity signal)

---

## 2026-05-17 — Aircraft/Infantry array global swap fix (round 6)

**Focus track:** `aircraft-infantry-array-swap-fix` — CLOSED.

### Mislabel fix (swap)

The two TypeClass array globals were transposed. Resolved via 3-step rename:

1. `g_InfantryTypeClass_Array` @ 0x00a8b21c → `g_AircraftTypeClass_Array_TMP_SWAP`
2. `g_AircraftTypeClass_Array` @ 0x00a8e34c → `g_InfantryTypeClass_Array`
3. `g_AircraftTypeClass_Array_TMP_SWAP` → `g_AircraftTypeClass_Array`

Final state:
- **0x00a8b21c** → `g_AircraftTypeClass_Array` (the aircraft type array)
- **0x00a8e34c** → `g_InfantryTypeClass_Array` (the infantry type array)

### New labels (count globals)

- **0x00a8b228** → `g_AircraftTypeClass_Array_Count` (incremented by
  `AircraftTypeClass__Constructor` when pushing self into the array)
- **0x00a8e358** → `g_InfantryTypeClass_Array_Count` (incremented by
  `InfantryTypeClass__Constructor` similarly)

### Ground-truth verification method

Decompilation of both constructors before the swap confirmed identity with
zero ambiguity:

- `AircraftTypeClass__Constructor` @ 0x0041c8b0 sets
  `*param_1 = &vtable__AircraftTypeClass` and then pushes self into the global
  that was labeled `g_InfantryTypeClass_Array` → that global IS the aircraft
  array.
- `InfantryTypeClass__Constructor` @ 0x005236a0 sets
  `*param_1 = &vtable__InfantryTypeClass` and then pushes self into the global
  that was labeled `g_AircraftTypeClass_Array` → that global IS the infantry
  array.

**Verified vtable-as-ground-truth principle:** Every TypeClass constructor sets
a specific `vtable__<X>TypeClass` pointer before any state mutation. That
pointer assignment is the strongest available evidence for class identity —
stronger than the label names on globals (which can be wrong) or caller
wiring (which can also be confused if the caller itself is mislabeled).
Future TypeClass disambiguation passes should follow this principle.

### Post-fix verification

Re-decompilation of `AircraftTypeClass__FindOrAllocate` @ 0x0041cef0 reads
cleanly with consistent names throughout: iterates `g_AircraftTypeClass_Array`
with `g_AircraftTypeClass_Array_Count`, allocates 0xe10 bytes, calls
`AircraftTypeClass__Constructor`. No further inconsistencies.

### DynamicVectorClass struct layout discovered

The two array-group neighborhoods reveal a 6-field DynamicVectorClass layout:
- `+0x00`: vtable pointer (e.g. `DAT_00a8b218`, `DAT_00a8e348`)
- `+0x04`: array pointer (`g_<TC>_Array`)
- `+0x08`: capacity (`DAT_00a8b220`, `DAT_00a8e350`)
- `+0x0d`: byte flag (`DAT_00a8b225`, `DAT_00a8e355`)
- `+0x10`: count (`g_<TC>_Array_Count`)
- `+0x14`: growth increment (`DAT_00a8b22c`, `DAT_00a8e35c`)

Worth a future struct-definition pass once more DynamicVectorClass methods
are labeled.

---

## 2026-05-17 — TypeClass FindOrAllocate factories (round 5) + mislabel discovery

**Focus track:** `rules-class-section-reader-factories` — final 3 factories.
**Track outcome:** CLOSED. All 8 section-reader factories now labeled.

### New labels (VERIFIED — canonical Westwood pattern)

- **`0x007480d0`** → `UnitTypeClass__FindOrAllocate`
  - Two-sentinel guard + search of `g_UnitTypeClass_Array` (count `DAT_00a83cf0`)
    + `operator_new(0xe78)` + `UnitTypeClass__Constructor`.
  - Called by `RulesClass__ReadVehicleTypes` (0x00672360) — INI section
    string `PTR_s_VehicleTypes_007f0cb0`.
  - All references in body are self-consistent — no mislabel here.

- **`0x0041cef0`** → `AircraftTypeClass__FindOrAllocate`
  - Two-sentinel guard + linear search + `operator_new(0xe10)` +
    `AircraftTypeClass__Constructor`.
  - Called by `RulesClass__ReadAircraftTypes` (0x006723d0) — INI section
    string `PTR_s_AircraftTypes_007f0cb4`. Identity is iron-clad on this chain.

- **`0x00524cb0`** → `InfantryTypeClass__FindOrAllocate`
  - Two-sentinel guard + linear search + `operator_new(0xed0)` +
    `InfantryTypeClass__Constructor`.
  - Called by `RulesClass__ReadInfantryTypes` (0x00672280) — INI section
    string `PTR_s_InfantryTypes_007f0ca8`. Identity is iron-clad.

### Mislabel discovery (NOT FIXED THIS ITERATION — parked as next focus track)

Decompilation of `AircraftTypeClass__FindOrAllocate` and
`InfantryTypeClass__FindOrAllocate` revealed that the two TypeClass array
globals `g_AircraftTypeClass_Array` and `g_InfantryTypeClass_Array` are
SWAPPED in their current labels.

Evidence:
- The Aircraft factory (whose identity is VERIFIED via the upstream reader's
  INI section string) references the global currently labeled
  `g_InfantryTypeClass_Array` — but since this IS the aircraft factory, that
  global is in fact the aircraft array.
- The Infantry factory (same verification chain) references the global
  currently labeled `g_AircraftTypeClass_Array` — which is in fact the
  infantry array.

This is the second mislabel from this cluster (first was `AnimTypeClass__FindByName`).
Suggests an earlier labeling pass mass-applied TypeClass names to globals by
positional vtable-scan order without verifying which class actually backs which
array. Worth a broader audit of other TypeClass globals (UnitType array label
is internally consistent in its factory body, but that's not proof of correct
identity — could be coincidence).

Fix is deferred to next iteration because it requires a 3-step rename to avoid
name-collision during the swap, and the iteration limit is 2-4 changes.

### Track close-out summary

**`rules-class-section-reader-factories`** is closed:
- 8 factory functions labeled (4 newly verified this iteration cluster + 4
  pre-existing or fixed earlier)
- Canonical Westwood TypeClass FindOrAllocate pattern fully characterized
- 6 TypeClass sizeofs confirmed (Anim=0x378, VoxelAnim=0x308, Building=0x1798,
  Unit=0xe78, Aircraft=0xe10, Infantry=0xed0)
- 1 mislabel fixed (AnimTypeClass__FindByName → __FindOrAllocate)
- 1 mislabel discovered, parked for next track (Aircraft/Infantry array swap)

---

## 2026-05-17 — TypeClass FindOrAllocate factories (round 4)

**Focus track:** `rules-class-section-reader-factories` — verify each section-reader's
factory callee against the canonical Westwood FindOrAllocate signature.

### Mislabel fix

- **`0x00428b80`**: `AnimTypeClass__FindByName` → `AnimTypeClass__FindOrAllocate`

  Evidence (decompilation-direct):
  - Body matches the canonical FindOrAllocate template (two sentinel strcmps on
    `<none>` + `<noname>`, linear name search of `g_AnimTypes_Array` by name@+0x24,
    fall-through to `operator_new(0x378)` + `AnimTypeClass__Constructor`).
  - A pure `FindByName` cannot call a constructor or allocate. The presence of
    both makes the old label structurally impossible.
  - The reader `RulesClass__ReadAnimations` (0x006728b0) needs a create-or-find
    factory because the registration loop runs over `[Animations]` entries that
    do not yet exist as objects — `FindByName` would have silently skipped every
    entry on the first pass.
  - `sizeof(AnimTypeClass)` = `0x378`, consistent with `g_AnimTypes_Array` layout.

### New labels (VERIFIED — canonical Westwood pattern)

- **`0x0074b960`** → `VoxelAnimTypeClass__FindOrAllocate`
  - Two-sentinel guard + search + `operator_new(0x308)` + `VoxelAnimTypeClass__Constructor`.
  - Called by `RulesClass__ReadVoxelAnims` (0x00672920).
  - Array global: `DAT_00a8eb2c` (LIKELY `g_VoxelAnimTypeClass_Array`), count at
    `DAT_00a8eb38`. Rename of the globals deferred as a side-quest.

- **`0x004653c0`** → `BuildingTypeClass__FindOrAllocate`
  - Two-sentinel guard + search + `operator_new(0x1798)` + `BuildingTypeClass__constructor`.
  - Called by `RulesClass__ReadBuildingTypes` (0x00672660).
  - Array global: `g_BuildingTypeClass_Array` (already labeled), count
    `g_BuildingTypeClass_Array_Count` (already labeled).

### Canonical pattern locked in

All TypeClass `FindOrAllocate` factories in gamemd.exe follow this body shape:

```
if (strcmp(s_None /* @0x00817474 */, name) != 0 &&
    strcmp(s_NoName /* @0x00817694 */, name) != 0) {
    for (i = 0; i < g_<TC>_Count; i++)
        if (strcmp(g_<TC>_Array[i]->name (@+0x24), name) == 0)
            return g_<TC>_Array[i];
    void *p = operator_new(sizeof(<TC>));
    if (p) return <TC>__Constructor(p, name);
}
return 0;
```

This signature is now strong enough to verify the remaining 3 factories
(`FUN_007480d0`, `FUN_0041cef0`, `FUN_00524cb0`) on inspection alone.

### Track progress

4 of 8 factories verified + labeled this iteration. Remaining: UnitTypeClass,
AircraftTypeClass, InfantryTypeClass (LIKELY targets queued in state file).

---

## 2026-05-17 — ParticleType vs ParticleSystemType disambiguation

**Focus track:** ParticleType / ParticleSystemType factory cluster.

### Mislabel fix

- **`0x00645430`**: `ParticleSystemTypeClass__FindOrCreate` → `ParticleTypeClass__FindOrCreate`

  Evidence (decompilation-direct):
  - Allocates `0x318` bytes (`sizeof(ParticleTypeClass)`), not `0x310` (`sizeof(ParticleSystemTypeClass)`).
  - Calls `ParticleTypeClass__Constructor`, not `ParticleSystemTypeClass__Constructor`.
  - Operates on `g_ParticleTypeClass_Array` (`0x00a83d9c`, count `0x00a83da8`), not
    `g_ParticleSystemTypeClass_Array` (`0x00a83d6c`).
  - Returns an INDEX into the array, not a pointer (distinguishes it from the
    `FindOrAllocate` sibling).
  - Sentinel handling: checks only `"<none>"`, not the `"<none>"`/`"none"` pair used
    by FindOrAllocate variants.

  The prior name had already been flagged in a plate comment by an earlier iteration;
  this pass confirmed via independent decompilation and applied the rename.

### New labels

- **`0x00645820`**: `FUN_00645820` → `ParticleTypeClass__FindOrAllocate`

  Sibling of `ParticleTypeClass__FindOrCreate`. Standard Westwood TypeClass
  FindOrAllocate pattern — same array, same constructor, `"<none>"`/`"none"`
  sentinel pair, allocates `0x318`, returns POINTER.

  Confirmed via caller `RulesClass__ReadParticleTypes` (below), which iterates
  the rules `[Particles]` section and feeds each entry name to this function.

- **`0x00672a00`**: `FUN_00672a00` → `RulesClass__ReadParticleTypes`

  Byte-identical structure to the inline `[OverlayTypes]` / `[SuperWeaponTypes]` /
  `[Warheads]` / `[SmudgeTypes]` / `[TerrainTypes]` loops in the master rules
  dispatcher `FUN_00668bf0`: count entries via `INIClass__Section_Entry_Count`,
  read each entry name via `CCINIClass__ReadString`, call
  `ParticleTypeClass__FindOrAllocate` per entry.

  Sole INI section referenced: `"Particles"`. Sole factory called:
  `ParticleTypeClass__FindOrAllocate`.

### Renamed data

- **`0x00a83d9c`** → `g_ParticleTypeClass_Array` (was `DAT_00a83d9c`)
- **`0x00a83da8`** → `g_ParticleTypeClass_Array_Count` (was `DAT_00a83da8`)

### Plate comments

- `0x00645430`: rewritten — removed mislabel-warning notice, replaced with verified
  description and a short history line documenting the rename rationale.
- `0x00645820`: full FindOrAllocate description with confidence rationale.
- `0x00672a00`: full rules-section-reader description, cross-referencing the
  sibling FUN sequence in `FUN_00668bf0` (likely the AnimationTypes / BuildingTypes /
  InfantryTypes etc. readers — flagged as a follow-up focus track).

### Pending follow-ups (forwarded to `ghidra_loop_state.md`)

- The sibling sequence in `FUN_00668bf0` — `FUN_00672660`, `FUN_00672360`,
  `FUN_006723d0`, `FUN_00672280`, `FUN_006728b0`, `FUN_00672920`, `FUN_00672a70` —
  appears to follow the same single-section-list-reader pattern as
  `RulesClass__ReadParticleTypes`. Each is a candidate for a 2–4 verified rename
  iteration on its own.
- `FUN_00668bf0` itself is the master `RulesClass__ReadAllSections` (or similar)
  dispatcher and is a strong candidate for renaming once the children are settled.

---

## 2026-05-17 — Rules section-reader siblings (round 2)

**Focus track:** `rules-class-section-readers`.

### New labels

Four more siblings of `RulesClass__ReadParticleTypes` confirmed and renamed. Each
is byte-identical in shape to the canonical template: read section entry count,
loop the index, read each entry name, call a TypeClass factory with the name.

- **`0x00672660`** → `RulesClass__ReadBuildingTypes`
  - INI section pointer: `PTR_s_BuildingTypes_007f0cc0` ("BuildingTypes")
  - Factory: `FUN_004653c0` (LIKELY `BuildingTypeClass__FindOrAllocate`)

- **`0x00672360`** → `RulesClass__ReadVehicleTypes`
  - INI section pointer: `PTR_s_VehicleTypes_007f0cb0` ("VehicleTypes")
  - Factory: `FUN_007480d0` (LIKELY `UnitTypeClass__FindOrAllocate`)

- **`0x006723d0`** → `RulesClass__ReadAircraftTypes`
  - INI section pointer: `PTR_s_AircraftTypes_007f0cb4` ("AircraftTypes")
  - Factory: `FUN_0041cef0` (LIKELY `AircraftTypeClass__FindOrAllocate`)

- **`0x00672280`** → `RulesClass__ReadInfantryTypes`
  - INI section pointer: `PTR_s_InfantryTypes_007f0ca8` ("InfantryTypes")
  - Factory: `FUN_00524cb0` (LIKELY `InfantryTypeClass__FindOrAllocate`)

### Plate comments

- Each of the four readers received a plate comment documenting the section name,
  the LIKELY factory, the sibling relationship, and the dispatch site
  (`FUN_00668bf0`).

### Confidence

- VERIFIED for the four reader identities (section pointer + sole factory call
  match the canonical template established by `RulesClass__ReadParticleTypes`).
- LIKELY for the four factory callees — not renamed this iteration. Each must
  be confirmed via alloc size + sentinel pattern + array globals (the rigor used
  for `ParticleType`/`ParticleSystemType` disambiguation) before promotion.

### Pending follow-ups (forwarded to `ghidra_loop_state.md`)

- Remaining sibling candidates: `FUN_006728b0`, `FUN_00672920`, `FUN_00672a70`.
- Factory verification pass: `FUN_004653c0`, `FUN_007480d0`, `FUN_0041cef0`,
  `FUN_00524cb0`.
- `FUN_00668bf0` rename to `RulesClass__ReadAllSections` (or canonical Westwood
  name) once siblings and factories are settled.

---

## 2026-05-17 — Rules section-reader siblings (round 3 — track CLOSED)

**Focus track:** `rules-class-section-readers` — CLOSED. All 8 sibling readers
in the `FUN_00668bf0` dispatch group are now labeled.

### New labels

- **`0x006728b0`** → `RulesClass__ReadAnimations`
  - INI section pointer: `s_Animations_0083d0dc` ("Animations")
  - Factory: `AnimTypeClass__FindByName` (SUSPICIOUS — see mislabel note below)

- **`0x00672920`** → `RulesClass__ReadVoxelAnims`
  - INI section pointer: `s_VoxelAnims_0083d0e8` ("VoxelAnims")
  - Factory: `FUN_0074b960` (LIKELY `VoxelAnimTypeClass__FindOrAllocate`)

- **`0x00672a70`** → `RulesClass__ReadParticleSystems`
  - INI section pointer: `s_ParticleSystems_0083d100` ("ParticleSystems")
  - Factory: `ParticleSystemTypeClass__FindOrAllocate` (already labeled — strong
    cross-confirmation of the canonical pattern)

### Mislabel suspicion flagged for future verification

- **`AnimTypeClass__FindByName`** (called by `RulesClass__ReadAnimations`):

  The canonical Westwood section-reader pattern is a register-from-INI loop: each
  iteration of the read pulls an entry name from `rules(md).ini` and feeds it to
  a factory that must CREATE the corresponding TypeClass instance (with a
  find-fallback for already-registered duplicates). A pure `FindByName` would
  silently skip every entry on first pass — there is no upstream registration
  call site for animation types in `FUN_00668bf0`.

  Comparison: every other reader in this cluster calls a `*FindOrAllocate` (or
  unverified `FUN_*` matching that pattern). `AnimTypeClass__FindByName` is the
  only outlier.

  Verdict: LIKELY mislabeled. Probable correct identity:
  `AnimTypeClass__FindOrAllocate`. Not corrected this iteration — pending
  verification against alloc size + constructor call + array global. Flagged in
  state file as HIGH-priority for the next-track factory verification pass.

### Plate comments

- Each of the three readers received a plate comment.
- The ReadAnimations plate comment documents the suspicious factory name and the
  rationale for flagging it.

### Confidence

- VERIFIED for the three reader identities — section pointer + sole factory call
  + body shape match the canonical template byte-for-byte.

### Track promotion

- `rules-class-section-readers` → CLOSED.
- New active track: `rules-class-section-reader-factories` — verify and rename
  the 5 unnamed factory callees and confirm/fix the `AnimTypeClass__FindByName`
  suspicion.
