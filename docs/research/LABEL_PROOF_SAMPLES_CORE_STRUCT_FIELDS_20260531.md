# Label Proof Samples: Core Struct Fields And Bitflags

Date: 2026-05-31
Mode: cross-class label-validation sample set, read-only Ghidra evidence, no Rust implementation.

## Purpose

This note checks whether the highest-priority labeling target is real:

> Struct fields and bitflags are the most important Ghidra labels because wrong field names poison every
> later function read.

Verdict: yes. The sampled labels are mostly correct, but this should become a formal label-validation
ledger because the evidence is spread across many reports and some offsets are easy to misread from
decompiler output.

## Summary

| # | Class / substrate | Offset / field | Safe label | Verdict |
|---|---|---|---|---|
| 1 | `CellClass` | `+0x78` | `VisibleToHousesGapGenMask` | Correct |
| 2 | `MapClass` | `+0x13C` | `CellArrayBase` / `CellClass**[512*512]` | Correct |
| 3 | `ObjectClass` | `+0x81` | `InLimbo` | Correct |
| 4 | `ObjectClass` | `+0x98` | `LogicVectorMember` / active-list guard | Correct |
| 5 | `TechnoClass` | `+0x3D2` | `HasStealthAbility` | Correct |
| 6 | `UnitClass` | `+0x6C4` (`0x1B1*4`) | `UnitTypeClass*` | Correct |
| 7 | `BuildingClass` | `+0x520` (`0x148*4`) | `BuildingTypeClass*` | Correct |
| 8 | `HouseClass` | `+0x30C` | `Balance` / live spendable credits | Correct |
| 9 | `HouseClass` | `+0x184` | `AIDifficultyIndex` | Correct |
| 10 | `FactoryClass` | production timer/value fields | production state, not generic object state | Correct but needs struct ledger |
| 11 | `RulesClass` | `+0x33C` | `[General] WarpOut` anim type | Correct |
| 12 | `LogicClass` | `+0x04/+0x10` | active object vector ptr/count | Correct |

## Samples

### 1. `CellClass+0x78`: visibility/GapGen per-house mask

Safe label:

```text
CellClass+0x78 = VisibleToHousesGapGenMask
```

Evidence:

- `CellClass__IsVisibleToHouse @ 004870B0` reads `*(uint *)(cell + 0x78) & (1 << houseIndex)`.
- `FUN_00487110 @ 00487110` writes `*(uint *)(cell + 0x78) |= (1 << houseIndex)`.
- `CellClass__Constructor @ 0047BC50` initializes this dword independently from `+0xDC`.

Action:

Keep or rename to the safe label. Do not merge with `+0xDC`.

### 2. `MapClass+0x13C`: canonical cell-array base

Safe label:

```text
MapClass+0x13C = CellArrayBase
```

Evidence:

- `MapClass__Get_CellClass @ 005657A0` computes `index = y * 0x200 + x`.
- It reads `*(undefined **)(*(int *)(this + 0x13c) + index * 4)`.
- Out-of-bounds or null cells return dummy `DAT_00ABDC50` after storing the probed coord at
  `DAT_00ABDC74`.

Action:

Keep. This is one of the highest-value anchor labels in the project.

### 3. `ObjectClass+0x81`: InLimbo

Safe label:

```text
ObjectClass+0x81 = InLimbo
```

Evidence:

- `ObjectClass__Reveal @ 005F4EC0` only proceeds when `byte[this+0x81] != 0`, then clears it:

```c
*(undefined1 *)((int)this + 0x81) = 0;
```

- `ObjectClass__Conceal @ 005F4D30` early-outs when `byte[this+0x81] != 0`, and near the tail sets it:

```c
*(undefined1 *)((int)this + 0x81) = 1;
```

Action:

Keep. This is a lifecycle-critical label.

### 4. `ObjectClass+0x98`: LogicVectorMember / active-list guard

