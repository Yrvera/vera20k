# Death-Weapon Writer Classification and Shadow Boundary

**Date:** 2026-07-23  
**Program:** active Yuri's Revenge `gamemd.exe`, x86 32-bit  
**Investigation mode:** exhaustive slice  
**Scope:** active-YR ordered death-weapon production from the fatal Techno/aircraft
entry through area collection, per-target damage receiver entry, recursive
death-weapon handling, and the corresponding current Rust shortcut  
**Output restriction:** research and implementation handoff only; no Rust source
was changed

## 1. Verdict

**The direct Rust death-AoE HP subtraction is not a distinct native HP writer.**
It is a collapsed approximation of a synchronous special producer which re-enters
the ordinary per-target damage receiver.

The native ownership boundary is:

```text
fatal Techno receiver / fatal aircraft crash path
  -> death-helper reachability gate
  -> FUN_0070D690 weapon selection and signed damage construction
  -> ephemeral BulletClass initialization
  -> synchronous WarheadTypeClass::Detonate
  -> Apply_area_damage target collection
  -> ordered, synchronous target ReceiveDamage calls
       -> possible nested fatal receiver
       -> possible nested death weapon and complete recursive chain
  -> resume the outer target sequence
  -> destroy ephemeral bullet
  -> resume the original fatal caller
```

Current Rust instead:

```text
ordinary damage batch
  -> fixed dead_entities list
  -> collect all death blasts
  -> perform death/lifecycle classification for all listed objects
  -> calculate unsigned AoE hits
  -> direct saturating_sub on target HP
```

That difference is **DRIFT** in gate semantics, weapon selection, signed damage
construction, exact coordinate, source identity, lifecycle order, per-target
receiver behavior, recursive closure, and same-stack timing.

The recommended staged migration remains shadow-first, but the shadow operation
must begin at the death special-producer boundary. Adding an authoritative
per-operation vitality transaction only around the existing direct subtraction
would formalize the wrong native operation.

This is a static mechanism proof. It is not retail runtime parity certification.

## 2. Duplication Guard and Evidence Basis

### 2.1 Existing research reconciled

The following current reports were read before the fresh pass:

- `DAMAGE_SPECIAL_PRODUCER_TIMING_REINVESTIGATION_2026-07-13.md`
- `DAMAGE_SIGNED_VITALITY_WRITER_AND_FATAL_HANDOFF_GHIDRA_REPORT.md`
- `combat/systems/suicide_weapons.md`

The July 13 special-producer report already established the broad death-weapon
route. This report does not replace it. It fresh-verifies the load-bearing native
sequence, closes the later signed-writer report's deferred classification of the
specific Rust death-AoE writer, and defines the safe shadow boundary against the
current feature worktree.

`combat/systems/suicide_weapons.md` remains useful as historical navigation, but
its hypotheses are not used as authority where they disagree with the fresh
binary evidence.

### 2.2 Fresh binary calls

All Ghidra evidence below came from the connected `gamemd.exe` instance through
read-only calls to the local Ghidra MCP backend:

- `GET /decompile_function?address=0x007122F0`
- `GET /disassemble_function?address=0x007122F0`
- `GET /disassemble_function?address=0x0070D690`
- `GET /decompile_function?address=0x00701900`
- `GET /disassemble_function?address=0x00701900`
- `GET /decompile_function?address=0x004CD600`
- `GET /disassemble_function?address=0x004CD600`
- `POST /disassemble_bytes` around `0x00746100..0x0074623F`
- `GET /decompile_function?address=0x0046B050`
- `GET /decompile_function?address=0x004664C0`
- `GET /decompile_function?address=0x004690B0`
- `GET /disassemble_function?address=0x004690B0`
- `GET /decompile_function?address=0x00489280`
- `GET /disassemble_function?address=0x00489280`
- read-only instruction, caller, and memory searches for `0xD18`,
  `0x0070D690`, and `0x007E1738`

The Ghidra database was not modified.

### 2.3 Current Rust surface

The source comparison used the uncommitted feature worktree
`feature/gsi-08-10-damage-authority`:

