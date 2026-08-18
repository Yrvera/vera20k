# Bridge Object OnBridge Extra Writers - Ghidra Research Report

**Address(es):** `0x0051A407`, `0x006FF0B0`, false-positive scan hits listed below
**Confidence:** High for hit classification; Medium for exact feature naming on non-object false positives
**Active in YR:** Conditional. `0x0051A407` is active in infantry enter flows; `0x006FF0B0` is active for `Inviso=yes` bullets fired at on-bridge targets. The other classified hits are not runtime `ObjectClass+0x8C`.

## 1. Overview

This follow-up classifies the extra `+0x8C` write/copy hits found while verifying `BRIDGE_OBJECT_ONBRIDGE_FIELD_GHIDRA_REPORT.md`. The parent report's normal movement timing remains intact: movement removes an object from the old cell using old `OnBridge`, updates `OnBridge`, then inserts into the new cell using new `OnBridge`.

The new finding is not another normal movement relayer. Most extra hits are unrelated structs whose own `+0x8C` byte is not `ObjectClass::OnBridge`. Two hits are real object-derived writes: one clears an infantry object before a successful enter/conceal path, and one propagates target bridge state to an invisible bullet.

## 2. Class Layout / Key Offsets

| Offset | Owner | Meaning | Evidence |
|--------|-------|---------|----------|
| `ObjectClass+0x8C` | Object-derived runtime objects | `OnBridge` bool | Parent report; constructor `0x005F3900`; consumers in CellClass add/remove |
| `ObjectClass+0x81` | Object-derived runtime objects | `InLimbo` bool | `OBJECTCLASS_GHIDRA_REPORT.md`, Conceal/Reveal section |
| `ObjectClass+0xAC` | Object-derived runtime objects | Type pointer in derived runtime classes | `BulletTypeClass` report says `BulletClass+0xAC` is `BulletTypeClass*` |
| `BulletTypeClass+0x29E` | Bullet type | `Inviso` bool | `BULLETTYPECLASS_GHIDRA_REPORT.md`; `rulesmd.ini` has many `Inviso=yes` projectiles |
| `CellClass+0xE4` | Cell | ground object list head | Parent bridge OnBridge report |
| `CellClass+0xE8` | Cell | bridge object list head | Parent bridge OnBridge report |

## 3. Core Logic

### 3.1 Extra Immediate Writes Scan

Scan pattern:

```text
c6 ?? 8c 00 00 00 ??
```

Extra hits not already explained by the parent report:

| Address | Classification | Finding |
|---------|----------------|---------|
| `0x0051A407` | True runtime object write | `InfantryClass::Mission_Enter` clears `this+0x8C` before a successful enter/conceal branch. |
| `0x006DD711` | False positive | Non-object string/data struct; clears a byte at its own `+0x8C` after copying/trimming text into `+0x6D`. |
| `0x006E3FEB` | False positive | Same string/data struct pattern; function starts at `0x006E3FB0`, string buffer at `+0x6D`, not ObjectClass. |
| `0x006FF0B0` | True runtime object write | `TechnoClass::Fire_At` sets `BulletClass+0x8C` on an invisible bullet when its target object is on a bridge. |

### 3.2 `0x0051A407` - Infantry Enter Clears OnBridge

Binary evidence from bytes around `0x0051A360`:

```asm
0051A3E6  mov  eax,[esi]
0051A3E8  push edi
0051A3E9  push 0x0f
0051A3ED  call dword ptr [eax+0x278]
0051A3F3  cmp  eax,0x1
0051A3F6  jne  0x0051A488
0051A3FC  push ebp
0051A3FF  call 0x0070C610
0051A404  push ebp
0051A407  mov  byte ptr [esi+0x8c],0
0051A40E  mov  dword ptr [esi+0xc4],ebp
0051A414  call 0x0070DE00
0051A41C  call 0x0070DDD0
0051A441  call dword ptr [edx+0xd4]
```

Details:

- `esi` is the entering infantry object in the `InfantryClass::Mission_Enter` body (`0x005196A0` per `ADDRESS_MAP.md` and multiple mission-enter reports).
- The branch is reached only after the preceding virtual call with argument `0x0F` returns `1`.
- `0x0070C610` is the one-liner `TechnoClass::SetGhostCell`/`SetWarpVisualState`, writing the argument to `this+0x218`.
- `0x0070DDD0` and `0x0070DE00` are tiny setters for `this+0x140` and `this+0x144` style counters; they are not bridge state writers.
- The later `vtable+0xD4` dispatch resolves to `ObjectClass::Conceal` in `OBJECTCLASS_GHIDRA_REPORT.md`.
- `ObjectClass::Conceal` calls `vtable+0x124(0)` / Mark-remove according to `OBJECTCLASS_GHIDRA_REPORT.md`.

