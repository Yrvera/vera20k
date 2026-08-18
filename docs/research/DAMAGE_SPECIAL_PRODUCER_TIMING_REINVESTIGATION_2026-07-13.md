# Damage Special-Producer Timing Reinvestigation

**Date:** 2026-07-13  
**Plan unit:** damage authoritative cutover Task 3C  
**Scope:** death weapon, radiation, and lightning only  
**Program:** active Ghidra program `/gamemd.exe`, x86 32-bit  
**Investigation mode:** coverage map with fresh verification of prior partial work  
**Output restriction:** this report only; no Rust, plan, index, or other research-document edits

## 1. Verdict

**COMPLETE for the bounded static Task 3C producer contract.** The three mechanisms do not share one scheduler or one provenance lifetime:

| Producer | Native damage route | Native timing owner | Provenance lifetime | Task 3C verdict |
|---|---|---|---|---|
| Death weapon | temporary `BulletClass` detonation, then `Apply_area_damage @ 0x00489280` | synchronous inside the lethal receiver/crash path | dying object and selected weapon survive only through the nested call; the temporary bullet is destroyed before the caller resumes | VERIFIED, active YR; one TunnelLocomotion caller is dormant TS legacy |
| Radiation impact | ordinary bullet detonation, then `Apply_area_damage` for the impact | synchronous weapon impact | ordinary bullet provenance | VERIFIED, active YR |
| Radiation periodic HP | **direct** target vtable `+0x16C` receiver; no `Apply_area_damage` | each `FootClass::AI` in live Logic-vector order, after the reverse RadSite pass | no attacker/house/weapon/impact-warhead provenance is retained; site/cell state only | VERIFIED, active YR |
| Lightning | `LightningStorm__GroundStrike @ 0x0053A300`, then `Apply_area_damage` | global storm driver; damage waits until a tracked bolt animation passes half its frame count | mutable global owner pointer plus global Rules reads at strike time; no source object | VERIFIED, active YR |

The plan's radiation contingency is resolved negatively: **native periodic radiation does not retain source object, source house, impact warhead, or weapon**. Rust therefore must not invent persistent attribution fields to mimic native periodic damage. The actual Rust gap is timing/order and receiver semantics, not missing native provenance.

This is static proof, not parity certification. No retail runtime/Oracle trace was run because Task 3C prohibited game/debugger/input work.

## 2. Evidence Preflight and Boundaries

### 2.1 Sources read before live verification

- `docs/research/DAMAGE_AREA_DISPATCH_REINVESTIGATION_2026-07-13.md`
- `docs/research/combat/systems/suicide_weapons.md`
- `docs/research/combat/systems/radiation.md`
- `docs/research/RADIATION_EMP_GHIDRA_REPORT.md`
- `docs/research/LIGHTNING_STORM_SUPERWEAPON_GHIDRA_REPORT.md`
- `docs/research/LIGHTNING_STORM_RNG_CLASSIFICATION_GHIDRA_REPORT.md`
- Task 3C in `docs/plans/2026-07-13-damage-authoritative-cutover-plan.md:761-839`
- current Rust surfaces named in Sections 4.8, 5.9, and 6.8

The research-index brief for the special-producer query and anchor `0x00489280` was used only for navigation. Every load-bearing claim below comes from direct document/INI/code reads or fresh active-binary evidence.

### 2.2 Receiver argument convention reused from Task 3A

Task 3A proves that target vtable `+0x16C` receives, in semantic order:

```text
ReceiveDamage(
    i32* incoming_damage,
    i32 distance_leptons,
    WarheadTypeClass* warhead,
    ObjectClass* source_object,
    bool ignore_defenses,
    bool arg6,
    HouseClass* source_house
)
```

It also proves that `Apply_area_damage` calls this receiver synchronously with fresh damage storage per collected target. Evidence: `DAMAGE_AREA_DISPATCH_REINVESTIGATION_2026-07-13.md` Sections 3 and 9; live assembly `0x00489A97..0x00489AB6` cited there.

### 2.3 Classification rules

- **VERIFIED:** direct active-binary body/assembly plus active-YR reachability.
- **DRIFT:** current Rust differs in mechanism, ordering, argument, width, RNG, or state ownership.
- **UNCHECKED:** not proved in this bounded static pass.
- **DORMANT-TS:** code is in the binary, but the standard-YR route is not active.

## 3. Shared Global Tick Position

`LogicClassPerTickUpdateLiveVector @ 0x0055AFB0` establishes the load-bearing order:

```text
... global systems ...
LightningStorm__Process                    0x0055B5C8
for RadSite count-1 down to 0: vslot+0x5C 0x0055B5CD..0x0055B5E8
... light/EMP systems ...
for Logic object 0 up: vslot+0x5C          0x0055B608..0x0055B619
```

Evidence: live `decompile_function(0x0055AFB0)` and `disassemble_bytes(0x0055B5BE..0x0055B619)`. The Logic loop reloads its live count after every call; the RadSite loop snapshots the starting count and walks in reverse.

Consequences:

1. lightning strike damage finishes before any RadSite update or ordinary live-object AI that frame;
2. existing RadSites decrement/decay before any Foot reads cell radiation that frame;
3. a RadSite created later by a bullet in the live-object pass has missed that frame's RadSite pass;
4. on a `RadApplicationDelay` frame, only Foot objects later than that detonation in live Logic order can observe the newly deposited field in the same pass; earlier Foot objects wait for a later eligible frame.

