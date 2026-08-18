# Spark Light Effect Tick Rounding And First Visible Stage -- Reswarm Research Report

**Address(es):** `ParticleSystemClass::AI_Spark @ 0x0062E840`, `ParticleSystemClass::AI @ 0x0062FD60`, one-frame helper `0x0062E280`, light constructor `0x005FF250`, light updater `0x005FF390`, light draw/apply `0x005FF850`, persistent draw-all `0x005FFFA0`, `Math__ftol @ 0x007C5F00`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Exact one-frame spark-light radius conversion, first visible persistent-light stage, and ordering between spark-light creation, light-effect update, tactical draw, and expiry.
**Non-Scope:** Generic particle physics, all particle systems, map/cell lighting, combat lights, railgun trails, and non-spark light-effect callers.
**Confidence:** High for scoped mechanism; Medium only for how many physical frames are presented under render starvation.
**Active in YR:** Yes, conditional on stock spark systems being spawned; persistent spark lights additionally require high extra-animation/detail gate value `2`.

## 1. Overview

The remaining uncertainty from `PARTICLE_SPARK_LIGHTSIZE_ONEFRAMELIGHT_LIFECYCLE_GHIDRA_REPORT.md` is resolved. `Math__ftol` in this path truncates toward zero, so positive one-frame spark light radii are floored after the clamp/multiply. Persistent spark lights created by `AI_Spark` are first visible at stage `0`, because the global light-effect updater has already run earlier in the same logic pass.

The verified normal sequence for a persistent spark light is:

```text
logic light update -> live object/particle AI creates effect stage 0 -> tactical draw persistent light stage 0
next logic updates/draws: 8,16,24,32,40,48,56,64,72 -> next update 80 removes before draw
```

## 2. Class Layout / Key Offsets

| Offset | Owner | Type | Purpose in this slice | Evidence |
|--------|-------|------|-----------------------|----------|
| `+0x9C/+0xA0/+0xA4` | `ParticleSystemClass` | coord dwords | copied to the light-effect object | `0x0062EC2C..0x0062EC55`, `0x0062E329..0x0062E347` |
| `+0xAC` | `ParticleSystemClass` | `ParticleSystemTypeClass*` | source type pointer | `0x0062E287`, `0x0062EBFB` |
| `+0xCC` | `ParticleSystemClass` | int | live particle count; one-frame path requires `>0` | `0x0062E2AD..0x0062E2B9` |
| `+0xF0` | `ParticleSystemClass` | int | spark spawn-frame counter; compared before decrement | `0x0062EC01..0x0062EC0D` |
| `+0xF4` | `ParticleSystemClass` | int | copied to one-frame effect `+0x0C` stage/facing | `0x0062E352..0x0062E35A` |
| `+0x2A4` | `ParticleSystemTypeClass` | int | `ParticleCap` denominator | `0x0062E2C3` |
| `+0x300` | `ParticleSystemTypeClass` | int | `SparkSpawnFrames` | `0x0062EC07` |
| `+0x304` | `ParticleSystemTypeClass` | int | `LightSize` | `0x0062E29B`, `0x0062EC0F` |
| `+0x30C` | `ParticleSystemTypeClass` | bool | `OneFrameLight` | `0x0062E28D`, `0x0062EC19` |
| `+0x0C` | 0x18 light effect | int | draw stage/index; persistent starts at `0` | `0x005FF250`, `0x005FF390`, `0x005FF850` |
| `+0x10` | 0x18 light effect | int | radius/table selector input | `0x005FF250`, `0x005FF850` |
| `+0x14` | 0x18 light effect | flags | channel/behavior flags; spark constructors leave `0` | `0x005FF250`, `0x005FF850` |

## 3. Core Logic

### One-frame radius conversion

`FUN_0062E280 @ 0x0062E280` returns unless all gates pass:

1. `PSType+0x30C != 0` (`OneFrameLight=true`): `0x0062E28D..0x0062E295`.
2. `PSType+0x304 > 0` (`LightSize>0`): `0x0062E29B..0x0062E2A7`.
3. `PSC+0xCC > 0` (live particles exist): `0x0062E2AD..0x0062E2B9`.

The radius formula is:

