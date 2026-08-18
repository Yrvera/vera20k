# Spine Rung #20 — T. MAIN object vector tick (universal per-object AI fan-out)

**Status:** VERIFIED from binary this session. Image base 0x400000.
**Rung:** #20 of `LogicClass::PerTickUpdate` (`LogicClassPerTickUpdateLiveVector` @ `0x0055AFB0`).
**Driver:** the polymorphic dispatch **`vt+0x5c` (ObjectClass vtable slot 23 = `ObjectClass::AI` @ `0x005F3E70`)** per live object, called from the inline loop @ `0055b5fb`-`0055b619`.
**Authority:** binary -> Ghidra. Disassembly of the spine body is ground truth (decompiler
reorders/elides surrounding rungs); the loop body itself is unambiguous in both.

Verification calls used (all this session):
- `decompile_function 0x0055AFB0` + `disassemble_function 0x0055AFB0` (the spine body; the rung loop @ `0055b5fb`)
- `decompile_function 0x0055D360` + `disassemble_function 0x0055D360` (Main_Tick; the only caller)
- `get_xrefs_to 0x0055AFB0` -> single caller `0055dc9e in Main_Tick`; call-site receiver read at `0055dc99 MOV ECX,0x87f778`
- `get_xrefs_to 0x0087f778` (LogicClass singleton; writers = ObjectClass Reveal/Conceal/Destructor/Detach)
- `read_memory 0x007EF060` (ObjectClass primary vtable) -> slot 23 (offset 0x5c) = `0x005F3E70`
- `decompile_function 0x005F3E70` (`ObjectClass__AI`, the base slot-23 body)
- `decompile_function 0x005f503b` (`ObjectClass__Reveal`, the registration entry calling the active-vector add)
- `decompile_function 0x0055BAA0` (`FUN_0055baa0`, active-vector add-once gated by `+0x98`) + `get_function_callers 0x0055BAA0`
- `get_function_callees 0x006F9E50` (`TechnoClass::AI_Update`) -> contains `Random__RandomRanged 0x0065c7e0`
- `get_function_callees 0x004DA530` (`FootClass::AI`) -> contains `Random__Next 0x0065c780`
- `disassemble_function 0x004DA530` (FootClass::AI; concrete RNG draw site @ `0x004daac0`, ECX=`0x00886b88`)
- `get_function_by_address 0x0065c780` (`Random__Next`) / `0x0065c7e0` (`Random__RandomRanged`)
- `get_xrefs_to 0x00886b88` (g_MainRng; writers = Init_Random_Number_System)
- `disassemble_function 0x0052FE00` (`Init_Random_Number_System`; seeds Scen+0x218 AND 0x00886b88)

---

## Order placement (the lockstep contract)

The rung is the inline loop at `0055b5fb`-`0055b61b`, immediately after Rung S
(`0x004c54a0` timed-effect purge @ `0055b5f6`) and immediately before Rung U (AnimClass
mode-gated tick @ `0055b61b`):

```
0055b5f6: CALL 0x004c54a0           ; RUNG S (alpha/effect purge)
0055b5fb: MOV  EDI,[ESP + 0x10]     ; EDI = param_1 = LogicClass this (0x0087f778)
0055b5ff: XOR  ESI,ESI              ; i = 0
0055b601: MOV  EAX,[EDI + 0x10]     ; count = this->vector.Count   (this+0x10)
0055b604: TEST EAX,EAX
0055b606: JLE  0x0055b61b           ; gate: skip if count <= 0
0055b608: MOV  EAX,[EDI + 0x4]      ; base = this->vector.Items    (this+0x04)
0055b60b: MOV  ECX,[EAX + ESI*4]    ; obj = base[i]   (ECX = receiver)
0055b60e: MOV  EDX,[ECX]            ; vtable = *obj
0055b610: CALL [EDX + 0x5c]         ; ObjectClass::AI (slot 23) — polymorphic per subclass
0055b613: MOV  EAX,[EDI + 0x10]     ; RE-READ count each iteration
0055b616: INC  ESI
0055b617: CMP  ESI,EAX
0055b619: JL   0x0055b608
0055b61b: ...                        ; RUNG U (AnimClass): MOV EAX,[0x00a8b238] ...
```