Finding: this is a successful enter/conceal path that clears `OnBridge` before the object is concealed. It is not the normal drive/walk/ship boundary-crossing sequence. It may matter when infantry enter a building/transport while currently on a bridge, but that is a feature-specific enter/limbo case rather than the movement occupancy relayer.

Active in YR: Yes, conditional on the infantry enter mission path being reached. Stock YR uses infantry enter flows for garrison, grinder, Bio Reactor/Absorber, C4/engineer-style interactions, and transport/building entry depending on target.

### 3.3 `0x006FF0B0` - Inviso Bullet Copies Target OnBridge

Binary evidence from `TechnoClass::Fire_At` around `0x006FF08B`:

```asm
006FF08B  mov  edx,[ebx+0xac]        ; bullet type pointer
006FF091  mov  al,[edx+0x29e]        ; BulletTypeClass.Inviso
006FF097  test al,al
006FF099  je   0x006FF0B7
006FF09B  mov  eax,[esp+0x84]        ; target object pointer
006FF0A2  test eax,eax
006FF0A4  je   0x006FF0B7
006FF0A6  mov  cl,[eax+0x8c]         ; target.OnBridge
006FF0AC  test cl,cl
006FF0AE  je   0x006FF0B7
006FF0B0  mov  byte ptr [ebx+0x8c],1 ; bullet.OnBridge = true
```

Details:

- `ebx` is the newly-created `BulletClass` runtime object on this path. The dereference `ebx+0xAC -> +0x29E` matches the documented `BulletClass+0xAC = BulletTypeClass*` and `BulletTypeClass+0x29E = Inviso`.
- The write is one-way: it sets the bullet `OnBridge` byte to `1`; there is no paired clear at this site.
- The target pointer is null-checked before reading `target+0x8C`.
- The target must already have `OnBridge != 0`.
- This is not a CellClass add/remove path and does not call `CellClass::AddContent` or `CellClass::RemoveContent` in the immediate block.
- The result matters for bullet height/collision semantics because `ObjectClass::GetHeight` subtracts the bridge height when `OnBridge` is set, and bullet AI reports use GetHeight for ground/bridge collision checks.

Active in YR: Yes, conditional. `rulesmd.ini` has many `Inviso=yes` projectiles, including `Invisible`, `Invisible2`, `Invisible3`, `InvisibleVertical`, `InvisibleLow`, `InvisibleMedium`, `InvisibleHigh`, `InvisibleAll`, `PsychicControl`, `Psychic`, `QuadShell`, `FlakProj`, comet fragments, and Tesla projectiles.

### 3.4 False Positives

`0x006DD711`:

```asm
006DD6DB  lea  edi,[ebp+0x6d]
006DD700  push 0x1f
006DD702  push eax
006DD703  push edi
006DD704  call 0x007C91D0
006DD711  mov  byte ptr [ebp+0x8c],0
006DD718  repne scasb
```

Classification: not `ObjectClass`. The struct has a string buffer at `+0x6D`; the `+0x8C` byte is adjacent string/metadata storage, not bridge state.

`0x006E3FEB`:

```asm
006E3FB0  mov  eax,[esp+4]
006E3FB5  mov  esi,ecx
006E3FBA  lea  edi,[esi+0x6d]
006E3FDA  push 0x1f
006E3FDC  push eax
006E3FDD  push edi
006E3FDE  call 0x007C91D0
006E3FEB  mov  byte ptr [esi+0x8c],0
006E4008  ret  4
```

Classification: not `ObjectClass`, for the same reason as `0x006DD711`.

`0x0051FDB2`:

```asm
0051FDA0  je    0x0051FDE4
0051FDAB  test  eax,eax
0051FDAD  setne al
0051FDB2  mov   byte ptr [edi+0x8c],al
```

Classification: not runtime `ObjectClass`. This is a type/config parse-style function with surrounding string/lookup calls and later type fields such as `+0x421`/`+0x422`; its `+0x8C` byte is not object bridge state.

`0x00776EC9` / `0x00776F04`:

```asm
00776EB0  sub  esp,0x94
00776EC9  mov  byte ptr [esi+0x8c],bl
00776ECF  mov  byte ptr [esi+0x8d],bl
...
00776F04  mov  byte ptr [esi+0x8c],cl
00776F11  mov  byte ptr [esi+0x8d],cl
```

Classification: not runtime `ObjectClass`. The function initializes a stack-sized/config helper object, has paired state bytes at `+0x8C` and `+0x8D`, and copies a 0x7F-byte string to `+0x0C`.

## 4. INI Keys