```text
ratio = float(live_particle_count) / int(ParticleCap)
ratio = clamp(ratio, 0.4, 1.0)
radius = Math__ftol(ratio * LightSize)
```

Instruction proof:

| Step | Instructions | Detail |
|------|--------------|--------|
| load count | `0x0062E2BF: FILD dword ptr [ESP+0x8]` | signed integer live count to x87 |
| divide by cap | `0x0062E2C3: FIDIV dword ptr [EAX+0x2A4]` | integer `ParticleCap` denominator |
| upper clamp | `0x0062E2C9..0x0062E2D8` | compares against float `1.0` at `0x007E2AC8` |
| lower clamp | `0x0062E2E2..0x0062E2F1` | compares against double `0.4` at `0x007E3550` |
| multiply | `0x0062E310: FIMUL dword ptr [ESP+0xC]` | multiplies by integer `LightSize` |
| convert | `0x0062E314: CALL 0x007C5F00` | calls `Math__ftol` |

Binary-reader constants from retail `gamemd.exe`:

| VA | Bytes | Decoded value |
|----|-------|---------------|
| `0x007E2AC8` | `00 00 80 3F` | float `1.0` |
| `0x007E3550` | `9A 99 99 99 99 99 D9 3F` | double `0.4` |

`Math__ftol @ 0x007C5F00` uses `FISTP qword ptr [...]` under a cached x87 control word. Startup first calls the `_controlfp`-style wrapper with `(0x300, 0x300)` at `0x006BBFB7..0x006BBFC1`; the runtime mapper at `0x007CC01C` maps abstract `0x300` to x87 `0x0C00` rounding-control bits, i.e. chop/truncate. Then `0x006BBFC9` calls `0x007C5EE4`, which saves the current x87 control word into `0x00822D80`. `Math__ftol` compares the current word to that cached word at `0x007C5F03..0x007C5F19`; if needed it reloads the cached word at `0x007C5F26..0x007C5F32`, then executes `FISTP`.

Therefore the one-frame spark radius conversion is truncation toward zero. For this positive-radius expression, it is floor.

Acceptance boundary examples:

| Values | Native radius |
|--------|---------------|
| `LGSparkSys`: live `1`, cap `7`, `LightSize=25` | `10` |
| `LGSparkSys`: live `3`, cap `7`, `LightSize=25` | `10` |
| `LGSparkSys`: live `4`, cap `7`, `LightSize=25` | `14` |
| `LGSparkSys`: live `7`, cap `7`, `LightSize=25` | `25` |
| `WeldingSys`: live `1`, cap `15`, `LightSize=25` | `10` |
| artificial half: live `1`, cap `2`, `LightSize=25` | `12`, not `13` |

### Persistent creation and first visible stage

`AI_Spark @ 0x0062E840` creates persistent lights only when:

1. global extra animation/detail dword `0x00A8EB78 == 2` (`0x0062EBF2..0x0062EBF9`);
2. `PSC+0xF0 == PSType+0x300` (`0x0062EBFB..0x0062EC0D`);
3. `PSType+0x304 > 0` (`0x0062EC0F..0x0062EC17`);
4. `PSType+0x30C == 0` (`0x0062EC19..0x0062EC21`);
5. 0x18-byte allocation succeeds (`0x0062EC23..0x0062EC5B`).

The first visible persistent stage is verified as `0`:

1. `LogicClassPerTickUpdateLiveVector @ 0x0055AFB0` calls the global light updater at `0x0055B5BE: CALL 0x005FF390`.
2. The same function runs the live object vector later at `0x0055B5FB..0x0055B619`, calling each object vtable `+0x5C` at `0x0055B610`.
3. `ParticleSystemClass` vtable `+0x5C` is `0x0062FD60`; `ParticleSystemClass::AI` dispatches `BehavesLike=Spark` case `3` to `AI_Spark` at `0x0062FD96..0x0062FD9D`.
4. `FUN_005FF250 @ 0x005FF250` initializes new effect `+0x0C = 0`.
5. `TacticalClass_Draw` later calls persistent draw-all `FUN_005FFFA0` at `0x006D4664`, after `Tactical_ObjectRenderingLoop`.