## 4. Death Weapon

### 4.1 Active-YR reachability

`FUN_0070D690 @ 0x0070D690` is the death-weapon helper. Fresh xrefs and caller bodies classify its three routes:

| Caller | Reachability | Gate into helper | Evidence |
|---|---|---|---|
| `TechnoClass__ReceiveDamage`, call `0x0070266D` | active standard YR | lethal path and (`Type+0xD15 Explodes`, veteran/elite `EXPLODES` ability, or current weapon `WeaponType+0x144 Suicide`) | live decompile plus `0x007025D0..0x007026DF` assembly |
| `FlyLocomotionClass__Process`, call `0x004CD809` | active standard YR aircraft crash path | object health equals exactly zero; helper argument is zero | live decompile plus `0x004CD7A0..0x004CD840` assembly |
| function `0x00746100`, call `0x007461EF` | **DORMANT-TS** in standard YR | TunnelLocomotion death path | vtable slot bytes at `0x007F5C4C -> 0x00746100`; Complete Object Locator `0x0080CB48`; TypeDescriptor `.?AVTunnelLocomotionClass@@`; body assembly |

The TunnelLocomotion classification follows the project-wide verified caution that subterranean locomotion is dormant TS legacy in standard YR. It must not be implemented as an ordinary YR death route.

### 4.2 Weapon selection and damage

Fresh decompile and assembly of `0x0070D690..0x0070D789` prove this selection order:

1. use `TechnoTypeClass+0xD18 DeathWeapon` when non-null;
2. otherwise call object virtual `+0x3F4`; if the returned weapon-structure pointer and its `WeaponType*` are non-null, use that weapon;
3. otherwise use `RulesClass+0xFDC`, the default death weapon;
4. if the final weapon is null, return without a detonation.

For explicit/virtual-fallback weapons:

```text
damage = ftol((i32)WeaponType+0xA4 Damage * (float)TechnoType+0xD1C DeathWeaponDamageModifier)
```

For the Rules default:

```text
damage = ftol((i32)TechnoType+0xA0 Strength * 0.5)
```

The double at `0x007E1738` is raw `00 00 00 00 00 00 E0 3F` (`0.5`). Parser assembly `0x007122C0..0x00712370` binds `DeathWeapon` to `+0xD18`, `DeathWeaponDamageModifier` to `+0xD1C`, and the preceding `Explodes` key to byte `+0xD15`. Rules parsing `0x0066C540..0x0066C5D0` binds the default pointer at `Rules+0xFDC`.

An explicit `DeathWeapon=` is therefore **not itself a reachability gate**. The lethal path must still enter the helper through Explodes/ability/Suicide (or the separate aircraft crash caller).

### 4.3 Temporary bullet and exact dispatcher provenance

The helper calls `BulletClassAllocate @ 0x0046B050` with:

- selected weapon projectile (`WeaponType+0xA0`);
- dying object as bullet target;
- dying object as bullet owner/firer;
- computed signed i32 damage;
- selected weapon warhead (`WeaponType+0xAC`);
- literal zero and projectile byte `+0x12F` for the remaining allocator arguments.

`BulletClass__Init @ 0x004664C0` proves the resulting fields used here: `Bullet+0xB0` source/firer, `+0x6C` damage, `+0x128` warhead, `+0x10C` target, and `+0x150=0x100`. The helper writes the selected `WeaponType*` to `Bullet+0x130`, unlimbos through vslot `+0xD4`, obtains the dying object's coordinate through vslot `+0x48`, calls bullet detonation `0x004690B0` synchronously, then destroys the temporary bullet through vslot `+0x08`. Allocation failure skips the entire detonation.

At `0x00469A40..0x00469AA4`, bullet detonation calls `Apply_area_damage`. Because `Bullet+0x150` is `0x100`, its dispatched base damage is unchanged:

```text
(Bullet+0x150 * Bullet+0x6C) >> 8 == helper damage
```

#### Death-weapon argument-provenance/tick table

| Field / event | Exact native value | Lifetime / owner | Evidence |
|---|---|---|---|
| impact coordinate | dying object's vslot `+0x48` coordinate, copied into detonation | synchronous temporary bullet call | `0x0070D690..0x0070D789`; `0x00469A40..0x00469AA4` |
| area base damage | explicit/fallback formula above, or half type Strength for Rules default | `Bullet+0x6C`, through immediate detonation only | helper + Bullet init assembly |
| area warhead | selected weapon `+0xAC` | `Bullet+0x128` | helper + detonation assembly |
| area source object | dying object | `Bullet+0xB0`; object is still in the lethal call stack | helper + detonation assembly |
| area source house | `source_object+0x21C` at detonation, else null | looked up synchronously, not a durable record | `0x00469A40..0x00469AA4` |
| `affect_resource` | true | call literal | same assembly |
| receiver distance | captured by Task 3A area collector | fixed-record lifetime only | Task 3A report |
| receiver flags | `ignore_defenses=false`, `arg6=false` | per receiver call | Task 3A report |
| scheduler position | nested inside lethal receiver/crash caller; no live-vector bullet insertion | helper does unlimbo/detonate/destroy before returning | helper assembly |
| RNG owner | helper itself consumes no RNG; ordinary bullet/warhead detonation and nested receivers own any downstream Scenario RNG draws | synchronous call stack | helper body plus detonation callees |