Safe label:

```text
ObjectClass+0x98 = LogicVectorMember
```

Evidence:

- `FUN_0055BAA0` checks `byte[obj+0x98]`; if clear, inserts into the dynamic vector and sets it to `1`.
- `FUN_0055BAE0` checks `byte[obj+0x98]`; if set, removes from the vector and clears it.
- `ObjectClass__Reveal` reaches `FUN_0055BAA0` in active live-object registration contexts.
- `ObjectClass__Conceal` reaches `FUN_0055BAE0` in active unregister contexts.

Action:

Keep. This label is structural: wrong naming here causes scheduler/lifecycle drift.

### 5. `TechnoClass+0x3D2`: HasStealthAbility

Safe label:

```text
TechnoClass+0x3D2 = HasStealthAbility
```

Evidence:

- `TechnoClass__HasStealthAbility @ 0070C5A0` returns `byte[this+0x3D2] != 0`.
- `UnitClass__Constructor @ 007353C0` copies this byte from its type:

```c
*(undefined1 *)(unit + 0x3d2) = *(undefined1 *)(unitType + 0xcd0);
```

Action:

Keep. If expanding this later, verify infantry/aircraft/building constructors also source the same inherited
Techno field correctly.

### 6. `UnitClass+0x6C4`: UnitTypeClass pointer

Safe label:

```text
UnitClass+0x6C4 = UnitTypeClass*
```

Evidence:

- `UnitClass__Constructor @ 007353C0` stores constructor `param_2` at `param_1[0x1B1]`.
- `0x1B1 * 4 = 0x6C4`.
- The constructor immediately uses `param_1[0x1B1]` as the unit type: it reads locomotor CLSID at
  `type+0x34C`, weapon/locomotor/art fields, and other type defaults.

Action:

Keep. This is a good example of why decompiler `int*` indices must be converted to byte offsets before
renaming.

### 7. `BuildingClass+0x520`: BuildingTypeClass pointer

Safe label:

```text
BuildingClass+0x520 = BuildingTypeClass*
```

Evidence:

- `BuildingClass__Constructor @ 0043B740` stores constructor `param_2` at `param_1[0x148]`.
- `0x148 * 4 = 0x520`.
- The same constructor reads type fields through `param_1[0x148]`, including timer/contact defaults.
- Other building routines, such as exit/deploy-style logic, repeatedly read `param_1[0x148]` as the
  building type pointer.

Action:

Keep. This is another decompiler-index trap: `param_1[0x148]` is not byte offset `+0x148`.

### 8. `HouseClass+0x30C`: Balance / live credits

Safe label:

```text
HouseClass+0x30C = Balance
```

Evidence:

- `HouseClass__Add_Credits @ 004F9950` does:

```c
*(int *)(house + 0x30c) += amount;
```

- `HouseClass__Spend_Money @ 004F9790` reads `+0x30C`, subtracts from it, and only then falls back to
  tiberium storage if the balance is insufficient.
- `HouseClass__Add_Tiberium_Credits @ 004F9610` writes `+0x30C` after its `ftol` credit calculation.

Action:

Keep. Do not confuse with score/stat counters such as `HouseClass+0x54E8`.

### 9. `HouseClass+0x184`: AI difficulty index

Safe label:

```text
HouseClass+0x184 = AIDifficultyIndex
```

Evidence:

- `HouseClass__SetDifficulty @ 004F6EC0` saves the old value, writes `param_2` directly to `house+0x184`,
  then uses `param_2` to index RulesClass difficulty tables.

Action:

Keep. The exact value-to-name mapping should stay tied to the verified lobby/rules evidence, not guessed
from UI strings.

### 10. `FactoryClass` production fields

Safe labels:

```text
FactoryClass::Object
FactoryClass::Owner
FactoryClass::Production_Timer_StartTime
FactoryClass::Production_Timer_Duration
FactoryClass::Production_Timer_TimeLeft
FactoryClass::Production_Value
FactoryClass::QueuedObjects_Count
```