Because creation happens after that logic frame's `FUN_005FF390` update, the newly-created persistent spark light is not aged to `8` before its first tactical persistent draw. First draw uses stage/index `0`.

### Persistent ageing and expiry

`FUN_005FF390 @ 0x005FF390` iterates the persistent light vector backwards. For each entry it reads `effect+0x0C`, adds `8`, writes the new value, and removes/frees the entry when the new stage is greater than `0x4F` (`0x005FF3A8..0x005FF40C`).

With first visible stage `0`, the ordinary visible stage sequence is:

```text
0, 8, 16, 24, 32, 40, 48, 56, 64, 72
```

The next update changes `72 -> 80`; since `80 > 0x4F`, the entry is removed before a stage-`80` draw can occur.

### One-frame draw ordering

`ObjectClass::DrawIt @ 0x005F4CF0` calls object vtable `+0x114` during object rendering. For `ParticleSystemClass`, vtable `0x007EFB9C + 0x114 = 0x0062E280`, so one-frame spark lights are drawn from the object draw path, not the persistent pass.

The one-frame helper constructs a temporary 0x18 light object, calls `FUN_005FF850` immediately, calls `FUN_005FF2D0`, then frees it (`0x0062E347..0x0062E36E`). `TacticalClass_Draw` calls `Tactical_ObjectRenderingLoop` before `FUN_005FFFA0`, so one-frame lights are applied during object rendering, while persistent lights are applied afterward by the persistent vector pass.

## 4. INI Keys

| Key | Stock YR values checked | Binary effect |
|-----|-------------------------|---------------|
| `BehavesLike=Spark` | `SparkSys`, `WeldingSys`, `FirestormSparkSys`, `LGSparkSys` | selects `ParticleSystemClass::AI_Spark` case `3` |
| `ParticleCap` | `SparkSys=6`, `WeldingSys=15`, `FirestormSparkSys=20`, `LGSparkSys=7` | one-frame denominator at `PSType+0x2A4` |
| `SparkSpawnFrames` | `SparkSys=1`, `WeldingSys=20`, `FirestormSparkSys=1`, `LGSparkSys=5` | persistent creation compares `PSC+0xF0` to `PSType+0x300` |
| `LightSize` | `SparkSys=15`, `WeldingSys=25`, `FirestormSparkSys=21`, `LGSparkSys=25` | persistent raw radius; one-frame multiplicand |
| `OneFrameLight` | true for `WeldingSys` and `LGSparkSys`; absent/false for `SparkSys` and `FirestormSparkSys` | selects one-frame helper vs persistent creation |

Sources: `ini/rulesmd.ini:25935..26030`; base fallback `ini/rules.ini:18742..18832`.

## 5. Integration Points

| Integration | Ordering / role | Evidence |
|-------------|-----------------|----------|
| FPU rounding setup | sets/caches chop/truncate before `Math__ftol` | `0x006BBFB7..0x006BBFC9`, `0x007CBF49`, `0x007CC01C`, `0x007C5EE4` |
| Global persistent light update | ages/removes old lights before live object particle AI | `0x0055B5BE` |
| Live object vector AI | calls object vtable `+0x5C` after light update | `0x0055B5FB..0x0055B619` |
| Spark AI creation | creates persistent spark light at stage `0` | `0x0062FD60`, `0x0062E840`, `0x005FF250` |
| Object rendering | invokes particle vtable `+0x114` one-frame helper | `ObjectClass::DrawIt @ 0x005F4CF0`, `0x005F65D0` |
| Tactical persistent light pass | draws persistent vector after object rendering | `TacticalClass_Draw @ 0x006D4664`, `0x005FFFA0` |

## 6. Current Rust Implementation Status