- `src/sim/combat/mod.rs:810-825` — `death_weapon_aoe`
- `src/sim/combat/mod.rs:849-1100` — `handle_entity_deaths`
- `src/sim/combat/combat_aoe.rs:81-218` — `apply_aoe_damage`
- `src/rules/object_type.rs:325-330,1021-1022` — parsed type fields
- `src/rules/weapon_type.rs:257` — parsed `Suicide`
- `src/rules/veterancy_abilities.rs` — parsed `Explodes` ability

The worktree's existing changes were left untouched.

## 3. Native Entry Points and Reachability

`FUN_0070D690 @ 0x0070D690` has three direct call instructions in the active
binary:

| Call | Route | Classification | Helper argument |
|---|---|---|---|
| `0x0070266D` | `TechnoClass::ReceiveDamage` fatal path | active YR | literal `0` |
| `0x004CD809` | `FlyLocomotionClass::Process` zero-HP crash path | active YR | literal `0` |
| `0x007461EF` | Tunnel locomotion fatal path | dormant TS legacy | literal `0` |

Fresh caller search found the Techno and Fly functions. Fresh raw disassembly also
confirmed the third call at `0x007461EF`. The Tunnel identity and standard-YR
dormancy classification remain supported by the earlier fresh COL/vtable proof in
`DAMAGE_SPECIAL_PRODUCER_TIMING_REINVESTIGATION_2026-07-13.md`; this pass did not
repeat the RTTI proof.

### 3.1 Techno fatal gate

At `0x00702572..0x00702672`, the Techno receiver reaches the helper if any one of
these conditions is true:

1. `TechnoType+0xD15 Explodes` is true;
2. the object's active veterancy tier grants the `Explodes` ability:
   - veteran checks type byte `+0x2A6`;
   - elite accepts `+0x2A6` or elite byte `+0x2B8`;
3. the current weapon exists and `WeaponType+0x144 Suicide` is true.

An explicit `DeathWeapon=` pointer is not itself tested by this gate.

The current weapon used by this gate is obtained before the gate through virtual
slot `+0x3F8` with the object's current weapon number. The death helper later has
its own weapon-selection sequence; the gate and selection are not one lookup.

### 3.2 Aircraft crash route

At `0x004CD7C5..0x004CD8A1`, the Fly locomotor reads the associated object's
health. When health equals zero, it calls the helper immediately, selects the
crash-impact sound, and then calls the object's `UnInit` virtual.

The helper's return register is overwritten by the following map-cell lookup.
The crash path therefore does not defer or schedule the death weapon.

### 3.3 Dormant route

The call at `0x007461EF` pushes zero, calls the same helper, and then continues to
the route's lifecycle virtual. It is binary-real but not an ordinary standard-YR
mechanic. It must not be promoted into the active implementation surface.

## 4. Type and Rules State

Fresh `TechnoTypeClass::ReadINI @ 0x007122F0` verification binds:

| INI key | Native state |
|---|---|
| `Explodes` | `TechnoType+0xD15`, byte |
| `DeathWeapon` | `TechnoType+0xD18`, pointer |
| `DeathWeaponDamageModifier` | `TechnoType+0xD1C`, float |

Constructor writes prove these defaults:

```text
DeathWeapon = null
DeathWeaponDamageModifier = 1.0f
```

`RulesClass::ReadCombatDamage` binds the `[CombatDamage] DeathWeapon` default to
`Rules+0xFDC`; the Rules constructor initializes that pointer to null before INI
loading.

Stock `rulesmd.ini` provides:

```ini
[CombatDamage]
DeathWeapon=DefaultDeathWeapon

[DefaultDeathWeapon]
Projectile=Invisible
Warhead=DeathWH

[DeathWH]
CellSpread=1.5
PercentAtMax=.5
```

`DefaultDeathWeapon` intentionally has no `Damage=` entry. The helper's Rules
fallback does not read weapon damage; it derives damage from the dying type's
Strength.

Stock YR also exercises multiple branches:

- explicit `DeathWeapon` plus `Explodes`, such as Terrorist, Demolition Truck,
  nuclear reactor, oil derrick, and barrels;
- explicit modifiers including `.5`, `.1`, and `.01`;
- aircraft crash types with explicit death weapons/modifiers where the Fly route
  supplies reachability;
- `Explodes=yes` types without an explicit death weapon, which require the
  virtual fallback or Rules default path.