### 4.4 Ordering and recursion

In `TechnoClass__ReceiveDamage`, passenger purge precedes the death-weapon helper. The helper and every nested area receiver finish before the lethal receiver continues to its attached-bomb check and possible `BombClass::Detonate`. Evidence: `0x007025D0..0x007026DF`.

A death-weapon area receiver may lethally enter another object's receiver, which may call the same helper before the outer area dispatcher resumes. Recursion is therefore ordinary nested call-stack recursion, ordered by Task 3A's area record sequence. No separate death queue, projectile scheduler, or end-of-tick batch mediates it. Allocation failure or the victim's death-helper gate terminates a branch. No native global maximum depth was found; claiming one would be invented.

### 4.5 Negative facts and prior-doc corrections

- The source object is **not** the original attacker. It is the exploding/dying object.
- The source house is that object's current owner at detonation, not a house frozen from the killing blow.
- `DeathWeapon=` alone does not make every death explode.
- Explicit death damage is not current HP and not raw weapon damage when `DeathWeaponDamageModifier` differs from `1.0`.
- The default route uses half type Strength.
- The temporary bullet is not left in the live Logic scheduler.
- “Suicide means an unconditional second explosion” is not proved. The Suicide flag is a helper gate; which nested targets are hit still depends on the selected death weapon and normal area filters, including source/self filtering.

### 4.6 Current Rust drift

Current Rust is not an equivalent scheduler translation:

- `src/sim/combat/mod.rs:785-800` chooses explicit `DeathWeapon` without the native Explodes/ability/Suicide reachability gate, assumes the fallback is primary weapon, omits the Rules default, and omits both damage formulas/modifier branches.
- `:862-889` collects death AoE after batched HP application rather than executing it inside the lethal receiver.
- `:1036-1119` applies collected death AoE after the death loop, directly mutates HP, and does not reproduce temporary-bullet detonation or nested receiver recursion.
- the dying source object is not carried into the area call; only the owner string reaches `apply_aoe_damage` at `:1061-1075`.
- targets killed by the direct death-AoE HP subtraction at `:1076-1097` are not appended to the same local `dead_entities` closure, so the native same-stack chain is absent.

**Verdict: DRIFT.** This is not a `ProjectileImpactDamageCall` scheduling case; it needs a synchronous special-producer adapter capable of nested receiver entry with the dying object as source.

## 5. Radiation

### 5.1 Initial impact creation route

`WarheadTypeClass__Detonate @ 0x004690B0` checks `Bullet+0x130` selected weapon and requires `WeaponType+0x158 RadLevel > 0`. It converts the impact Cartesian X/Y to the containing cell with the native signed `+0xFF then >>8` rule, reads spread as:

```text
spread_cells = ftol((float)(WeaponType+0xAC WarheadType*)+0x124 CellSpread)
```

Evidence: `disassemble_bytes(0x00469130..0x0046920B)`, especially `0x0046917A..0x0046919D` and setter calls `0x004691D5/1E1/1ED/1F4/1FC`.

If the center cell has no `Cell+0xF8 RadSite*`, it allocates 0x74 bytes, constructs and globally registers a RadSite, then calls in order:

```text
SetCell(center) -> SetSpread(spread) -> SetRadLevel(weapon.RadLevel)
-> Activate() -> center_cell.SetRadSite(site)
```

If the center already has a site, it calls `AddRadLevel(incoming RadLevel)`. Adjacent centers remain separate sites; their contributions add in shared `Cell+0xF0` doubles.

This site work occurs long before the ordinary bullet's `Apply_area_damage` call at `0x00469A83`. The initial weapon impact still uses the normal bullet source/house/warhead contract. The persistent periodic path below is separate.

### 5.2 Persistent native state and provenance absence

`RadSiteClass::GetSize @ 0x0065B3A0` returns `0x74`. Fresh constructor/setter/AI/save/load reads establish:

| Offset | Width | State | Initialization / lifetime owner |
|---|---:|---|---|
| `+0x24` | 4 | `LightSourceClass*` | zero in constructor; allocated/updated by `Activate`; destroyed by RadSite destructor; save/load pointer-swizzled |
| `+0x28/+0x2C/+0x30` | 3 x i32 | level-decay timer start/aux/duration | constructor writes start=current frame and duration=0 but does not initialize the aux dword; `Activate` writes all three and takes duration from `Rules+0x1810` |
| `+0x34/+0x38/+0x3C` | 3 x i32 | light timer start/aux/duration | analogous, with duration from `Rules+0x1814` |
| `+0x40/+0x42` | 2 x i16 | center cell X/Y | `SetCell` copies packed two-short cell coordinate |
| `+0x44` | i32 | spread cells | `SetSpread` |
| `+0x48` | i32 | radius leptons | `spread * 256 + 128` |
| `+0x4C` | i32 | activation RadLevel | `SetRadLevel`; on merge becomes effective old level plus incoming |
| `+0x50` | i32 | level step count | `total_duration / Rules.RadLevelDelay` in `Activate` |
| `+0x54` | i32 | light intensity | `ftol(level * Rules.RadLightFactor)` |
| `+0x58/+0x5C/+0x60` | 3 x i32 | light tint | Rules color/tint-derived values |
| `+0x64/+0x68` | 2 x i32 | light steps/decrement | activation-derived |
| `+0x6C` | i32 | total duration | `level * Rules.RadDurationMultiple` |
| `+0x70` | i32 | remaining duration | initialized to total; decremented before timers each RadSite AI |