| Rust surface | Status |
|--------------|--------|
| `src/rules/particle_system_type.rs:223..228` | parses `SparkSpawnFrames`, `LightSize`, and `OneFrameLight` |
| `src/sim/particles/spawn.rs:41..49` | skips `Spark` and `Railgun` particle systems entirely |
| `src/sim/particles/system_ai.rs:103..112` | treats `Spark` and `Railgun` as no-op tier-3 paths |
| `src/sim/world/mod.rs:1666..1669` | has a particle tick phase, but no native-equivalent global light-effect update/draw split |
| `src/map/lighting.rs`, `src/app_init.rs` | implement map/cell and building point lighting, not this screen-space 0x18 light-effect vector |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|--------------------------|--------|----------|--------------|
| One-frame helper gates | verified | `0x0062E28D..0x0062E2B9` | none |
| Radius clamp constants | verified | `0x0062E2C9..0x0062E2F1`; binary-reader constants | none |
| `Math__ftol` rounding | verified | `0x007C5F00`; `0x006BBFC1..0x006BBFC9`; `0x007CC01C` | none |
| Persistent creation gates | verified | `0x0062EBF2..0x0062EC60` | none |
| First visible persistent stage | verified | `0x0055B5BE` before `0x0055B5FB..0x0055B619`; `0x005FF250`; `0x006D4664` | none |
| Persistent expiry | verified | `0x005FF390..0x005FF40C` | none |
| One-frame draw ordering | verified | `0x005F4CF0`; `0x005F65DC`; `0x0062E35D..0x0062E36E`; `0x006D4664` | none |
| Stock INI values | verified | `ini/rulesmd.ini:25935..26030` | none |
| Physical presentation under skipped renders | touched-not-exhausted | draw is separate from logic; no runtime capture | capture runtime if exact presented-frame count under starvation matters |
| Non-spark light-effect callers | deferred | out of scope | separate caller taxonomy |

## 8. Open Questions -- Final State of the Investigation Log

- `[RESOLVED] OQ-01 -- What exact integer rounding does one-frame radius use? -> x87 `FISTP` under startup-cached chop/truncate control word; positive radius floors.` (evidence: `0x007C5F00`, `0x006BBFC1..0x006BBFC9`, `0x007CC01C`)
- `[RESOLVED] OQ-02 -- Are clamp constants exactly `1.0` and `0.4`? -> Yes.` (evidence: `0x0062E2C9..0x0062E2F1`; binary-reader constants at `0x007E2AC8`, `0x007E3550`)
- `[RESOLVED] OQ-03 -- Is `LightSize` multiplied before or after conversion? -> Before conversion, via `FIMUL`, then `Math__ftol`.` (evidence: `0x0062E310..0x0062E314`)
- `[RESOLVED] OQ-04 -- Does one-frame mode require live particles? -> Yes, `PSC+0xCC > 0`.` (evidence: `0x0062E2AD..0x0062E2B9`)
- `[RESOLVED] OQ-05 -- Does persistent creation happen before or after the global light updater? -> After, in the later live object vector loop.` (evidence: `0x0055B5BE`; `0x0055B5FB..0x0055B619`)
- `[RESOLVED] OQ-06 -- Which live dispatch reaches spark AI? -> ParticleSystem vtable `+0x5C` is `0x0062FD60`; Spark case calls `0x0062E840`.` (evidence: vtable `0x007EFBF8`; `0x0062FD96..0x0062FD9D`)
- `[RESOLVED] OQ-07 -- What stage does a new persistent light hold? -> `0`.` (evidence: `FUN_005FF250 @ 0x005FF250`)
- `[RESOLVED] OQ-08 -- What is first visible persistent stage? -> `0`, because update already ran and tactical draw is later.` (evidence: `0x0055B5BE`; `0x006D4664`)
- `[RESOLVED] OQ-09 -- Is stage `80` drawn? -> No; `72 -> 80` removes/free before draw.` (evidence: `0x005FF3A8..0x005FF40C`)
- `[RESOLVED] OQ-10 -- Where does one-frame drawing occur? -> During object rendering via vtable `+0x114`, not the persistent vector pass.` (evidence: `0x005F4CF0`; `0x005F65DC`; vtable `0x007EFCB0`)
- `[RESOLVED] OQ-11 -- Does one-frame leave a persistent entry? -> No; it draws, removes, and frees immediately.` (evidence: `0x0062E35D..0x0062E36E`)
- `[RESOLVED] OQ-12 -- Are stock `LGSparkSys` and `WeldingSys` one-frame systems? -> Yes; both set `OneFrameLight=true`.` (evidence: `ini/rulesmd.ini:25935..26030`)
- `[RESOLVED] OQ-13 -- Does Rust already implement this? -> No; Spark/Railgun systems are skipped/no-op and lighting is map/cell point-light based.` (evidence: `src/sim/particles/spawn.rs:41..49`, `src/sim/particles/system_ai.rs:103..112`)
- `[DEFERRED] OQ-14 -- How many physical presented frames appear under render starvation?` (category: `needs-runtime-debugger`; reason: logic-stage sequence is verified, but presentation can skip draw opportunities; next-step-if-pursued: runtime capture around deterministic `SparkSys` spawn)
- `[DEFERRED] OQ-15 -- Do non-spark callers of `FUN_005FF250/850/390/FFA0` have different first-stage ordering?` (category: `out-of-scope`; reason: target is spark light effects only; next-step-if-pursued: run separate light-effect caller taxonomy)

