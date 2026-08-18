# Skirmish Speed and Particle Normalized Micro-Pass

Date: 2026-05-16

Binary: `gamemd.exe` from the configured Yuri's Revenge retail install.

Scope:

- Resolve the retail/default skirmish speed source after startup: stored speed `1`, temporary `2`, or options default `3`.
- Isolate the exact `ParticleType.Normalized` arithmetic in `ParticleClass__Constructor @ 0x0062B5E0`.

## Executive Result

Normal default YR skirmish uses stored game speed `1` after startup when no `[Skirmish] GameSpeed=` override exists in `RA2MD.INI`. The installed retail `RA2MD.INI` has `[Options] GameSpeed=3`, but no `[Skirmish] GameSpeed=`, and the skirmish loader does not use the options value as its fallback.

The speed-`2` path is real, but it is gated on game mode `0`. Normal skirmish is game mode `5`, so the startup and tick paths do not replace the skirmish speed with `2`.

`ParticleType.Normalized=yes` rewrites the per-particle `StateAIAdvance` byte at `ParticleClass+0x12C`. The rewrite is based on normalized X/Y component travel time and `FinalDamageState + 1`, not on the INI `StateAIAdvance` byte and not directly on `EndStateAI`.

## 1. Retail/Default Skirmish Speed Probe

### Verified Binary Findings

`RulesClass__ReadMultiplayerDialogSettings @ 0x00671EA0` reads:

```c
RulesClass+0x14A0 = ReadInt("MultiplayerDialogSettings", "GameSpeed", old_value);
```

Retail INI values:

```ini
; rulesmd.ini
[MultiplayerDialogSettings]
GameSpeed=1

; rules.ini
[MultiplayerDialogSettings]
GameSpeed=0
```

YR `rulesmd.ini` patches the base RA2 value, so the default multiplayer/skirmish dialog speed source is `1`.

`SessionClass__ReadSkirmishSettings @ 0x00697F10` reads the skirmish setting into the skirmish settings struct:

```c
settings+0x08 = ReadInt(section, "GameSpeed", RulesClass+0x14A0);
```

So `[Skirmish] GameSpeed=` can override the rules default, but if it is absent the fallback is `RulesClass+0x14A0`, i.e. `1` for YR.

The installed `RA2MD.INI` currently contains:

```ini
[Options]
GameSpeed=3
```

No `[Skirmish] GameSpeed=` was found in that file. This means the installed local default skirmish speed resolves to `1`, not the options value `3`.

The skirmish/lobby packet bridge also preserves that stored speed. The packet decode/update path at `0x005B67F0` copies packet byte `+0xA2` to the lobby/global speed and then to the live throttle speed:

```c
DAT_00A8B268 = *(byte *)(packet + 0xA2);
DAT_00A8EB60 = DAT_00A8B268;
```

The lobby speed slider uses inverted UI position:

```asm
slider_position = 6 - DAT_00A8B268
```

This confirms the stored speed byte is the canonical value, with `0=fastest` and `6=slowest`.

### The Speed-2 Path

`Main_Tick @ 0x0055D360` has a temporary speed-`2` path:

```c
if (g_GameMode == 0 && DAT_00A8EDDC == 0) {
    DAT_00A8EB60 = 2;
    DAT_00887350 = 2;
}
```

For skirmish, `g_GameMode == 5`, so this branch is skipped. The skirmish path instead carries the existing live speed into the tick throttle:

```c
DAT_00887350 = DAT_00A8EB60;
```

`FUN_0069BAB0 @ 0x0069BAB0` is also a real speed-`2` entry point:

```c
void __fastcall FUN_0069BAB0(int *mode_state) {
    if (((char)mode_state[0xC36] == 0) && g_GameActive != 0) {
        *(byte *)(mode_state + 0xC36) = 1;
        if (DAT_0083ED20 == -1) {
            DAT_0083ED20 = DAT_00A8EB60;
        }
        if (*mode_state == 0) {
            DAT_00A8EB60 = 2;
        }
        ...
    }
}
```

Call sites pass `ECX = 0x00A8B238`, whose first dword is `g_GameMode`. The main gameplay startup call immediately before the gameplay loop is:

```asm
0048CE79  MOV ECX,0x00A8B238
0048CE85  CALL 0x0069BAB0
```

Therefore, `FUN_0069BAB0` only forces speed `2` when `g_GameMode == 0`. For normal skirmish (`g_GameMode == 5`), it can cache and later restore the current speed, but it does not overwrite it.

`FUN_0069BB40 @ 0x0069BB40` is the paired restore:

```c
if (*(byte *)(mode_state + 0x30D8) != 0) {
    *(byte *)(mode_state + 0x30D8) = 0;
    if (DAT_0083ED20 != -1) {
        DAT_00A8EB60 = DAT_0083ED20;
        DAT_0083ED20 = -1;
    }
    ...
}
```

### Speed Conclusion

For retail/default YR skirmish on the checked install:

1. `rulesmd.ini [MultiplayerDialogSettings] GameSpeed=1`.
2. `RA2MD.INI [Skirmish] GameSpeed=` is absent, so skirmish settings fall back to `1`.
3. The lobby/session bridge copies that stored value to live `DAT_00A8EB60`.
4. `[Options] GameSpeed=3` is not the skirmish settings fallback.
5. The verified speed-`2` paths are gated on game mode `0`, not standard skirmish mode `5`.

Result: normal default YR skirmish should be calibrated as stored/live speed `1`, unless the user's `RA2MD.INI` has an explicit `[Skirmish] GameSpeed=` override or the player changes the skirmish speed slider.

## 2. Particle `Normalized` Arithmetic

### Verified Layout

`ParticleClass__Constructor @ 0x0062B5E0` copies type fields first:

```c
particle+0x12C = particle_type+0x30B; // StateAIAdvance
particle+0x12E = particle_type+0x30A; // StartStateAI
particle+0x12F = particle_type+0x2F4; // translucency-related byte
```

If `ParticleType.Normalized` at `type+0x30F` is false, `particle+0x12C` remains the INI `StateAIAdvance`.

If `ParticleType.Normalized` is true, the constructor rewrites `particle+0x12C` after normalizing the movement vector.

Relevant type offsets:

| Offset | Meaning |
| --- | --- |
| `type+0x2FC` | `Velocity` |
| `type+0x30B` | `StateAIAdvance` |
| `type+0x30C` | `FinalDamageState` |
| `type+0x30F` | `Normalized` |

Relevant particle offsets:

| Offset | Meaning |
| --- | --- |
| `particle+0x0E4` | current velocity, initialized from `type+0x2FC` |
| `particle+0x10C` | normalized X direction component |
| `particle+0x110` | normalized Y direction component |
| `particle+0x114` | normalized Z direction component |
| `particle+0x12C` | per-particle `StateAIAdvance` |

### Exact Arithmetic

Before the `Normalized` block, the constructor computes:

```c
dx = target.x - source.x;
dy = target.y - source.y;
dz = target.z - source.z;

distance = Sqrt_Approx(dx*dx + dy*dy + dz*dz);

if (distance != 0.0) {
    dir_x = dx / distance;
    dir_y = dy / distance;
    dir_z = dz / distance;
} else {
    dir_x = dx;
    dir_y = dy;
    dir_z = dz;
}
```

The `Normalized=yes` rewrite then uses only the X and Y axes for the travel-time estimate, although the direction components were normalized using the full 3D distance:

```c
step_x = abs(ftol_chop(dir_x * particle_velocity));
step_y = abs(ftol_chop(dir_y * particle_velocity));

best_ticks = 9999.0f;

if ((float)step_x > 0.000001) {
    best_ticks = abs(source.x - target.x) / (float)step_x;
}

if ((float)step_y > 0.000001) {
    y_ticks = abs(source.y - target.y) / (float)step_y;
    if (best_ticks >= y_ticks) {
        best_ticks = y_ticks;
    }
}

advance = ftol_chop(best_ticks / (FinalDamageState + 1) + 1.0);
particle.StateAIAdvance = (uint8_t)advance;
```

Important details:

- `Math__ftol @ 0x007C5F00` uses FPU control word `0x0E7F`, i.e. x87 rounding mode "toward zero". Treat it as truncation toward zero, not round-to-nearest.
- Component steps are integer after truncation and absolute value.
- The axis guard is `<= 0.000001`, so a truncated component step of `0` is skipped.
- The selected travel time is the smaller valid X/Y time. Z has no separate candidate.
- The denominator is `FinalDamageState + 1`, read from `type+0x30C`. For `FireStream`, this is `14 + 1`, not `EndStateAI + 1` (`19 + 1`) and not the original `StateAIAdvance=6`.
- The final `advance` is stored as one byte at `particle+0x12C`, so any out-of-range result is truncated to the low 8 bits by the byte store.

### Assembly Evidence

The branch guard:

```asm
0062B956  MOV EAX,dword ptr [ESI + 0xAC]    ; particle type
0062B95C  CMP byte ptr [EAX + 0x30F],BL      ; Normalized
0062B962  JE  0x0062BA51                     ; skip rewrite
```

