# Label Proof Samples - Struct Fields / Bitflags, Second Wave

Date: 2026-05-31
Scope: high-risk struct fields and bitflags after the first `CellClass` and core-struct samples.
Method: read-only Ghidra decompile/caller checks plus existing research-doc cross-checks. Local Ghidra names remain navigation hints only; the verdicts below are from field reads/writes, caller context, and offset arithmetic.

## Bottom Line

The sampled high-value labels are mostly correct, but something does need to be done before sending broad orchestrator work:

1. Build a central struct-field validation ledger so agents stop re-learning the same `int*` indexing traps.
2. Fix stale docs/names where old labels still say `IsOnMap` for `ObjectClass+0x98`.
3. Keep field offsets and vtable offsets in separate tables. Example: `TechnoClass field +0x294` is `AirstrikeClass*`; `TechnoClass vtable +0x294` is a `CanSelfHeal` virtual slot. Both can be true only if the table kind is explicit.
4. Do not mass-rename Ghidra symbols. Rename or document only fields proven from live code.

Two doc corrections were applied during this pass:

- `TECHNOCLASS_EXPANDED_STRUCT_LAYOUT.md`: `ObjectClass+0x98` changed from `IsOnMap` to `LogicVectorMembership`.
- `ABSTRACT_OBJECT_SUBSTRATE_SERVICE_DESIGN.md`: `ObjectClass+0x8D` clarified as fall/height-settle active flag, not an in-map/limbo marker.

## Samples

| # | Field / bit | Verified identity | Evidence sample | Verdict / action |
|---|-------------|-------------------|-----------------|------------------|
| 1 | `ObjectClass+0x8C` | `OnBridge` byte | `ObjectClass__Constructor @ 005F3900` initializes byte rendered as `param_1+0x23`; `ObjectClass__Unlimbo @ 005F5940` sets it when `CellClass+0x140 & 0x100`; `ObjectClass__GetHeight @ 005F5F40` reads `this->OnBridge` and subtracts bridge height. | Correct. Keep explicit note that decompiler `param_1[0x23]` is byte `+0x8C`. |
| 2 | `ObjectClass+0x8D` | fall/height-settle active flag | `ObjectClass__DropIn @ 005F4160` sets `+0x8D=1`; `ObjectClass__Unlimbo @ 005F5940` also sets it; `ObjectClass__AI @ 005F3E70` returns early if clear, otherwise updates `Location_Z`, applies fall rate, and clears `+0x8D` after landing. | Corrected wording needed. Do not call this an in-map marker. |
| 3 | `ObjectClass+0x90` | `IsAlive` byte | Constructor sets byte at `param_1+0x24` to `1`; `ObjectClass__IsDead @ 005F6690` returns `*(this+0x90)==0`; `ObjectClass__UnInit @ 005F65F0` clears it; `ReceiveDamage @ 005F5390` checks it repeatedly after trigger callbacks. | Correct. Separate from active-vector membership. |
| 4 | `ObjectClass+0x98` | active `LogicClass` vector membership guard | `FUN_0055BAA0 @ 0055BAA0` returns if byte is already set, otherwise inserts into the active vector and sets `+0x98=1`; `FUN_0055BAE0 @ 0055BAE0` removes and clears it; callers include `ObjectClass__Reveal` and `ObjectClass__Conceal`. | Stale label found. Patched expanded Techno layout. |
| 5 | `ObjectClass+0x9C/+0xA0/+0xA4` | world coordinate triple | `ObjectClass__GetCoords @ 005F65A0` copies these three dwords; `ObjectClass__Set_Raw_Coords @ 005F6940` writes the same three dwords. | Correct. Do not use older `coords +0x4C` claims. |
| 6 | `ObjectClass+0x6C` | current health | `ObjectClass__GetHealthRatio @ 005F5C60` reads `this->Health` and divides by `Type+0xA0`; `ObjectClass__ReceiveDamage @ 005F5390` reads/writes `this->Health`. | Correct. `Type+0xA0` is max health / Strength. |
| 7 | `ObjectClass+0x83` | selection byte | `TechnoClass__ChangeOwner @ 007014A0` reads byte `this+0x83` before auto-deselecting when the losing owner is the local player. Prior selection reports own the writer set. | Correct as sampled; not the same as limbo, alive, or active-vector state. |
| 8 | `TechnoClass+0x21C` | owner `HouseClass*` | `TechnoClass__ChangeOwner @ 007014A0` reads old owner as `param_1[0x87]`, writes new owner to `param_1[0x87]`; `FactoryClass__StartProduction @ 004C9C70` reads produced object `+0x21C` into `FactoryClass::Owner`. | Correct. High priority field. |
| 9 | `TechnoClass+0x294` | `AirstrikeClass*` manager pointer | `TechnoClass__Init_Managers @ 006F3F40` writes `param_1[0xA5]` only after creating `AirstrikeClass` when `Type+0x61C > 0`; existing aircraft radio report verifies assembly `[ESI+0x294]`. | Correct. Flag as decompiler-index trap: `param_1[0xA5]` is byte `+0x294`, not byte `+0xA5`. |
| 10 | `UnitClass+0x6C4` | `UnitTypeClass*` | `UnitClass__Facing_Update @ 00736990` reads `param_1[0x1B1]` for unit-type flags such as `+0xCA1`, `+0xD21`, `+0xE13`; prior constructor sample verifies the same type pointer. | Correct. Another `int*` indexing trap: `0x1B1*4 = +0x6C4`. |
| 11 | `FactoryClass+0x24` | production progress / `Production_Value` | `FactoryClass__GetProgress @ 004CA120` returns `this->Production_Value`; `FactoryClass__IsComplete @ 004CA130` checks it against `0x36`; `StartProduction @ 004C9C70` resets it to `0`. | Correct. It reuses StageClass value storage. |
| 12 | `FactoryClass+0x28` | production changed flag / `IsDifferent` | `FactoryClass__HasChanged @ 004C9C60` reads the bool and clears it; `StartProduction @ 004C9C70` sets it true. Caller check shows `StripClass__AI @ 006A8B30` polls it. | Correct. Sidebar redraw depends on read-and-reset semantics. |
| 13 | `FactoryClass+0x44/+0x50` | queued-object vector items/count | `StartProduction @ 004C9C70` appends to `QueuedObjects_Items` and increments `QueuedObjects_Count`; `FactoryClass__Save @ 004CA3C0` writes count then item pointers. | Correct enough for field identity. Exact save/load semantics still need assembly if load compatibility becomes the target. |
| 14 | `FactoryClass+0x58` | current produced object pointer | `FactoryClass__GetObject @ 004CA160` returns `this->Object`; `StartProduction @ 004C9C70` writes the created object pointer and then uses it for owner/balance setup. | Correct. |
| 15 | `FactoryClass+0x6C` | factory owner `HouseClass*` | `StartProduction @ 004C9C70` copies `Object+0x21C` into `FactoryClass::Owner`; production timing docs also use `+0x6C` as owner. | Correct. |
| 16 | `HouseClass+0x30C` | spendable credit balance | `HouseClass__Add_Credits @ 004F9950` adds to `+0x30C`; `HouseClass__Spend_Money @ 004F9790` subtracts from it and drains storage if cash is insufficient. | Correct. |
| 17 | `HouseClass+0x54E8` | harvested-credit/stat accumulator, not spendable balance | `HouseClass__Add_Tiberium_Credits @ 004F9610` writes both `+0x54E8` and `+0x30C`; `TechnoClass__ChangeOwner @ 007014A0` updates new owner `+0x54E8` by type cost when ownership changes. | Correct but easy to misuse. Keep distinct from balance. |
| 18 | `RulesClass+0xF3C` | ore purifier bonus multiplier | Existing `UNIT_MISSION_DEPLOY_BUILDING_GHIDRA_REPORT.md` verifies purifier deposit formula reads `g_RulesClass_Instance+0x0F3C`; `ADD_TIBERIUM_CREDITS_PURIFIER_VIRTUAL_PURIFIERS_GHIDRA_REPORT.md` cross-checks the same field. | Correct from existing reports; not re-decompiled in this wave. |