There is no field in the 0x74-byte object for source object, source house, weapon, impact warhead, impact damage, or impact coordinate beyond center cell/spread. The periodic consumer also never asks a site for any such value. This is a positive layout-plus-consumer proof that impact attribution is intentionally discarded.

Evidence: live `decompile_function(0x0065B1E0)`, `0x0065B4C0`, `0x0065B4D0`, `0x0065B4F0`, `0x0065B530`, `0x0065B580`, `0x0065B800`; assembly `0x0065B530..0x0065B577`.

### 5.3 Merge formula

`AddRadLevel @ 0x0065B530` first computes:

```text
effective_old = ftol(old_level * remaining_duration / total_duration)
```

It removes the site's outstanding old cell contribution, then sets:

```text
new_level = effective_old + incoming_level
total_duration = remaining_duration = Rules.RadDurationMultiple * new_level
```

and reactivates/re-spreads. It does **not** clamp `new_level` to `RadLevelMax`; the cap is applied only by the later cell-level damage accessor. Evidence: FPU and integer sequence `0x0065B534..0x0065B570`.

### 5.4 Site AI, expiry, and cell ownership

The global driver walks RadSites in reverse array order. Each `RadSiteClass__AI @ 0x0065B800`:

1. decrements `RemainingDuration`;
2. when its activation-anchored level timer expires, calls `0x0065BD00` to subtract `falloff / level_steps` from every affected cell, then resets the timer;
3. updates the light when its light timer expires;
4. destroys the site when remaining duration is less than one.

The misleading local label `ApplyRadDamage @ 0x0065BD00` does **not** damage Techno HP; it subtracts site contribution from `Cell+0xF0`. `CellClass::DecreaseRadLevel @ 0x00487D00` clamps negative results to exact double zero.

`RadSiteClass__Destructor @ 0x0065B2F0` clears the center cell's `+0xF8` site pointer, detaches, destroys the light, stably removes the pointer from the global RadSite vector, and then runs the Abstract destructor. It does not perform an extra final sweep of all affected cell levels. Residual sub-damaging floating residue can therefore survive site deletion.

### 5.5 Exact periodic receiver call

The formerly open consumer is `FootClass__AI @ 0x004DA530`. After parent `TechnoClass__AI_Update`, the periodic block requires:

- `Object+0x90 IsAlive != 0`;
- signed `g_CurrentFrameCounter % Rules+0x1808 RadApplicationDelay == 0`;
- `TechnoType+0xD37 ImmuneToRadiation == false`;
- virtual `+0x54 ObjectClass::IsHighFlying == false`;
- `Object+0x81 InLimbo == false`;
- capped/truncated cell level greater than zero.

The coordinate conversion uses the Foot's exact vslot `+0x48` Cartesian coordinate, with native signed lepton-to-cell conversion separately for X/Y. `Cell accessor 0x00487CB0` returns:

```text
level_i32 = ftol(min(Cell+0xF0 double, Rules+0x180C RadLevelMax i32))
damage_i32 = ftol(level_i32 * Rules+0x1818 RadLevelFactor double)
```

#### Periodic-radiation argument-provenance/tick table

| Receiver field / event | Exact native value | Lifetime / owner | Evidence |
|---|---|---|---|
| target | current Foot object | live Logic-vector entry | `FootClass__AI` |
| `incoming_damage` | stack i32 from the two-stage formula above | one direct receiver call | `0x004DA5EB..0x004DA610` |
| `distance_leptons` | `0` | call literal | `0x004DA625` |
| warhead | `RulesClass+0x1834 RadSiteWarhead` read at receiver time | global Rules pointer; not site state | `0x004DA614..0x004DA620` |
| source object | null | call literal | `0x004DA61F` |
| `ignore_defenses` | false | call literal | `0x004DA61E` |
| `arg6` | **true** | call literal | `0x004DA61C` |
| source house | null | call literal | `0x004DA60F` |
| scheduler position | current Foot's AI in forward live Logic order, after reverse RadSite pass | no queued event | `0x0055B5CD..0x0055B619` |
| post-call behavior | if receiver cleared `IsAlive`, Foot AI returns immediately | same call stack | `0x004DA62F..0x004DA635` |
| RNG | no RNG in the periodic block or RadSite create/merge/decay methods | none locally; nested receiver death effects may draw | live bodies listed above |

The exact push sequence is `0, 1, 0, 0, warhead, 0, &damage` right-to-left at `0x004DA60F..0x004DA629`, which maps to source house null, `arg6=true`, ignore false, source null, warhead, distance zero, and damage pointer under the Task 3A receiver ABI.

### 5.6 Initial-impact RNG versus persistent RNG

