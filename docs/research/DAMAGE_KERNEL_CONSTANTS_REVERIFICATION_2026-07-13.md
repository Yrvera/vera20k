# Damage Kernel Constants Re-verification (2026-07-13)

**Status:** VERIFIED correction report  
**Target:** active retail Yuri's Revenge `gamemd.exe`  
**Scope:** CellSpread-to-lepton constant and `MaxDamage` fallback/runtime value  
**Implementation:** none

## Purpose

This report resolves two contradictory claims in the existing damage research
corpus:

1. whether the damage kernel uses 128 or 256 leptons per cell; and
2. whether standard stock YR runs with `MaxDamage` 1000 or 10000.

For these two facts, this report supersedes the conflicting clauses in:

- `GATE_DAMAGE_VERSES_F64_RESOLUTION_GHIDRA_REPORT.md` section D1c; and
- `GATE_DAMAGE_MAXDAMAGE_CLAMP_RESOLUTION_GHIDRA_REPORT.md` where the constructor
  fallback is described as the standard stock runtime value.

The rest of those reports is outside this correction's scope.

## Target Identity

**VERIFIED.** After reconnecting the existing `testProsjekt-12.1.2-test`
project, `GET /list_open_programs` reported current program `gamemd.exe`, image
base `00400000`, and executable path
`<ra2-install>/gamemd.exe`.

The findings below rely on instruction bodies and raw bytes, not Ghidra labels.

## Finding 1: The conversion constant is 256.0f

**Verdict: VERIFIED.** The active kernel and area dispatcher both multiply
CellSpread by the same 32-bit float at `0x007e2224`. Its bytes encode 256.0f.

### Raw value

`GET /read_memory?address=0x007e2224&length=4&program=gamemd.exe` returned:

```text
data = [0, 0, 128, 67]
hex  = 00008043
```

The little-endian bit pattern is `0x43800000`, IEEE-754 single precision
`256.0`. The prior 128 claim was a hex-to-decimal interpretation error; 128.0f
would be `0x43000000` (`00 00 00 43`).

### Damage kernel use

`GET /disassemble_function?address=0x00489180&program=gamemd.exe` returned the
following active instruction sequence:

```text
004891d8: FLD  float ptr [EDI + 0x124]
004891de: FMUL float ptr [0x007e2224]
004891e4: CALL 0x007c5f00
```

Therefore the kernel conversion is:

```text
cell_spread_leptons = Math__ftol(warhead.CellSpread * 256.0f)
```

### Area-dispatcher use

`GET /disassemble_function?address=0x00489280&program=gamemd.exe` returned:

```text
004892dd: FLD  float ptr [ESI + 0x124]
004892e3: FMUL float ptr [0x007e2224]
004892e9: CALL 0x007c5f00
```

The collection radius and receiver falloff conversion therefore use the same
256-lepton cell unit. This does not by itself certify every target-order or
distance-special-case behavior in `Apply_area_damage`.

## Finding 2: fallback 1000, stock runtime 10000

**Verdict: VERIFIED.** `RulesClass` constructs `MaxDamage` with 1000, then the
rules reader loads `[CombatDamage] MaxDamage` over that value. Both stock rule
files specify 10000.

### Constructor fallback

`GET /disassemble_function?address=0x006674d0&program=gamemd.exe` includes:

```text
006674f6: MOV dword ptr [ESI + 0x16c8],0x3e8
```

Thus a missing `MaxDamage` key leaves `Rules+0x16C8` at 1000.

### INI read into the same field

`GET /disassemble_function?address=0x0066ce2c&program=gamemd.exe` includes:

```text
0066ce2c: MOV EDX,dword ptr [ESI + 0x16c8]
0066ce3d: PUSH EDX
0066ce3e: PUSH 0x83ad4c
0066ce46: CALL 0x005276d0
0066ce51: MOV dword ptr [ESI + 0x16c8],EAX
```

`GET /read_memory?address=0x0083ad4c&length=16&program=gamemd.exe` returned bytes
beginning `4d617844616d61676500`, the ASCII string `MaxDamage\0`. The old value
is supplied as the parser fallback and the returned INI value is stored back to
the same field.

The immediately following read at `Rules+0x16C4` uses the string `MinDamage`,
confirmed by
`GET /read_memory?address=0x0083ad40&length=16&program=gamemd.exe`. Active-use
analysis of `MinDamage` is outside this narrow correction.

### Stock merged-rules value

Repository stock data contains:

```text
ini/rules.ini:716    MaxDamage=10000
ini/rulesmd.ini:896  MaxDamage=10000
```

YR `rulesmd.ini` patches the base data, but both files agree. Standard stock YR
therefore runs with `Rules+0x16C8 == 10000`. The value 1000 is only the
missing-key constructor fallback.

## Corrected Contract

| Item | Correct value/behavior | Evidence status |
|---|---|---|
| Leptons per cell in `ApplyWarheadDamage` | 256.0f, then `Math__ftol` | VERIFIED raw bytes + assembly |
| Leptons per cell in `Apply_area_damage` radius conversion | Same 256.0f constant | VERIFIED assembly |
| `MaxDamage` missing-key fallback | 1000 (`0x3E8`) | VERIFIED constructor assembly |
| Standard stock YR running `MaxDamage` | 10000 | VERIFIED parser assembly + both stock INIs |

## Implementation Handoff

- Store the `MaxDamage` constructor fallback as 1000; do not hardcode 10000 as
  the parser default.
- Load `[CombatDamage] MaxDamage` over that fallback. Stock merged rules then
  produce 10000 naturally.
- Use the exact stored CellSpread value and the 256.0f constant at the verified
  conversion point. Do not use 128.
- Preserve `Math__ftol` behavior. This report establishes constants and call
  sites, not host-float equivalence.
- Any plan or test derived from the superseded 128-lepton worked result must be
  recalculated or removed.

## Confidence and Limits

- **Constant bytes:** HIGH, raw memory read.
- **Instruction binding:** HIGH, both active function bodies read that address.
- **MaxDamage fallback and parser destination:** HIGH, direct instruction reads.
- **Stock runtime value:** HIGH for repository stock YR data, both patch layers
  explicitly set the same value.
- **Full damage parity:** not claimed. Numeric x87 equivalence, complete receiver
  stages, and full area-dispatch traversal remain separate verification gates.

