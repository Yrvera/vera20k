# RulesClass Cliff Speed Multipliers — Ghidra Research Report

**Date:** 2026-05-19
**Investigator:** subagent slot 3 / re-swarm batch
**Active in YR:** Yes — every move across a height change triggers one of these multipliers.
**Confidence:** HIGH (binary verified — all four key strings read from binary, FSTP addresses
confirmed, INI lines located in rulesmd.ini, no prior doc had the exact key names verified)

---

## 1. Summary

Four `double` fields in `RulesClass` (singleton at `0x0066D530`) control how much a unit's
speed is scaled when it crosses a cell whose `GroundHeight` differs from the previous cell.
Two multipliers are for `SpeedType == 1` (Track), two for all other SpeedTypes.
Each axis has an uphill and a downhill variant.

The prior doc (`CLIFF_RAMP_TRAVERSAL_GHIDRA_REPORT.md §7.3`) had the **Downhill/Uphill
assignment for 0x768/0x770 reversed**. The binary shows TrackedUphill at 0x768,
TrackedDownhill at 0x770 (not the other way around). This report supersedes that mapping.

---

## 2. Verified offset → key mapping

All four key-name strings were read directly from binary (ASCII, `0x0083c87c`–`0x0083c8b8`).
All four FSTP instructions were confirmed to be the **sole write sites** for their offsets
(`search_byte_patterns` returned exactly one match each). The INI section is `[General]`
(section pointer `0x7F0C9C` → `0x00826278` → "General", read via `read_memory`).

| RulesClass offset | INI key (exact binary string) | Default¹ | INI value (rulesmd.ini) | INI line |
|---|---|---|---|---|
| `+0x768` | `TrackedUphill` | 0.0² | `1.0` | 401 |
| `+0x770` | `TrackedDownhill` | 0.0² | `1.2` | 402 |
| `+0x778` | `WheeledUphill` | 0.0² | `1.0` | 403 |
| `+0x780` | `WheeledDownhill` | 0.0² | `1.2` | 404 |

¹ "Default" here = the value ReadDouble receives when the key is absent from INI.
  The constructor zero-initialises the field (no separate initializer found); ReadDouble
  reads back the field's current value as its fallback, so a missing key leaves the
  field at 0.0. In practice this never fires — stock rulesmd.ini always ships all four keys.

² The practical shipped default is the INI value (1.0 or 1.2); the fallback-if-absent
  is 0.0 (field zero-initialised at RulesClass construction with no separate assignment).

---

## 3. Binary verification details

### String addresses (read_memory)

```
0x0083c87c  "WheeledDownhill"   (15 bytes + null)
0x0083c88c  "WheeledUphill"     (13 bytes + null)
0x0083c89c  "TrackedDownhill"   (15 bytes + null)
0x0083c8ac  "TrackedUphill"     (13 bytes + null)
```

### Call sites (RulesClass__ReadGeneral)

All four ReadDouble call sites are in `RulesClass__ReadGeneral`.
The xref from each key string was confirmed:

| Key string addr | Xref from | FSTP at | Writes RulesClass+ |
|---|---|---|---|
| `0x0083c8ac` (TrackedUphill) | `0x0066f227` | `0x0066f234` | `0x768` |
| `0x0083c89c` (TrackedDownhill) | `0x0066f24e` | `0x0066f25b` | `0x770` |
| `0x0083c88c` (WheeledUphill) | `0x0066f275` | `0x0066f282` | `0x778` |
| `0x0083c87c` (WheeledDownhill) | `0x0066f29c` | `0x0066f2a9` | `0x780` |

Call pattern (x86, each block identical except for string addr and struct offset):
```asm
MOV EAX, [ESI+offset+4]   ; high 4 bytes of current double (= default fallback)
MOV ECX, [ESI+offset]     ; low 4 bytes of current double  (= default fallback)
MOV EDX, [0x7F0C9C]       ; section = "General"
PUSH EAX
PUSH ECX
PUSH key_string_addr
PUSH EDX
MOV ECX, EDI              ; this = RulesClass*
CALL ReadDouble
FSTP qword ptr [ESI+offset]
```

### Sole write confirmation