Evidence:

- `FactoryClass__Constructor @ 004C98B0` initializes the timer/value/queue fields and inserts the factory
  into `g_FactoryClass_Array`.
- `FactoryClass__StartProduction @ 004C9C70` clears timer/value state, creates the produced object, stores
  `this->Object`, stores `this->Owner`, stores the production cost/balance, and queues extra requests when
  production is already occupied.
- `FactoryClass__GetProgress @ 004CA120` returns `this->Production_Value`.

Action:

Keep the current semantic names, but create a dedicated FactoryClass field ledger. This class is important
enough that relying on scattered decompiler struct names is risky.

### 11. `RulesClass+0x33C`: `[General] WarpOut`

Safe label:

```text
RulesClass+0x33C = WarpOutAnimType
```

Evidence:

- `RulesClass__ReadGeneral @ 0066D530` reads the `[General]` key `WarpOut`.
- On successful parse, it resolves an `AnimTypeClass` and writes the result to `rules+0x33C`.
- Teleport/warp reports already tie active consumers to this offset.

Action:

Keep. RulesClass labels should always include the INI key and destination type, not just a generic
`anim`.

### 12. `LogicClass+0x04/+0x10`: active object vector

Safe labels:

```text
LogicClass+0x04 = ActiveObjectVectorItems
LogicClass+0x10 = ActiveObjectVectorCount
```

Evidence:

- `LogicClassPerTickUpdateLiveVector @ 0055AFB0` iterates:

```c
if (0 < *(int *)(this + 0x10)) {
  do {
    (**(code **)(**(int **)(*(int *)(this + 4) + i * 4) + 0x5c))();
    i++;
  } while (i < *(int *)(this + 0x10));
}
```

- `FUN_0055BAA0` inserts an object into this vector and sets `ObjectClass+0x98`.
- `FUN_0055BAE0` removes an object from this vector and clears `ObjectClass+0x98`.

Action:

Keep. These labels are foundational for scheduler parity.

## Should We Do Something About This?

Yes, but the right action is not mass renaming.

Recommended action:

1. Create a formal `STRUCT_FIELD_LABEL_VALIDATION_LEDGER.md`.
2. Start with `CellClass/MapClass`, then `ObjectClass/TechnoClass`, then `HouseClass/RulesClass`,
   then `BuildingClass/FactoryClass`, then class-specific `Unit/Infantry/Aircraft`, then `LogicClass`.
3. For each field, require direct reader/writer evidence and byte-offset conversion when the decompiler
   shows `param[N]`.
4. Only rename `LIVE_VERIFIED` labels.
5. Use conservative suffixes for partial proof:
   - `_UNCHECKED_WRITERS`
   - `_UNCHECKED_ACTIVE`
   - `_TS_LEGACY_UNVERIFIED`
   - `_INFERRED_ROLE`

Why:

- The sampled labels are mostly correct, which is good.
- The bigger risk is that correctness is not centrally tracked, and future agents may trust stale names,
  comments, or decompiler index syntax.
- Known project evidence already shows this trap: `param_1[0xA5]` in one aircraft report was not byte
  offset `+0xA5`; it was byte offset `+0x294`.

## Next Best Batch

Run a dedicated validation wave for:

1. `ObjectClass` lifecycle bytes: `+0x80`, `+0x81`, `+0x8C`, `+0x8D`, `+0x90`, `+0x98`.
2. `TechnoClass` shared state: owner pointer/index, health, target, mission, facing, cloak state, warp state.
3. `BuildingClass` fields around `+0x520..+0x720`, especially foundation/exit/power/factory/superweapon
   connections.
4. `HouseClass` financial, power, factory, alliance, and AI fields.
5. `RulesClass` INI destination offsets that feed active gameplay formulas.