## 5. Exact Helper Contract

### 5.1 Weapon selection

Fresh disassembly `0x0070D698..0x0070D724` proves this exact order:

1. obtain the dying object's TechnoType through virtual `+0x84`;
2. if `TechnoType+0xD18` is non-null, select that explicit `WeaponType`;
3. otherwise call object virtual `+0x3F4`;
4. if the returned structure and its first `WeaponType*` are non-null, select
   that virtual fallback weapon;
5. otherwise select `Rules+0xFDC`;
6. if the final selected weapon is null, return zero with no temporary bullet.

The exact semantic name of virtual `+0x3F4` is intentionally not guessed here.
Its load-bearing contract is the returned weapon-structure pointer and first
`WeaponType*`.

### 5.2 Signed base damage

For an explicit or virtual-fallback weapon:

```text
base = Math__ftol(
    (i32)selected_weapon.Damage
    * (float)dying_type.DeathWeaponDamageModifier
)
```

For the Rules default:

```text
base = Math__ftol((i32)dying_type.Strength * 0.5)
```

Fresh memory read at `0x007E1738` returned:

```text
00 00 00 00 00 00 E0 3F
```

which is the little-endian double `0.5`.

The helper then adds its signed stack argument:

```text
final_damage = base + helper_addend
```

All three discovered call instructions pass literal zero. The argument's
mechanical identity is therefore a signed damage addend; no active caller gives
it a nonzero semantic role.

The arithmetic is signed `i32`. It is not an unsigned HP delta and must not be
clamped before normal receiver entry.

### 5.3 Ephemeral bullet

With a non-null selected weapon, `0x0070D735..0x0070D78D` calls
`BulletClassAllocate @ 0x0046B050` using:

- selected projectile at `WeaponType+0xA0`;
- dying object as target;
- dying object as source/firer;
- signed `final_damage`;
- selected warhead at `WeaponType+0xAC`;
- literal zero;
- selected projectile/weapon byte at `+0x12F`.

Fresh `BulletClass::Init @ 0x004664C0` verification establishes the fields used
by the subsequent detonation:

| Bullet field | Value |
|---|---|
| `+0x10C` | target = dying object |
| `+0x128` | selected warhead |
| `+0x6C` | signed final damage |
| `+0xAC` | selected projectile |
| `+0xB0` | source/firer = dying object |
| `+0x150` | `0x100` |

The helper then:

1. attaches the selected weapon to the bullet;
2. calls the bullet's `Unlimbo` virtual;
3. gets the dying object's exact coordinate through vslot `+0x48`;
4. calls `WarheadTypeClass::Detonate @ 0x004690B0` synchronously;
5. destroys the temporary bullet through vslot `+0x08`;
6. returns `final_damage`.

If allocation fails, detonation is skipped and `final_damage` is still returned.
The known callers do not consume that return value.

The temporary bullet is not left for a later projectile or Logic scheduler pass.

## 6. Detonation and Area-Dispatcher Contract

### 6.1 Exact area arguments

At `0x00469A56..0x00469A83`, bullet detonation computes:

```text
dispatched_damage = (Bullet+0x150 * Bullet+0x6C) >> 8
```

Because the helper-initialized bullet has `+0x150 == 0x100`, the death helper's
signed damage is unchanged.

The call to `Apply_area_damage @ 0x00489280` receives:

| Argument | Death-weapon value |
|---|---|
| coordinate / `this` | exact dying-object coordinate passed into detonation |
| signed damage / `EDX` | helper `final_damage` |
| source object | dying object from `Bullet+0xB0` |
| warhead | selected weapon's warhead from `Bullet+0x128` |
| affect-resource flag | true |
| source house | `source_object+0x21C` at detonation, or null |

The source house is therefore the dying object's **current owner at the nested
detonation**, not the attacker of the killing hit and not a separately frozen
house value.

### 6.2 Collection before mutation

Fresh `Apply_area_damage` decompile and assembly prove two separate phases:

1. enumerate selected map-cell occupant lists and append heap records containing
   target pointer plus measured distance;
2. walk those records in their collected order and invoke each eligible target's
   vtable slot `+0x16C`.

The collection phase filters, among other conditions:

- target object is active;
- target health is greater than zero;
- target is not limboed;
- target is not otherwise excluded by the native area/air/bridge/source rules;
- measured distance is within the native radius.

The already-zero dying source therefore does not receive its own blast through
the ordinary target loop even when it was encountered during collection.

### 6.3 Per-target receiver arguments

At `0x00489A97..0x00489AB6`, every eligible record receives a fresh stack-local
copy of the original signed area damage:

```text
target.ReceiveDamage(
    &fresh_signed_damage,
    collected_distance,
    warhead,
    source_object,
    false,                 // ignore_defenses
    false,                 // arg6
    source_house
)
```

The receiver is responsible for Verses, armor, defenses, positive damage,
negative damage/healing behavior, fatal classification, side effects, and its
concrete class postlude. `Apply_area_damage` does not preconvert the operation to
an unsigned HP subtraction.

If the dispatched damage is exactly zero, `Apply_area_damage` returns before
target collection. A negative nonzero damage value is not rejected at entry and
remains signed into the target receiver.

### 6.4 Recursive ordering

The receiver call is synchronous. If the first outer target dies:

```text
outer area target 1 ReceiveDamage
  -> target 1 fatal Techno path
  -> target 1 death helper
  -> nested area collection
  -> all nested target receiver work, including deeper deaths
  -> nested helper returns
  -> target 1 fatal receiver resumes and completes
outer area dispatcher resumes
outer area target 2 ReceiveDamage
```

There is no native death-blast queue and no end-of-tick recursive closure.
Recursive depth is ordinary call-stack depth. No native artificial maximum depth
was found.

An empty eligible-target list simply performs no receiver calls, releases the
temporary records, returns through detonation, and destroys the ephemeral bullet.

## 7. Fatal Lifecycle Order

Fresh Techno receiver assembly `0x00702603..0x00702684` proves this local order
after the death-helper gate succeeds:

1. process/purge the passenger list and its callbacks;
2. call the death helper with zero;
3. complete every outer and nested area receiver before the helper returns;
4. examine the attached bomb at `Techno+0x38`;
5. detonate that bomb if present;
6. continue the concrete fatal wrapper/postlude.

The dying object must remain addressable, typed, owned, and coordinate-bearing
through the complete nested death detonation. Premature `UnInit`, store removal,
owner erasure, or coordinate erasure changes source provenance and behavior.

The Fly route similarly calls the helper before its following crash sound and
object `UnInit`.

## 8. Current Rust Classification

### 8.1 Parsed state

Current Rust parses:

- type `Explodes`;
- type `DeathWeapon`;
- veteran and elite ability sets, including `Explodes`;
- weapon `Suicide`.

It does not currently expose a death-weapon damage modifier on `ObjectType`, and
the scanned rules surface has no parsed `[CombatDamage] DeathWeapon` authority
used by `death_weapon_aoe`.

The required inputs partly exist, but the current death path does not consume
them with the native contract.

### 8.2 Producer selection drift

`death_weapon_aoe` currently:

1. returns an explicit `DeathWeapon` whenever the field exists;
2. otherwise uses `Primary` only when `Explodes` is true;
3. returns the weapon's raw `Damage` and warhead.

This differs from native behavior:

- explicit `DeathWeapon` is selection state, not a reachability gate;
- veteran/elite `Explodes` and current-weapon `Suicide` gates are absent;
- the native virtual fallback is not proved equivalent to static `Primary`;
- the Rules default weapon is absent;
- `DeathWeaponDamageModifier` is absent;
- the half-Strength default formula is absent;
- `Math__ftol` ordering is absent.

### 8.3 Coordinate and provenance drift

`handle_entity_deaths` records cell coordinates, sub-cell coordinates, height,
damage, warhead, and owner. The damage query then calls `apply_aoe_damage` using
only `rx` and `ry`.

`apply_aoe_damage` explicitly treats the impact as cell center
`(128,128)` for distance. Native death detonation uses the dying object's exact
vslot `+0x48` coordinate.

The Rust AoE call carries the owner string but not the dying source object. It
therefore cannot reproduce:

- normal source/self filtering;
- target receiver source-object arguments;
- the dying object's current owner lookup at the exact nested call;
- downstream source-dependent effects.

### 8.4 Width and receiver drift