Confirmed via `disassemble_function 0x0055AFB0`. Forward walk, **count re-read each
iteration** (`0055b613`) — so an object revealed mid-pass (added to the back of the vector)
is ticked the same tick. This re-read is part of the lockstep contract.

### `param_1` identity (the receiver / vector owner)

`get_xrefs_to 0x0055AFB0` -> exactly one caller, `Main_Tick @ 0x0055D360`. The call site:
```
0055dc99: MOV ECX,0x87f778      ; ECX = the LogicClass singleton
0055dc9e: CALL 0x0055afb0
```
So **`param_1` = `0x0087f778`** — the **LogicClass singleton object**. Its embedded
`DynamicVectorClass` is:
- **`+0x04`** = items base ptr,
- **`+0x10`** = element count.

(Static `read_memory 0x0087f778` returns zeros — the vector is populated at runtime, not in
the image.)

---

## Purpose (one line)

**The primary live-object AI fan-out:** walk every revealed/active world object registered in
the LogicClass live vector and dispatch its per-object AI (vtable slot 23, `vt+0x5c`).

---

## What it walks / dispatches — scope is UNIVERSAL, not just bullets/particles

The rung's seed label ("bullets / voxel-anims / particles") **undersells the scope.** The
vector at `0x0087f778+0x4` holds **every live `ObjectClass`-derived object**, because
registration goes through the base lifecycle methods that all object types inherit:

- `get_xrefs_to 0x0087f778` shows the writers are `ObjectClass__Reveal` (0x005f503b /
  0x005f4ec0), `ObjectClass__Conceal`, `ObjectClass__Destructor`, and
  `Detach_From_All_Lists` — all base ObjectClass machinery.
- The add-once helper `FUN_0055baa0` (`decompile_function 0x0055BAA0`) is gated by the
  ObjectClass membership bit **`+0x98`** (test-and-set), and is called from
  `ObjectClass__Reveal 0x005f4ec0`, `BuildingLightClass__Constructor`,
  `TechnoClass__SetInOpenTransport`, and two FUN_ helpers (`get_function_callers 0x0055BAA0`).

So the dispatched `vt+0x5c` reaches the slot-23 override of **whatever subclass** each object
is. Cross-referenced with `TECHNOCLASS_AI_MIGRATION_BOUNDARY_GHIDRA_REPORT.md` (verified):
- `UnitClass::AI` @ `0x007360C0`
- `InfantryClass::AI` @ `0x0051BAB0`
- `AircraftClass::AI` @ `0x00414BB0`
- `BuildingClass::Update` (slot 23 override)
- `FootClass::AI` @ `0x004DA530` (called by UnitClass/InfantryClass/AircraftClass AI), which
  immediately calls `TechnoClass::AI_Update @ 0x006F9E50`
- bullets, voxel anims, particle systems, building lights, terrain — every other
  ObjectClass-derived live object
- the **base** `ObjectClass::AI @ 0x005F3E70` for objects that do not override slot 23.

**This rung is therefore where the bulk of per-unit gameplay logic runs** — mission
dispatch, movement step, target acquisition, firing, scatter, voice — for every unit,
building, and projectile in the world.

### The base body (`ObjectClass__AI 0x005F3E70`, slot 23)

`decompile_function 0x005F3E70`: handles the spawn-VocClass cue (`VocClass__PlayAt`), the
drop-in / height-settle physics gated by `+0x8d` (fall flag), per-frame Z update via
`Math__ftol` (float->long, **not RNG**), `DisplayClass__Submit_Object` re-submit on level
change, and a Mission-4 (cell==2 i.e. water?) splash `AnimClass__Constructor`. **No RNG in
the base body.** Subclasses that override slot 23 do their own work (and most draw RNG).

---

## Exact gate (confirmed)