`search_byte_patterns` for each FSTP opcode pattern:
- `DD 9E 68 07 00 00` → 1 match (0x0066f234) — TrackedUphill only writer
- `DD 9E 70 07 00 00` → 1 match (0x0066f25b) — TrackedDownhill only writer
- `DD 9E 78 07 00 00` → 1 match (0x0066f282) — WheeledUphill only writer
- `DD 9E 80 07 00 00` → 1 match (0x0066f2a9) — WheeledDownhill only writer

---

## 4. Clamping

**No clamping is applied.** The byte sequence after each FSTP immediately begins the
setup for the next ReadDouble call (confirmed via `read_memory 0x0066f23a` length 100
and `0x0066f2af` length 40). There are no comparisons (CMP/JLE/JGE), no `fmin`/`fmax`,
and no MIN/MAX helper calls between the FSTP and the next field load.

The values are used raw as `speed *= multiplier` in the Process_Movement consumer
(documented in `CLIFF_RAMP_TRAVERSAL_GHIDRA_REPORT.md §7.3`).

---

## 5. Correction to prior doc

`CLIFF_RAMP_TRAVERSAL_GHIDRA_REPORT.md §7.3` documented the consumer correctly but
had the offset→key mapping wrong:

```
PRIOR (wrong):
  +0x768 = TrackedDownhillSpeed
  +0x770 = TrackedUphillSpeed

CORRECT (verified from binary this pass):
  +0x768 = TrackedUphill   (=1.0 in rulesmd.ini)
  +0x770 = TrackedDownhill (=1.2 in rulesmd.ini)
```

The Wheeled pair (0x778 / 0x780) had the right order inferred (Downhill / Uphill),
but the binary confirms:
```
  +0x778 = WheeledUphill   (=1.0 in rulesmd.ini)
  +0x780 = WheeledDownhill (=1.2 in rulesmd.ini)
```

The `RULESCLASS_FIELDS.csv` rows 97–100 had the correct mapping; the §7.3 prose was
the source of the inversion. Open question #2 in §10 of the cliff doc is now resolved.

---

## 6. Active in YR — TS-legacy filter

No SpecialFlags gate found. The multipliers are unconditionally read from [General]
on every RulesClass load. Stock rulesmd.ini ships all four keys. Every unit move that
crosses a height boundary triggers the speed multiplication (as documented in §7.3 of
the cliff report). These are live, player-visible behavior.

---

## 7. Rust implementation note

For the Rust port, these four fields map to:

```rust
pub struct RulesGeneral {
    // ...
    pub tracked_uphill_speed:   f64,  // [General] TrackedUphill   = 1.0
    pub tracked_downhill_speed: f64,  // [General] TrackedDownhill = 1.2
    pub wheeled_uphill_speed:   f64,  // [General] WheeledUphill   = 1.0
    pub wheeled_downhill_speed: f64,  // [General] WheeledDownhill = 1.2
}
```

Per CLAUDE.md: simulation math uses `fixed`-point; these doubles should be parsed
from INI as `f64` and then converted to `Fixed` for use in the per-tick speed
multiply in Process_Movement. No clamping needed on read (none in original).

---

## 8. Sources

**Binary reads this pass:**
- `get_xrefs_to 0x0083c8ac` → TrackedUphill string xref
- `get_xrefs_to 0x0083c89c` → TrackedDownhill string xref
- `get_xrefs_to 0x0083c88c` → WheeledUphill string xref
- `get_xrefs_to 0x0083c87c` → WheeledDownhill string xref
- `read_memory 0x0083c87c` len 36 → confirmed all 4 string contents
- `read_memory 0x0066f1e0` len 200, `0x0066f23a` len 100, `0x0066f2af` len 40 → x86 call patterns + post-FSTP clamping check
- `read_memory 0x007F0C9C` + `0x00826278` → section string pointer → "General" confirmed
- `search_byte_patterns` DD 9E 68/70/78/80 07 00 00 → sole-write confirmation (1 match each)

**INI file:**
- `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini` lines 401–404 ([General])

**Companion docs:**
- `CLIFF_RAMP_TRAVERSAL_GHIDRA_REPORT.md` — consumer logic §7.3, Open Q #2 (now resolved)
- `RULESCLASS_FIELDS.csv` rows 96–100 — offset/key/type cross-reference (was already correct)

---

*End of report.*