## 9. Visual/UI Composition Ledger

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|-------|--------------------|------------------------|---------------|---------------|-------------------|--------------------|------|
| 1 | `Tactical_ObjectRenderingLoop` | normal tactical draw path | object/particle visuals | object-specific | normal object draw path | yes | base object rendering |
| 2 | one-frame helper `0x0062E280` via object vtable `+0x114` | `OneFrameLight=true`, `LightSize>0`, live count `>0` | generated 0x18 light effect, table selected from `DAT_00AC1698` | coords converted by `TacticalClass__CoordsToClient2`; draw rect starts around `client_x-0x80`, `client_y-0x40` | direct primary-surface 16-bit light-effect path in `FUN_005FF850` | yes for `WeldingSys`/`LGSparkSys` | immediate overlay |
| 3 | persistent draw-all `0x005FFFA0`, call site `0x006D4664` | persistent vector count `DAT_00AC1688` | same 0x18 light-effect table path | same draw/apply helper | direct primary-surface 16-bit light-effect path | yes for persistent spark systems when detail gate passes | persistent overlay after object rendering |

Asset role matrix:

| Asset | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|-------|--------|-------|-------------------|-----------------|------------------|---------|-----------------|----------|----------|
| 0x18 light effect object | allocated at runtime | yes | conditional | no | no | yes | no | no | `0x005FF250`, `0x0062E347..0x0062E36E`, `0x005FFFA0` |
| Static light-effect table `DAT_00AC1698` | global binary data | yes | conditional | no | no | yes | no | no | `FUN_005FF850` |
| Particle color lists | parsed elsewhere | not by this light constructor | no for this slice | no | no | no | no | yes | `FUN_005FF250` takes coords + radius only |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|-------------------|----------|--------------------|-----------------------|--------------------------------|---------------------|------------------|
| One-frame radius is `trunc_toward_zero(clamp(live_count / ParticleCap, 0.4, 1.0) * LightSize)` | `0x0062E2BF..0x0062E314`; `0x007C5F00`; startup `0x006BBFC1..0x006BBFC9` | missing | future spark AI/render light-effect bridge; parsed fields already in `src/rules/particle_system_type.rs` | use truncating integer conversion after clamp/multiply | `LGSparkSys` live `4/7`, `LightSize=25` yields `14`; artificial `1/2*25` yields `12` | do not use `round()` or raw `LightSize` |
| Persistent spark lights first draw at stage `0` | updater `0x0055B5BE` precedes live loop `0x0055B5FB..0x0055B619`; constructor `0x005FF250`; draw `0x006D4664` | missing | screen-space persistent light-effect vector and tick/draw bridge | create after that frame's light update or otherwise preserve first draw stage `0` | `SparkSys` under high detail creates an effect whose first draw uses `+0x0C == 0` | do not age newly-created persistent lights before first draw |
| Persistent stages draw `0..72`; `80` expires before draw | `0x005FF3A8..0x005FF40C` | missing | light-effect lifetime/update scheduler | add `8` per logic light update; remove when new stage `>0x4F` | deterministic lifetime test removes on `72 -> 80` | do not use `>=0x4F`; do not draw stage `80` |
| One-frame lights draw during object rendering and never persist | `0x005F4CF0`; `0x005F65DC`; `0x0062E35D..0x0062E36E`; `0x006D4664` | missing | object/render pass or immediate screen-light queue | emit immediate light only when live particles exist; do not enter persistent vector | `LGSparkSys` emits one-frame draw and leaves persistent vector unchanged | do not age or persist one-frame lights |
| Rust skips spark systems entirely | `src/sim/particles/spawn.rs:41..49`; `src/sim/particles/system_ai.rs:103..112` | missing | `src/sim/particles/*`, render bridge | implement spark particle spawning/ticking before parity light tests can pass | damaged object using `DamageParticleSystems=SparkSys,LGSparkSys` spawns sparks and correct light mode | do not route through `src/map/lighting.rs` |