`WarheadTypeClass__Detonate` can make up to two conditional `RandomRanged` calls from warhead range pairs at `0x004690CE..0x0046912B` **before** RadSite creation. These are ordinary impact/warhead RNG draws, not RadSite-owned state. Site construction, activation, merge, decay, periodic HP dispatch, and expiry contain no RNG calls. The periodic receiver may of course enter downstream death behavior that consumes Scenario RNG; that belongs to the receiver/death mechanism.

### 5.7 Save, load, and hash ownership

Native persistence is raw-object based:

- `RadSiteClass__Save @ 0x0065B450` delegates to `AbstractClass__Save @ 0x00410320`, which writes the identity pointer and then `GetSize()==0x74` bytes from the object.
- `RadSiteClass__Load @ 0x0065B3D0` delegates to `AbstractClass__Load @ 0x00410380`, restores vtables, writes both timer starts to the current frame, writes both timer durations to zero (so the next AI expires and reloads the current Rules durations), leaves the two serialized aux dwords untouched, and registers `+0x24 LightSource*` for pointer swizzling.
- center cell radiation doubles and `Cell+0xF8` are owned by the map/cell state, not duplicated as an array inside RadSite.
- RadSite vtable slot `+0x34` points to `0x0065B3B0`, whose complete body only calls `AbstractClass__ComputeCRC @ 0x00410410`. Therefore RadSite-specific fields `+0x24..+0x70` are **not** directly included by RadSite's CRC virtual; only Abstract base CRC inputs are added there. Cell state may participate through its own map/cell checksum path, which was outside this bounded producer pass.

Evidence: live decompile of all six functions; vtable read `0x007F0844 -> 0x0065B3B0`; assembly `0x0065B3B0..0x0065B3BA`.

### 5.8 Negative facts and corrected prior claims

- Periodic radiation does not call `Apply_area_damage`.
- `0x0065BD00` decays cell radiation; it does not dispatch HP damage.
- Periodic damage is not attributed to the firing unit or firing house.
- The site's original warhead is not retained; periodic damage always reads the current global `RadSiteWarhead` rule.
- Buildings do not run `FootClass::AI`; standard building radiation HP is therefore absent through this path. Foot-derived low/non-high-flying objects remain subject to the exact gates above.
- `RadLevelMax` caps damage input, not stored site or cell intensity.
- Site decay occurs before Foot periodic reads in the native frame driver, not after the object loop.

### 5.9 Current Rust drift

Rust correctly omits attacker/house/warhead provenance from `RadSite` and `RadDetonation`; that omission matches native periodic attribution. Material drift remains:

- `src/sim/combat/mod.rs:1773-1847` folds all new detonations, scans every victim as a batch, pre-applies Verses, and queues ordinary `damage_events`; native invokes each Foot's concrete receiver directly at that Foot's live-order position with `arg6=true`.
- Rust gives every eligible victim visibility of every detonation folded before the batch. Native same-frame visibility depends on whether that Foot's Logic position is after the bullet detonation.
- Rust runs `RadiationState::tick_decay` after combat at `src/sim/world/mod.rs:2356-2363`; native runs the reverse RadSite pass before the forward Foot/Logic pass.
- Rust iterates sites by ascending center key (`radiation.rs:318-340`); native iterates reverse global insertion order.
- Rust maps each center to a `BTreeMap` site and hashes every cell double and site field (`world_hash.rs:450-468`). Native has raw map cell state, a global RadSite vector, and RadSite CRC excludes its specific fields.
- Rust's serde layout (`radiation.rs:37-74`) is not native 0x74-byte object persistence and does not model native light pointer/timer aux/tint/light fields.
- Rust comments at `world/mod.rs:2356-2358` state the native RadSite driver is after the object loop; fresh live assembly proves the opposite.

**Verdict: DRIFT.** Do not add native-nonexistent provenance to fix it. Fix the site-before-Foot order, same-pass visibility, direct-receiver call shape, and native iteration/state ownership.

## 6. Lightning Storm

### 6.1 Active-YR reachability and durable globals

`LightningStorm__Process @ 0x0053A6C0` is called unconditionally from the global driver at `0x0055B5C8`; its active-state globals gate actual cloud/strike work. `LightningStorm__Start @ 0x00539EB0` stores the target cell to `0x00A9F9CC` and the `HouseClass*` owner to `0x00A9FACC` **before** checking whether a storm is already active or deferred.

This means owner/target are mutable global provenance, not per-bolt fields. A later Start call can replace them before an already-created cloud reaches its damage frame. GroundStrike reads owner at damage time.

The owner pointer and target are serialized by `FUN_00539890` and restored by `FUN_00539AE0`; the load path registers owner `0x00A9FACC` for pointer swizzling. The load/save pair also persists the active tracked cloud/bolt animation vectors. At storm cleanup, Process clears the owner global to null (`0x0053A8E4`). Evidence: live Start/Process decompiles, xrefs to `0x00A9FACC`, save assembly `0x00539981..0x005399CE`, and load decompile `0x00539AE0`.

### 6.2 Bolt creation and delayed strike

While active:

- when `CurrentFrame % Rules+0x17A0 LightningHitDelay == 0`, Process calls `CreateCloudBolt` at the center;
- when `CurrentFrame % Rules+0x17A4 LightningScatterDelay == 0`, it makes up to exactly three scatter attempts;
- each attempt draws signed inclusive X then Y offsets from `[-(LightningCellSpread >> 1), +(LightningCellSpread >> 1)]`, rejects out-of-bounds cells, and rejects cells whose Manhattan distance is less than `LightningSeparation` from **any active cloud animation**;
- the first accepted candidate creates one tracked cloud/bolt animation; all three failures create none.