`apply_aoe_damage` returns `Vec<(u64, u16)>`. It precomputes falloff and Verses,
clamps the result to `0..=u16::MAX`, applies a prone modifier, and drops zero
records.

`handle_entity_deaths` then executes:

```text
target.health.current =
    target.health.current.saturating_sub(aoe_dmg)
```

That Rust write is not the native death operation. Native code passes one signed
base-damage copy plus distance, warhead, source, flags, and house into each
target's complete receiver.

The Rust shortcut bypasses or reorders receiver-owned:

- signed damage/healing;
- armor and Verses writeback rules;
- defenses and immunity gates;
- attacker/source bookkeeping;
- retaliation and fear semantics;
- fatal result classification;
- concrete wrapper side effects;
- nested death-helper reachability;
- receiver-local RNG.

### 8.5 Batch and recursion drift

Rust first gathers death blasts while processing a fixed `dead_entities` slice.
It applies those blasts only after the per-dead-object loop has classified
lifecycle outcomes.

Consequences:

- all listed deaths are processed before the first death blast;
- passenger/lifecycle effects from multiple dying objects can precede blasts
  which native code executes earlier;
- a target reduced to zero by the direct death-AoE write is not appended to the
  same `dead_entities` slice;
- the same-call recursive death chain is absent;
- attached-bomb and nested-blast order cannot match native call-stack order.

This is not an internal-only representation difference. It changes same-tick
state visibility, side effects, and potentially RNG consumption.

## 9. Shadow-First Implementation Contract

### 9.1 Safe operation boundary

Introduce the next shadow operation at the **death special-producer invocation**,
not at the legacy `saturating_sub`.

The shadow input must be able to represent:

| Field | Required semantics |
|---|---|
| operation sequence | deterministic position inside the fatal receiver |
| source stable ID | dying object, retained through the nested call |
| source owner | read from the dying object at producer execution |
| exact coordinate | full game-space coordinate, not cell center |
| gate reason | type Explodes, veteran ability, elite ability, current-weapon Suicide, or Fly crash |
| selected path | explicit, virtual fallback, or Rules default |
| selected weapon | exact resolved weapon identity |
| projectile and warhead | exact selected references |
| modifier | native float bits/value used by `Math__ftol` |
| signed base/final damage | `i32`, including zero or negative values |
| helper addend | `i32`; zero for every known active call |
| resource flag | true |
| receiver flags | false, false |

The first version may be observation-only. It must not create lifecycle effects,
consume RNG, mutate HP, or enqueue authoritative objects.

### 9.2 Comparison classification

Until the synchronous nested receiver adapter exists, compare:

- native-intent producer trace;
- legacy Rust death-AoE trace;
- ordered target IDs;
- exact coordinate;
- selected weapon path;
- signed input damage;
- source ID and owner;
- terminal legacy HP results where available.

But classify per-target HP outcomes as **UNCOMPARABLE**, not `Match`, when the
legacy path bypasses the normal receiver. Equal HP in one fixture does not prove
operation parity.

The existing ordinary Object-vitality shadow may be reused only when each death
blast target actually enters the same receiver transaction boundary as ordinary
damage.

### 9.3 Incremental migration order

1. Parse and preserve exact death state:
   `DeathWeaponDamageModifier` and the Rules default death weapon.
2. Add a pure native-intent selector/gate shadow:
   gate reason, selected path, weapon, and signed `Math__ftol` result.
3. Add exact source and coordinate provenance to the shadow record.
4. Add an ordered area-dispatch observation which preserves native record order
   without applying damage.
5. Route each observed target through a shadow normal-receiver transaction with a
   fresh signed damage copy.
6. Add synchronous recursive shadow nesting and ordered diagnostics.
7. Reconcile passenger cleanup, attached-bomb, fatal wrapper, and `UnInit`
   boundaries around the nested producer.
8. Perform one coordinated authority flip for the producer plus receiver entry.
9. Remove the batched direct HP shortcut only after parity evidence supports the
   flip.

### 9.4 Authority-flip blockers

Do not make the death producer authoritative while any of these remain:

- explicit pointer incorrectly acts as a gate;
- veterancy or current-weapon Suicide gate is missing;
- virtual fallback identity/behavior is unresolved in Rust;
- Rules fallback or half-Strength formula is missing;
- float conversion order differs from `Math__ftol`;
- exact source object cannot survive the nested call;
- damage coordinate is reduced to a cell center;
- area target order differs;
- damage is converted to `u16` before receiver entry;
- receiver work is batched rather than synchronous;
- recursive deaths are queued or deferred;
- passenger/helper/bomb/UnInit order differs;
- receiver-local RNG cannot commit in native nested order.

## 10. Validation Requirements

### 10.1 Static and deterministic checks

The implementation phase should add focused checks for:

1. explicit `DeathWeapon` without any gate does not invoke the helper;
2. `Explodes=yes`, no explicit/virtual weapon, uses Rules default weapon and
   `ftol(Strength * 0.5)`;
3. `.1`, `.5`, and `.01` modifiers are converted in native operation order;
4. veteran/elite `Explodes` gates work independently of type `Explodes`;
5. current weapon `Suicide` gates the helper but does not itself choose the death
   weapon;
6. exact sub-cell source coordinate reaches the area dispatcher;
7. dying object/current owner reaches every target receiver;
8. target 1 recursive death completes before outer target 2;
9. passenger cleanup precedes the blast and attached bomb follows it;
10. zero signed damage causes no target dispatch;
11. negative nonzero damage remains signed into receivers;
12. empty target sets and allocation failure do not leave scheduled bullets;
13. Fly zero-HP crash invokes the helper before crash `UnInit`;
14. dormant Tunnel code is not enabled as standard-YR gameplay.

Rust-vs-Rust tests are regression checks only. They do not certify gamemd parity.

### 10.2 Runtime certification still required

Final authority needs instrumented retail traces which demonstrate:

- gate reason and selected path;
- exact signed damage before each receiver;
- target collection order;
- nested receiver/death-helper order;
- source object and source house;
- passenger, attached-bomb, and `UnInit` ordering;
- RNG position before and after recursive chains;
- final state/replay agreement.

## 11. Open-Question Ledger

| ID | Status | Question | Resolution |
|---|---|---|---|
| OQ01 | RESOLVED | What are all helper call sites? | Techno, Fly, dormant Tunnel |
| OQ02 | RESOLVED | Which routes are active standard YR? | Techno and Fly; Tunnel is dormant TS legacy |
| OQ03 | RESOLVED | What gates the Techno route? | type/veterancy Explodes or current-weapon Suicide |
| OQ04 | RESOLVED | Does `DeathWeapon` itself gate? | no |
| OQ05 | RESOLVED | What is selection order? | explicit, virtual fallback, Rules default |
| OQ06 | RESOLVED | What are the damage formulas? | modifier formula or half-Strength default, then signed addend |
| OQ07 | RESOLVED | What does the helper argument do? | signed addend; all known callers pass zero |
| OQ08 | RESOLVED | What happens on null weapon/allocation failure? | null weapon returns zero; allocation failure skips detonation |
| OQ09 | RESOLVED | What coordinate is used? | dying object's exact vslot `+0x48` coordinate |
| OQ10 | RESOLVED | What is source provenance? | dying object and its current owner |
| OQ11 | RESOLVED | How does area damage reach HP authority? | ordered synchronous normal receivers with fresh signed storage |
| OQ12 | RESOLVED | Can death weapons recurse? | yes, ordinary nested call-stack recursion |
| OQ13 | RESOLVED | What is local lifecycle order? | passengers, helper/nested chain, attached bomb, fatal postlude |
| OQ14 | RESOLVED | Are zero/negative values unsigned-clamped? | zero exits area dispatch; negative nonzero remains signed |
| OQ15 | RESOLVED | Is the Rust direct subtraction a separate native writer? | no; it collapses producer plus receiver |
| OQ16 | RESOLVED | Where should per-operation shadow begin? | death special-producer invocation |
| OQ17 | RESOLVED | Is the temporary bullet durable/scheduled? | no; synchronous ephemeral lifetime |
| OQ18 | RESOLVED | What happens with an empty target list? | no receiver calls; normal synchronous cleanup/return |
| OQ19 | DEFERRED — focused binary follow-up | What exact semantic name belongs to vslot `+0x3F4`? | preserve its verified returned-weapon contract without guessing |
| OQ20 | DEFERRED — runtime certification | Does retail reproduce every static order in instrumented fixtures? | requires Oracle/debugger capture |