| INI key | Owner | YR state | Relevance |
|---------|-------|----------|-----------|
| `Inviso` | Projectile / BulletType | Many stock `rulesmd.ini` projectile sections set `Inviso=yes` | Gates `0x006FF0B0` bullet `OnBridge` propagation. |
| `Image=none` | Projectile / BulletType | Common on `Inviso=yes` projectiles | Confirms invisible bullet family. |
| `AA`, `AG`, `SubjectToCliffs`, `SubjectToElevation`, `SubjectToWalls` | Projectile / BulletType | Varies by projectile | Affects whether the invisible projectile is used against bridge targets but does not gate the `OnBridge` copy once an inviso bullet exists. |
| `Occupier`, `Passengers`, `EnterTransportSound`, `EnterBioReactorSound` | Infantry/building/transport systems | Stock YR uses these flows | Context for `InfantryClass::Mission_Enter`; not direct `OnBridge` keys. |

## 5. Integration Points

Normal movement:

- Parent report remains authoritative for drive/walk/ship movement: remove with old `OnBridge`, update `OnBridge`, add with new `OnBridge`.
- The extra writers here do not add another normal movement occupancy path.

Infantry enter:

- `InfantryClass::Mission_Enter @ 0x005196A0` has a branch at `0x0051A407` that clears `OnBridge` before `ObjectClass::Conceal`.
- If porting infantry enter/garrison/transport/grinder later, this should be treated as its own enter/limbo timing question, not folded into generic cell movement.

Projectile fire:

- `TechnoClass::Fire_At @ 0x006FDD50` sets `BulletClass.OnBridge` for invisible bullets targeting an object already on a bridge.
- This impacts projectile height/collision/detonation parity, not cell list occupancy.

## 6. Current Rust Implementation Status

Rust movement occupancy status from the audit remains:

- `src/sim/occupancy.rs` has `OccupancyGrid::move_entity`, which removes the entity ID from all lists and inserts using one requested layer.
- `src/sim/movement/movement_step.rs` calls `move_entity` before bridge state resolution, so insertion can use the wrong layer on boundary ticks.
- The drive-track branch in `src/sim/movement/movement_tick.rs` resolves bridge state before calling `move_entity`, but still uses a single layer for the whole move rather than explicit old-layer/remove and new-layer/insert selectors.
- `src/sim/world/bridge_orchestrator.rs::drop_in_bridge_deck_entities` clears `on_bridge` and `bridge_occupancy` but does not relayer same-cell occupancy.
- `src/sim/occupancy.rs::rebuild` derives layer from locomotor state rather than `GameEntity::on_bridge`.

Additional status from this follow-up:

- Rust currently has no full `BulletClass` runtime equivalent for general projectiles; `BULLETTYPECLASS_GHIDRA_REPORT.md` already notes most weapon fire applies damage immediately, with only separate rocket movement support.
- Therefore the `0x006FF0B0` bullet `OnBridge` propagation is not implemented as a runtime projectile behavior.
- This is outside the current sim occupancy audit, but it is relevant when projectile/bridge collision parity is implemented.

## 7. Open Questions

1. Does the `0x0051A407` enter/conceal path remove a bridge-standing infantry from the ground list because `OnBridge` was cleared first, or was the infantry already unmarked by an earlier path? `OBJECTCLASS_GHIDRA_REPORT.md` says Conceal calls Mark-remove, but this exact enter-branch cell-list effect should be traced before implementing infantry enter on bridge cells.
2. Which stock weapons most commonly combine `Inviso=yes` with bridge targets in normal gameplay, and do their current Rust combat paths need an interim bridge-height rule before full BulletClass exists?
3. The parent writer table remains scoped evidence, not a full xref catalog. The classified false positives here reduce the unknown set from the verify-doc scan but do not prove every register-copy writer in the binary has been exhaustively audited.

## Sources

- Ghidra memory/disassembly reads from `gamemd.exe`: `0x0051A360`, `0x006DD680`, `0x006E3FB0`, `0x0051FD80`, `0x006FF050`, `0x0070C610`, `0x0070DDD0`, `0x0070DE00`, `0x00776EB0`.
- `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_OBJECT_ONBRIDGE_FIELD_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/OBJECTCLASS_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/BULLETTYPECLASS_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/BULLET_CLASS_AI_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/FIRE_AT_PIPELINE_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/ADDRESS_MAP.md`
- `ini/rulesmd.ini`, `ini/rules.ini` grep for `Inviso=yes`, infantry-enter and transport/garrison keys.
- Rust audit files: `src/sim/occupancy.rs`, `src/sim/game_entity.rs`, `src/sim/movement/movement_bridge.rs`, `src/sim/movement/movement_tick.rs`, `src/sim/movement/movement_step.rs`, `src/sim/world/bridge_orchestrator.rs`, `src/sim/world/world_spawn.rs`.