Stale doc replacement wording for `PARTICLE_SPARK_LIGHTSIZE_ONEFRAMELIGHT_LIFECYCLE_GHIDRA_REPORT.md`:

- Replace the rounding uncertainty with: "`Math__ftol @ 0x007C5F00` uses x87 `FISTP` under the startup-cached chop/truncate control word set at `0x006BBFC1..0x006BBFC9`; one-frame spark radius truncates toward zero after clamp/multiply. Since radius is positive, this is floor."
- Replace the first-visible-stage uncertainty with: "The first visible persistent stage is `0`: `LogicClassPerTickUpdateLiveVector` calls `FUN_005FF390` at `0x0055B5BE` before the live object vtable `+0x5C` loop at `0x0055B5FB..0x0055B619`; `ParticleSystemClass::AI` creates the persistent light later in that loop, and `TacticalClass_Draw` calls `FUN_005FFFA0` afterward at `0x006D4664`."

## Negative Facts / Do Not Do

- Do not round one-frame radius to nearest; native truncates toward zero.
- Do not use raw `LightSize` for one-frame mode.
- Do not age persistent spark lights during creation; first visible stage is `0`.
- Do not draw stage `80`; it is removed before draw.
- Do not implement spark lights as map/cell lighting; this is a screen-space primary-surface effect.
- Do not persist `OneFrameLight=true` entries.
- Do not consume particle color lists for this light effect; constructor input is coords plus radius.

## Remaining Uncertainty

No material uncertainty remains for the scoped spark mechanism. Runtime presentation can skip draw opportunities under load/window conditions, but the logic-stage sequence and expiry rule are verified. Non-spark users of the same helper family remain out of scope.

## Rust-Facing Acceptance Scenario / Test Suggestions

- `spark_one_frame_radius_truncates_after_clamp_multiply`: assert the boundary radii above.
- `spark_persistent_first_draw_stage_zero`: create a persistent spark light after the light-update phase and assert next persistent draw sees stage `0`.
- `spark_persistent_expires_before_stage_80_draw`: assert removal on `72 -> 80`, with no stage-`80` draw.
- `spark_one_frame_draws_during_object_render_not_persistent_pass`: assert `LGSparkSys` immediate draw does not leave a persistent vector entry.
- `spark_light_detail_gate_only_affects_persistent_creation`: persistent `SparkSys` honors extra-animation/detail value `2`; one-frame helper uses its own gates.

## Sources

- Prior report: `C:/Users/enok/Documents/ra2-rust-game/docs/research/PARTICLE_SPARK_LIGHTSIZE_ONEFRAMELIGHT_LIFECYCLE_GHIDRA_REPORT.md`
- Ghidra read-only decompile/assembly context: `0x0062E280`, `0x0062E840`, `0x0062FD60`, `0x005FF250`, `0x005FF390`, `0x005FF850`, `0x005FFFA0`, `0x0055AFB0`, `0x005F4CF0`, `0x005F65D0`, `0x006D3B60`, `0x007C5F00`, `0x007CBF49`, `0x007CC01C`, `0x007C5EE4`, startup caller `0x006BBFC1..0x006BBFC9`.
- Binary-reader constants from retail `gamemd.exe`: `0x007E2AC8 == float 1.0`, `0x007E3550 == double 0.4`.
- INI files checked: `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`, `C:/Users/enok/Documents/ra2-rust-game/ini/rules.ini`.
- Rust files scanned: `C:/Users/enok/Documents/ra2-rust-game/src/rules/particle_system_type.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/sim/particles/spawn.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/sim/particles/system_ai.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/mod.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/map/lighting.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/app_init.rs`.