**Gate = the vector count `*(int*)(param_1 + 0x10) > 0`** (`0055b604 TEST EAX,EAX / JLE`).
That is the *only* gate. It is **NOT mode-gated** — unlike the very next rung (U, AnimClass)
which is gated `g_GameMode != 0 && g_GameMode != 5`. Confirms the seed fact: bullets /
particles / units (Rung T) tick unconditionally every frame; standalone anims (Rung U) are
mode-gated and run *after* T. In any live skirmish the count is always > 0 (units/buildings
present), so this rung is effectively unconditional.

---

## RNG: the loop draws NO RNG itself — all RNG is inside the `vt+0x5c` subtree, BOTH streams

The rung loop body (`0055b5fb`-`0055b619`) contains **no RNG call**. Every draw happens
inside the polymorphic `vt+0x5c` override and its callees. Because this is a fan-out over the
entire live-object AI surface, the **number of draws per tick is not statically enumerable**
— it depends on how many units exist and which conditional branches (fire, scatter, miss,
death, voice, etc.) fire this tick. The lockstep-relevant property is the **per-callsite
stream binding** and the **deterministic iteration order** (forward, count re-read), both of
which are fixed.

Stream binding is **per-callsite ECX** (consistent with
`reference_rng_instance_routing_truth.md`). Two distinct RandomClass instances are reached
from this rung's subtree, seeded together in `Init_Random_Number_System`
(`disassemble_function 0x0052FE00`):

- **`Scen->Random`** = `Scen + 0x218` (Scen = `[0x00a8b230]`) — seeded at `0052fe26 LEA
  EDI,[ECX+0x218]`. The **lockstep gameplay stream.** Drawn by gameplay-affecting AI logic.
- **`g_MainRng`** = `0x00886b88` — seeded at `0052fe4c MOV EDI,0x886b88`. The **cosmetic /
  non-lockstep stream.** Drawn for sound/voice/visual selection that must not perturb the
  simulation hash.

`Random__Next` = `0x0065c780`; `Random__RandomRanged` = `0x0065c7e0`
(`get_function_by_address`).

### Concrete evidence of BOTH streams in this rung's subtree

1. **`FootClass::AI` voice draw uses `g_MainRng`** (cosmetic) —
   `disassemble_function 0x004DA530` @ `0x004daac0`:
   ```
   004daac0: MOV ECX,0x886b88        ; ECX = g_MainRng  (NOT Scen+0x218)
   004daacb: CALL 0x0065c780         ; Random__Next
   004daad3: DIV [EDI+0x10]          ; result % count -> pick a random idle/move voice index
   004daae2: CALL 0x007509e0         ; VocClass__PlayAt the chosen sample
   ```
   Correctly routed to g_MainRng: voice selection is cosmetic and must stay out of the
   lockstep hash.

2. **`TechnoClass::AI_Update` draws `Random__RandomRanged`** —
   `get_function_callees 0x006F9E50` lists `Random__RandomRanged @ 0x0065c7e0`. (Gameplay AI:
   scatter / passive-acquire / damage-particle jitter — per-callsite ECX determines stream;
   the gameplay-affecting ones bind to Scen->Random.)

3. **`FootClass::AI` callees** (`get_function_callees 0x004DA530`) include `Random__Next
   0x0065c780` (the voice draw above) plus `Math__ftol` (not RNG).

Many other subclass AI paths in this subtree draw Scen->Random for lockstep-relevant
decisions (e.g. `InfantryClass::PerCellProcess`, `UnitClass::PerCellProcess`,
`TechnoClass::ReceiveDamage`, `TechnoClass::IncreaseGattlingStage`, HouseClass-adjacent —
all visible in `get_xrefs_to 0x00886b88`/`0x0065c780` as RNG callsites, though their *stream*
must be read per-callsite). Enumerating every draw is out of scope for the spine; the
load-bearing facts are: (a) the loop itself draws nothing, (b) iteration is deterministic,
(c) both streams appear in the subtree and binding is per-callsite ECX.

---

## Active in YR / Tiberian Sun legacy

**Active in YR: YES — unconditionally, every tick.** This is the single most active rung in
the ladder: it advances every unit, building, projectile, and effect in the world. Its
player-visible output is essentially *all* of moment-to-moment gameplay (units move, turrets
track, weapons fire, infantry die, miners harvest, etc.).