`LightningStorm__CreateCloudBolt @ 0x0053A140` rejects the special duplicate coordinate before its raw RNG draw, then consumes one raw Scenario RNG `Next`, selects `WeatherConClouds[Next % count]`, constructs an animation, and appends that animation pointer to both tracked vectors when capacity permits.

Process walks the damage-bearing animation vector in reverse. Only when:

```text
animation_current_frame > animation_type_total_frames / 2
```

does it read the animation coordinate, call `LightningStorm__GroundStrike`, and remove the tracking entry. Damage is therefore delayed by asset frame data; it is not applied at cloud creation.

Evidence: live `decompile_function(0x0053A6C0)`, `decompile_function(0x0053A140)`, and their complete assembly; Rules INI keys at `rulesmd.ini:130-137,532-534`.

### 6.3 GroundStrike coordinate and exact area call

GroundStrike resolves the cell and constructs Cartesian world-lepton impact coordinates:

```text
x = cell_x * 256 + 128
y = cell_y * 256 + 128
z = signed Cell+0x11B level * global level-height
  + (Cell+0x140 & 0x100 ? global bridge-height : 0)
```

It first consumes one raw Scenario RNG draw to choose `WeatherConBolts[Next % count]`, constructs/tracks the visual bolt, and then performs the exact duplicate-coordinate early return. Thus the duplicate case consumes the bolt-animation draw and creates the visual but performs no sound, explosion, damage, or later scorch draws.

For a non-duplicate strike with `LightningSounds` count greater than zero, it consumes another raw `Next` and selects `LightningSounds[Next % count]` even when the count is one. The lightning-special `Warhead__SelectExplosionAnim @ 0x0048A4F0` branch is deterministic and returns `Rules+0x2F4`; it does not add a random AnimList draw for this warhead.

#### Lightning argument-provenance/tick table

| Dispatcher field / event | Exact native value | Lifetime / owner | Evidence |
|---|---|---|---|
| impact coordinate | cell-center Cartesian coordinate with signed level and conditional bridge height | GroundStrike stack local | `0x0053A3E7..0x0053A445` |
| area base damage | `RulesClass+0x1798 LightningDamage`, read at strike time | global Rules | `0x0053A4A8..0x0053A4BF`, reused in EBP at call |
| area warhead | `RulesClass+0x17B4 LightningWarhead`, read at strike time | global Rules | `0x0053A5C0..0x0053A5C8` |
| area source object | null | call literal | `0x0053A5C9` |
| area source house | global `0x00A9FACC` current storm owner, read at strike time | saved/swizzled mutable global; cleared at cleanup | `0x0053A5B7..0x0053A5BD` |
| `affect_resource` | true | call literal | `0x0053A5BE` |
| receiver distance | Task 3A captured signed lepton distance | fixed-record lifetime | Task 3A |
| receiver flags | `ignore_defenses=false`, `arg6=false` | area dispatcher literals | Task 3A |
| scheduler position | reverse tracked-animation pass in global `LightningStorm__Process`, before RadSites and live Logic objects | animation remains authoritative until `frame > total/2` | Process + global-driver assembly |
| recursion | area receivers execute synchronously; nested death weapons finish before GroundStrike resumes | ordinary call stack | Task 3A + Section 4 |
| post-damage work | rechecks building/object/height; conditionally draws scorch count `RandomRanged(2,4)`, then one debris-animation range draw per scorch | only after area call | `0x0053A5D5..0x0053A693` |

The exact `Apply_area_damage` setup at `0x0053A5B1..0x0053A5D0` is: push current owner house, push true, push current lightning warhead, push null source, put current lightning damage in EDX, and pass the stack CoordStruct in ECX.

### 6.4 RNG sequence per accepted damage-bearing strike

For clarity, all listed draws use the shared Scenario RNG, not a lightning-private stream:

1. cloud creation: one raw `Next % WeatherConCloudsCount` after duplicate rejection;
2. delayed GroundStrike: one raw `Next % WeatherConBoltsCount` before duplicate rejection;
3. non-duplicate and sounds-present: one raw `Next % LightningSoundsCount`;
4. synchronous explosion/area damage and all nested receiver draws;
5. only if the post-damage terrain/object predicate requests scorch: `RandomRanged(2,4)` then one `RandomRanged(0, ScorchesCount-1)` per spawned scorch.

Scatter scheduling separately consumes X then Y `RandomRanged` for each of at most three attempts. A rejected attempt still consumes both. There is no fallback fourth candidate.

### 6.5 Negative facts

- Lightning damage is not applied when the cloud/bolt is first created.
- A tracked strike does not freeze owner, damage, or warhead; GroundStrike reads global owner and Rules at the later damage frame.
- Source object is always null.
- The owner is not stored per animation.
- The RNG stream is not lightning-specific.
- Separation is against every active cloud animation, not only the last accepted coordinate.
- `LightningCellSpread` is halved with signed right shift before inclusive offset draws.
- The special duplicate early return is after the visual bolt draw/creation but before sound and damage.

### 6.6 Current Rust drift