## What Needs To Be Done

Create `docs/research/STRUCT_FIELD_LABEL_VALIDATION_LEDGER.md` or equivalent and make the orchestrator maintain it. Each row should include:

- struct/class name
- byte offset
- decompiler rendering, if misleading
- verified name
- type/width/signedness
- read sites
- write sites
- active-YR reachability
- stale names to avoid
- last verification date

Seed it from the three label-proof reports:

- `LABEL_PROOF_SAMPLE_CELLCLASS_0XDC.md`
- `LABEL_PROOF_SAMPLES_CELLCLASS_TOP10_20260531.md`
- `LABEL_PROOF_SAMPLES_CORE_STRUCT_FIELDS_20260531.md`
- this second-wave report

Immediate stale-label cleanup queue:

1. Search for `ObjectClass+0x98` / `0x98 | ... IsOnMap` and replace only when the row is the object active-vector byte. Do not global-replace `IsOnMap`, because some docs use it for different abstract flags.
2. Search for `ObjectClass+0x8D` claims that call it an in-map marker. The verified role is fall/height-settle active flag.
3. Add explicit warnings for `int*` index traps: `param_1[0x23] -> byte +0x8C`, `param_1[0xA5] -> byte +0x294`, `param_1[0x1B1] -> byte +0x6C4`, `param_1[0x148] -> byte +0x520`.
4. Split field-offset and vtable-offset tables anywhere they share a `+0xNNN` notation.
5. Do not send agents to mass-rename Ghidra. Send them to prove and record fields in small waves, then apply only verified doc/Ghidra naming corrections.

## Orchestrator Prompt Shape

Use this as the broad task:

```text
Audit struct-field and bitflag labels that can poison later decompilation reads.
Prioritize CellClass, ObjectClass, TechnoClass, BuildingClass, UnitClass,
HouseClass, RulesClass, MapClass, FactoryClass, and LogicClass.

For each sampled field, prove the byte offset from function bodies, not from
existing labels. Record reads, writes, type/width/signedness, active-YR caller
reachability, and decompiler indexing traps. Distinguish field offsets from
vtable offsets. Update docs only for verified facts. Do not implement Rust and
do not mass-rename symbols.
```

## Residual Risk

This is still a sample set, not a complete struct audit. `FactoryClass__Save/Load` decompilation has register-recovery artifacts, so save/load exactness should be checked at assembly level if it becomes load-bearing. `RulesClass+0xF3C` was accepted from existing reports in this wave. `ObjectClass+0x8D` has a verified fall/height-settle role, but a full writer/caller drain across all subclasses would still be useful before making it a final canonical ledger row.