**TS legacy: NO.** The dispatch mechanism (live-vector + slot-23 AI) and its YR subclass
overrides (UnitClass/InfantryClass/AircraftClass/BuildingClass/TechnoClass/FootClass) are
all live in standard YR. Individual *branches inside* a given subclass AI may be TS-dormant,
but that is a per-subclass concern, not a property of this rung.

---

## Field / global reference (verified this session)

| Symbol | Address/offset | Meaning | Verified by |
|---|---|---|---|
| LogicClass singleton (`param_1`) | `0x0087f778` | live-vector owner / `this` | asm `0055dc99` MOV ECX,0x87f778 |
| live vector items ptr | `param_1 + 0x04` | object array base | asm `0055b608` |
| live vector count | `param_1 + 0x10` | object count (re-read each iter) | asm `0055b601/0055b613` |
| AI dispatch slot | `vtable + 0x5c` (slot 23) | per-object AI | asm `0055b610`; `read_memory 0x007EF060`[23] |
| `ObjectClass::AI` (base slot 23) | `0x005F3E70` | base per-object AI (no RNG) | vtable slot 23 read |
| ObjectClass primary vtable | `0x007EF060` | slot table | `read_memory` |
| active-vector membership bit | `ObjectClass + 0x98` | gates registration | `decompile_function 0x0055BAA0` |
| active-vector add-once | `0x0055BAA0` (`FUN_0055baa0`) | test/set `+0x98` then insert | `get_function_callers` |
| `ObjectClass::Reveal` (registration) | `0x005f4ec0` / `0x005f503b` | calls the add-once | xrefs to 0x0087f778 |
| `UnitClass::AI` | `0x007360C0` | unit slot-23 override | TECHNOCLASS_AI_MIGRATION report |
| `InfantryClass::AI` | `0x0051BAB0` | infantry slot-23 override | TECHNOCLASS_AI_MIGRATION report |
| `AircraftClass::AI` | `0x00414BB0` | aircraft slot-23 override | TECHNOCLASS_AI_MIGRATION report |
| `FootClass::AI` | `0x004DA530` | common Foot AI (calls TechnoClass::AI_Update) | `disassemble_function` |
| `TechnoClass::AI_Update` | `0x006F9E50` | common Techno AI (draws RandomRanged) | `get_function_callees` |
| `Scen->Random` (lockstep RNG) | `Scen + 0x218` (Scen=`[0x00a8b230]`) | gameplay stream | asm `0052fe26` |
| `g_MainRng` (cosmetic RNG) | `0x00886b88` | non-lockstep stream | asm `0052fe4c`; `get_xrefs_to` |
| `Random__Next` | `0x0065c780` | RandomClass draw | `get_function_by_address` |
| `Random__RandomRanged` | `0x0065c7e0` | ranged RandomClass draw | `get_function_by_address` |
| `Init_Random_Number_System` | `0x0052FC20` (block) | seeds both RNG instances | `disassemble_function 0x0052FE00` |

---

## One-line summary for the ladder

**T. MAIN object vector tick @ `0055b5fb`-`0055b619`** — forward loop over the LogicClass
live vector (`this=0x0087f778`, items `+0x4`, count `+0x10`, **count re-read each iteration**),
calling **`vt+0x5c` (ObjectClass slot 23 = `ObjectClass::AI 0x005F3E70`)** on every revealed
ObjectClass. This is the **universal per-object AI fan-out** (units/infantry/aircraft/
buildings/bullets/voxel-anims/particles via their slot-23 overrides — NOT just bullets/
particles). Gate = **count > 0** only; **NOT mode-gated** (Rung U/AnimClass after it is). The
loop body draws **no RNG**; RNG is drawn inside the dispatched subclass AI — **both** streams
appear, per-callsite ECX: gameplay-affecting draws -> **Scen->Random** (`Scen+0x218`),
cosmetic voice/sound draws -> **g_MainRng** (`0x00886b88`, e.g. `FootClass::AI @ 0x004daac0`).
Number of draws/tick is not statically enumerable (scales with live-object count and which AI
branches fire); the lockstep guarantees are the deterministic forward iteration + per-callsite
stream binding. Active in YR: YES, every tick; not TS legacy.