`src/sim/superweapon/lightning_storm.rs` differs materially:

- `:17-21` hardcodes three bolt animation names and ten retries instead of consuming Rules/retail animation vectors and native three-attempt behavior.
- `:57-74` stores a separate queued request when a storm is active. Native Start overwrites the mutable target/owner globals before its active/deferment branch; it does not freeze owner per queued Rust record in this way.
- `:142-173` uses per-state countdown timers, while native uses global frame modulo gates.
- `:187-207` draws over full `[-spread,+spread]`, compares only with one last coordinate, tries ten times, then consumes two extra fallback draws and always returns a cell. Native uses half-spread, all active clouds, exactly three attempts, bounds rejection, and may spawn nothing.
- `:210-269` applies area HP immediately in `spawn_bolt`; native waits past half of the selected animation's asset-derived frame count.
- `:212-216` uses `superweapon_rng()` and hardcoded bolt visuals; native uses the shared Scenario RNG and two distinct Rules lists (`WeatherConClouds`, then `WeatherConBolts`).
- `:243-269` directly mutates HP through the already-DRIFT coarse AoE helper, without the exact Cartesian Task 3A collector/receiver transaction, null source object, explicit source-house call field, or resource flag.
- `:271-317` emits generic effects/sound after direct HP with different RNG and post-damage terrain-change ordering.

Rust state hashing at `src/sim/world/world_hash.rs:471-510` also hashes its active and queued state shape, which is not the native mutable-global/tracked-animation ownership model.

**Verdict: DRIFT.** Lightning requires a global storm/animation scheduler adapter, not immediate `spawn_bolt` damage and not the normal projectile-impact scheduler.

## 7. Cross-Producer Contract for the Damage Cutover

### 7.1 These are not one G2 scheduler

The plan's normal `ProjectileImpactDamageCall` cannot be applied mechanically to all three special producers:

| Producer | Required Rust-native owner preserving gamemd semantics |
|---|---|
| death weapon | lethal receiver/death-transition service that can create an ephemeral bullet-detonation transaction and synchronously recurse before the outer receiver resumes |
| radiation periodic | live Foot-object AI window, after the reverse RadSite pass; direct receiver call with null attribution and `arg6=true` |
| lightning | global storm tracked-animation service before RadSites/Logic; delayed strike reads mutable global owner/current Rules, then synchronous area dispatch |

Rust-native structure is still appropriate, but the commit/call positions, argument reads, RNG draws, and same-pass visibility must be gamemd-native.

### 7.2 Required exact call facts