X/Y component speed:

```asm
0062B96E  FLD  dword ptr [EBP]               ; dir_x
0062B976  FMUL dword ptr [ESI + 0xE4]        ; velocity
0062B987  CALL 0x007C5F00                    ; ftol_chop
0062B98C  CDQ
0062B98D  XOR EAX,EDX
0062B98F  SUB EAX,EDX                        ; abs(step_x)

0062B99D  FLD  dword ptr [ESI + 0x110]       ; dir_y
0062B9A3  FMUL dword ptr [ESI + 0xE4]        ; velocity
0062B9A9  CALL 0x007C5F00                    ; ftol_chop
0062B9AE  CDQ
0062B9AF  XOR EAX,EDX
0062B9B1  SUB EAX,EDX                        ; abs(step_y)
```

X candidate, with `9999.0f` fallback and `1e-6` guard:

```asm
0062B9BF  FLD   dword ptr [0x007EF920]       ; 9999.0f
0062B9C5  FLD   dword ptr [ESP + 0x34]       ; step_x float
0062B9C9  FCOMP qword ptr [0x007EF918]       ; 1e-6
0062B9D1  TEST  AH,0x41
0062B9D4  JNE   0x0062B9ED                   ; skip if <= 1e-6
0062B9DB  FSTP  ST0                          ; drop fallback
0062B9E5  FILD  dword ptr [ESP + 0x30]       ; abs(source.x-target.x)
0062B9E9  FDIV  dword ptr [ESP + 0x34]       ; x_ticks
```

Y candidate and minimum selection:

```asm
0062B9ED  FLD   dword ptr [ESP + 0x2C]       ; step_y float
0062B9F1  FCOMP qword ptr [0x007EF918]       ; 1e-6
0062B9F9  TEST  AH,0x41
0062B9FC  JNE   0x0062BA26                   ; skip if <= 1e-6
0062BA09  FILD  dword ptr [ESP + 0x34]       ; abs(source.y-target.y)
0062BA0D  FDIV  dword ptr [ESP + 0x2C]       ; y_ticks
0062BA15  FCOM  dword ptr [ESP + 0x34]
0062BA1B  TEST  AH,0x1
0062BA1E  JNE   0x0062BA26                   ; keep current if current < y
0062BA20  FSTP  ST0
0062BA22  FLD   dword ptr [ESP + 0x34]       ; replace with y_ticks
```

Final rewrite:

```asm
0062BA26  MOV   EAX,dword ptr [ESI + 0xAC]
0062BA2C  MOVSX ECX,byte ptr [EAX + 0x30C]   ; FinalDamageState
0062BA33  INC   ECX                          ; +1
0062BA38  FILD  dword ptr [ESP + 0x34]
0062BA3C  FDIVR ST0,ST1                      ; best_ticks / (FinalDamageState+1)
0062BA3E  FADD  qword ptr [0x007E1718]       ; +1.0
0062BA44  CALL  0x007C5F00                   ; ftol_chop
0062BA4B  MOV   byte ptr [ESI + 0x12C],AL    ; StateAIAdvance rewrite
```

Constants:

| Address | Value |
| --- | --- |
| `0x007EF920` | `9999.0f` |
| `0x007EF918` | `1e-6` |
| `0x007E1718` | `1.0` |

### FireStream Example

Retail `rulesmd.ini`:

```ini
[FireStream]
Velocity=28.0
BehavesLike=Fire
StartStateAI=1
EndStateAI=19
StateAIAdvance=6
Normalized=yes
FinalDamageState=14
```

For `FireStream`, the constructor first copies `StateAIAdvance=6`, then `Normalized=yes` overwrites it with:

```text
floor_toward_zero(min_valid_xy_travel_ticks / 15 + 1)
```

where `15` is `FinalDamageState + 1`.

## Implementation Implications

- Wall-clock speed calibration for default YR skirmish should use stored speed `1`, not `2` or `[Options] GameSpeed=3`.
- The engine should still support `[Skirmish] GameSpeed=` overrides and explicit player slider changes.
- The speed-`2` logic should be modeled only for the game-mode-`0` paths that actually use it, not for standard skirmish.
- Particle spawn must not leave `StateAIAdvance` as the INI byte for `Normalized=yes` particles. It should compute the constructor rewrite once per particle using the exact truncating arithmetic above.
- `FinalDamageState` is load-bearing for normalized particle timing. A particle implementation that uses `EndStateAI` here will make `FireStream` visibly wrong.