No `[OPEN]` question remains in this bounded slice.

## 12. Adversarial Review

1. **Could equal final HP make the Rust direct subtraction acceptable?** No.
   Receiver side effects, fatal recursion, ordering, and RNG remain different even
   when one target's sampled HP happens to match.
2. **Could the explicit death weapon imply the gate indirectly?** No. The Techno
   gate does not read `+0xD18`; stock aircraft also show that a separate route can
   invoke the helper without type `Explodes`.
3. **Could the temporary bullet make the effect asynchronous?** No. The same helper
   initializes, detonates, and destroys it before returning.
4. **Could source house safely be frozen from the killing attacker?** No. Detonation
   dereferences the dying source object's current `+0x21C`.
5. **Could a post-loop recursive queue be equivalent?** No. Native target 1's full
   nested chain completes before outer target 2, including receiver effects and RNG.
6. **Could the current `u16` AoE result be a harmless storage choice?** No. Native
   receiver input is signed `i32`; negative nonzero values remain live.
7. **Could sub-cell position be ignored for stock default death blasts?** No proof
   supports that. Native distance collection consumes the exact coordinate, so a
   one-lepton boundary difference is DRIFT.

## 13. Zero-Add Pass and Cold Spot-Checks

The zero-add pass searched:

- all direct helper calls;
- all `TechnoType+0xD18` instruction references;
- parser/constructor/load consumers for the three death fields;
- current Rust `death_weapon`, `Explodes`, `Suicide`, and direct HP-write surfaces;
- stock `rulesmd.ini` death variants.

No fourth active-YR helper entry, durable death-bullet scheduler owner, or distinct
native direct death-AoE HP writer was found.

Two cold spot-checks were repeated after drafting:

1. `0x00702572..0x00702684` still resolves to gate evaluation, passenger purge,
   helper call, then attached-bomb detonation;
2. `0x00489A91..0x00489AD0` still resolves to ordered target eligibility, fresh
   signed damage storage, synchronous vslot `+0x16C`, and only then the next record.

No factual addition was required after the cold checks.

## 14. Coverage Ledger

| Required slice | Status | Primary proof |
|---|---|---|
| active helper entry points | VERIFIED | caller and exact CALL-instruction search |
| active/dormant classification | VERIFIED with prior RTTI carry-forward for Tunnel | fresh bodies plus July 13 COL proof |
| Techno reachability gates | VERIFIED | `0x00702572..0x00702603` |
| parser fields/defaults | VERIFIED | ReadINI, constructor, Rules parser |
| weapon selection | VERIFIED | `0x0070D698..0x0070D724` |
| signed formulas/addend | VERIFIED | helper assembly and constant memory |
| ephemeral bullet fields/lifetime | VERIFIED | allocator, Init, helper |
| exact area provenance | VERIFIED | detonation `0x00469A56..0x00469A83` |
| target collection before mutation | VERIFIED | Apply-area decompile/assembly |
| receiver ABI/order | VERIFIED | `0x00489A97..0x00489AB6` |
| recursive call-stack order | VERIFIED statically | synchronous receiver/helper bodies |
| passenger/helper/bomb order | VERIFIED | Techno fatal assembly |
| current Rust writer classification | VERIFIED by direct source read | cited feature-worktree surfaces |
| safe shadow boundary | DERIVED from verified ownership | Sections 8–9 |
| exact semantic name of vslot `+0x3F4` | DEFERRED | name is not required to preserve return contract |
| retail runtime parity | NOT CERTIFIED | later Oracle task |

## 15. Handoff

The next implementation artifact should be a **shadow-only death special-producer
transaction**, not another HP writer wrapper.

Its first success criterion is diagnostic completeness: for each fatal producer,
record why it fired, which selection path won, exact signed damage, exact source
and owner, exact coordinate, ordered target observations, and nested operation
structure. It should mark legacy HP comparison uncomparable where the legacy path
bypasses the normal receiver.

The later authority flip must be coordinated across producer selection, ordered
area dispatch, normal receiver entry, recursion, and fatal lifecycle order. Flipping
only `src/sim/combat/mod.rs:1088` would preserve the central parity error.