| Producer | source object | source house | warhead | damage | area? | receiver flags |
|---|---|---|---|---|---|---|
| death weapon | dying object | dying object's current owner | selected death weapon warhead | verified death formula | yes | area dispatcher: false/false |
| radiation initial impact | ordinary bullet source | ordinary bullet source house | impact bullet warhead | ordinary bullet damage | yes | area dispatcher: false/false |
| radiation periodic | null | null | current `Rules.RadSiteWarhead` | `ftol(ftol(min(cell, max))*factor)` | **no; direct** | false/**true** |
| lightning | null | current global storm owner | current `Rules.LightningWarhead` | current `Rules.LightningDamage` | yes | area dispatcher: false/false |

### 7.3 Do-not-generalize rules

- Do not queue death weapon until an end-of-tick death phase.
- Do not make periodic radiation an attributed AoE event.
- Do not move RadSite decay after Foot damage.
- Do not damage on lightning cloud creation.
- Do not give these mechanisms private RNG streams.
- Do not freeze lightning owner/damage/warhead per cloud when native reads mutable globals later.

## 8. Contradictions Reconciled

| Earlier statement | Fresh verdict | Correction |
|---|---|---|
| radiation damage site was unknown / likely `TechnoClass::AI` | wrong level of specificity | exact site is `FootClass::AI @ 0x004DA530`, direct vslot `+0x16C` call |
| periodic radiation likely uses normal source provenance | wrong | source object and source house are both null; site has no provenance fields |
| `RadSiteClass::ApplyRadDamage` damages units | wrong label inference | `0x0065BD00` only decreases `Cell+0xF0`; Foot AI damages units |
| RadSite driver runs after object loop | wrong | global assembly puts reverse RadSite pass before forward live Logic pass |
| DeathWeapon fires whenever a type defines it | wrong | lethal helper reachability still needs Explodes/ability/Suicide, except separate crash caller |
| death source is original killer | wrong | source is the dying object; house is its current owner |
| Demo/Suicide always means two explosions | unsupported | Suicide is a helper gate; nested hit outcome remains conditional |
| lightning damage occurs at bolt spawn | wrong | damage occurs only after tracked animation frame exceeds half of asset total |
| lightning owner is frozen per bolt | wrong | GroundStrike reads mutable global owner at damage time |

## 9. Coverage Ledger

| Required Task 3C item | Status | Primary proof |
|---|---|---|
| death active reachability including TS filter | VERIFIED | three helper callers; Tunnel vtable COL/type proof |
| death exact area arguments | VERIFIED | helper, Bullet init, detonation assembly |
| death scheduler/recursion/RNG | VERIFIED | synchronous helper and lethal receiver order |
| radiation impact-to-site creation | VERIFIED | detonation assembly `0x00469130..0x0046920B` |
| radiation persistent fields/init/expiry | VERIFIED | constructor/setters/Activate/AI/destructor |
| radiation periodic source/house/warhead/damage | VERIFIED | Foot AI assembly `0x004DA530..0x004DA635` |
| radiation save/load/hash ownership | VERIFIED for RadSite virtual/object; broader Cell checksum path out of scope | Abstract Save/Load, RadSite CRC slot |
| radiation scheduler and same-pass visibility | VERIFIED | global driver plus Foot AI |
| lightning active route and delayed strike | VERIFIED | Start/CreateCloud/Process/GroundStrike |
| lightning exact area arguments | VERIFIED | `0x0053A5B1..0x0053A5D0` |
| lightning RNG and post-damage sequence | VERIFIED | CreateCloud/GroundStrike/Process bodies |
| lightning provenance lifetime/storage | VERIFIED | mutable globals plus save/load/swizzle |
| current Rust drift | VERIFIED by direct reads | cited source lines |
| retail runtime trace / pixel certification | NOT RUN, prohibited | later Oracle task |

## 10. Open-Question Log

- `[RESOLVED] OQ-3C-01 — Does a death weapon enter the normal projectile scheduler?` No; it constructs, detonates, and destroys an ephemeral bullet synchronously.
- `[RESOLVED] OQ-3C-02 — What is the death source?` The dying object; its current owner becomes source house.
- `[RESOLVED] OQ-3C-03 — Can death weapons recurse?` Yes, by nested synchronous receivers in area-record order.
- `[RESOLVED] OQ-3C-04 — Does defining DeathWeapon alone reach the helper?` No; the lethal route has Explodes/ability/Suicide gates.
- `[RESOLVED] OQ-3C-05 — Does periodic radiation retain impact provenance?` No; layout and consumer both prove absence.
- `[RESOLVED] OQ-3C-06 — Does periodic radiation use Apply_area_damage?` No; it calls the Foot receiver directly.
- `[RESOLVED] OQ-3C-07 — What are its two receiver flags?` `ignore_defenses=false`, `arg6=true`.
- `[RESOLVED] OQ-3C-08 — Is RadLevelMax a storage clamp?` No; it is a damage-read cap.
- `[RESOLVED] OQ-3C-09 — Does RadSite update before or after Foot AI?` Before, in reverse site order.
- `[RESOLVED] OQ-3C-10 — When does lightning damage?` After the tracked animation's current frame is strictly greater than half its type frame count.
- `[RESOLVED] OQ-3C-11 — What provenance is frozen per lightning bolt?` Coordinate through the animation; owner/damage/warhead are not frozen and are read later.
- `[RESOLVED] OQ-3C-12 — Which RNG stream?` Shared Scenario RNG for all producer-local draws verified here.
- `[DEFERRED / broader checksum scope] OQ-3C-13 — Which exact CellClass fields enter the whole-game sync CRC and in what order?` RadSite virtual ownership is proved; a full Cell/map checksum audit is outside special-producer timing.
- `[DEFERRED / runtime certification] OQ-3C-14 — Does an instrumented retail trace reproduce every static sequence?` Requires the separate Oracle task; no runtime work was authorized here.

## 11. Adversarial Review

1. **Could the RadSite secretly recover an attacker through its light or center cell?** No consumer follows either path for HP. Foot AI reads only cell intensity and global Rules, then pushes null source/null house.
2. **Could the radiation `PUSH 1` be source house truthiness rather than `arg6`?** No. Task 3A's seven-argument receiver ABI and right-to-left push order map the first push to source house and the second to `arg6`; Foot pushes `0` then `1`.
3. **Could death weapon be delayed because the helper unlimbos a Bullet?** No. The same helper immediately calls detonation and destroys the bullet before returning; no later live-vector AI owns the damage.
4. **Could a lightning cloud freeze owner in its AnimClass?** GroundStrike is called with only the animation coordinate and loads `0x00A9FACC` immediately before area dispatch. No owner is read from the animation.
5. **Could Rust's different site/storm containers be internal-only?** Not proved. Native order changes same-frame radiation visibility and RNG/damage timing; the differences are DRIFT under the project's burden of proof.

## 12. Zero-Add Pass and Cold Spot-Checks

The final zero-add pass found no fourth producer inside the bounded Task 3C scope. Ordinary projectile impact remains Task 3B; area-dispatch enumeration remains Task 3A/3S; BombClass and nuke-specific orchestration are adjacent mechanisms, not a license to expand this report.

Two cold spot-checks were repeated after drafting:

1. radiation receiver assembly `0x004DA60F..0x004DA629`: still maps to null house, `arg6=true`, ignore=false, null source, current `Rules.RadSiteWarhead`, zero distance, damage pointer;
2. lightning area setup `0x0053A5B1..0x0053A5D0`: still maps to mutable global owner, `affect_resource=true`, current Rules warhead, null source, EDX current Rules damage, ECX exact stack coordinate.

No factual addition was needed after those spot-checks.

## 13. Status and Handoff

**Task 3C static research status: COMPLETE.**

The reconciliation owner can consume the three tables in Sections 4.3, 5.5, and 6.3. The implementation-planning consequence is precise: preserve three different authority windows rather than forcing all damage producers through one late batch. Runtime parity certification remains explicitly open for the Oracle phase.
