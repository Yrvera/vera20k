# STR_INIKey_ChronoHarvTooFarDistance — 0x0083c464

**Proposed Ghidra label:** STR_INIKey_ChronoHarvTooFarDistance
**Kind:** string / INI-key
**Task:** decode-string-chrono-harv-too-far
**Active in YR:** Yes — read by `RulesClass__ReadGeneral` unconditionally on startup

---

## Summary

The `ChronoHarvTooFarDistance` `[General]` INI key sets the maximum teleport range for
chrono harvesters. When the warp destination exceeds this distance, the harvester does
not teleport and drives normally instead, preventing cross-map warps. Stored as an
integer at `Rules+0xD7C`. Not consumed directly by `TeleportLocomotionClass`; consumed
by the harvester mission dispatch that decides whether to initiate a warp at all.

Verified via `inspect_memory_content 0x0083C464` (string confirmed) and
`get_assembly_context 0x00670003` (Rules struct offset from RulesClass__ReadGeneral).

---

## Active in YR

**Yes — unconditionally loaded** from `[General]` on startup.
`get_xrefs_to 0x0083C464` → single DATA ref from `RulesClass__ReadGeneral @ 0x00670003`.
No gating flag.

---

## String verification

`inspect_memory_content 0x0083C464` (28 bytes):
- Hex: `43 68 72 6F 6E 6F 48 61 72 76 54 6F 6F 46 61 72 44 69 73 74 61 6E 63 65 00 00 00 00`
- Detected string: "ChronoHarvTooFarDistance" (null at byte 24, char[25])

---

## Struct offset

From `get_assembly_context 0x00670003` (xref site in `RulesClass__ReadGeneral`):

```asm
0066fff0: MOV [ESI + 0xd78],EAX   ; store prior key result at Rules+0xD78
0066fff6: MOV ECX,[ESI + 0xd7c]   ; load existing Rules+0xD7C
00670003: PUSH 0x83c464            ; ← push "ChronoHarvTooFarDistance"
0067000b: CALL 0x005276d0          ; ReadInt("ChronoHarvTooFarDistance")
0067001b: MOV [ESI + 0xd7c],EAX   ; → store result at Rules+0xD7C
```

**Rules+0xD7C** (int) = ChronoHarvTooFarDistance.

---

## INI value

From `ini/rulesmd.ini` line 294:
```
ChronoHarvTooFarDistance=50
; gs Same as above, but for Chrono harvesters. Rather than have them teleport super
; far and then repick an ore patch (or teleport super far and drive super far back),
; they will stay on their side of the map (like for two bases)
```

Stock value: **50**. Units: YELLOW — likely cells given the map-side distance context.

---

## Behavioral role

Chrono harvesters should not warp across the entire map to an ore field on the enemy's
side. The threshold prevents this: if distance to the intended ore destination exceeds
ChronoHarvTooFarDistance, the harvester drives normally rather than warping.

The harvester mission state machine in `src/sim/miner/` checks this before calling
`TeleportLocomotionClass` to initiate a warp. This is the "too-far" gate referenced in
the chrono miner locomotion feedback note `[feedback_chrono_teleport_direction]`.

Adjacent key Rules+0xD78 (the prior ReadInt in the assembly, identity YELLOW) is likely
a related Chrono harvester distance variant.

---

## Proposed Ghidra label

| Symbol | Address | Proposed name |
|---|---|---|
| 0x0083C464 | string | STR_INIKey_ChronoHarvTooFarDistance |

---

## Out-of-scope refs

- `RulesClass__ReadGeneral` — general Rules infrastructure; not teleport-locomotion-specific
- `ReadInt` at 0x005276D0 — general INI reader; not teleport-specific
- Harvester mission state machine (`src/sim/miner/`) — consumer of Rules+0xD7C; not
  TeleportLocomotionClass scope

---

## Unverified / YELLOW

- **Rules+0xD7C units**: Stock value 50 in rulesmd.ini; comment implies map-scale distance.
  Whether engine treats it as cells or leptons depends on the consumption site — not
  traced to a specific comparison instruction in this session. YELLOW.
- **Rules+0xD78 adjacent key identity**: The prior ReadInt stores to Rules+0xD78.
  Likely another Chrono harvester distance threshold. Not decoded. YELLOW.
- **Consumption site in harvester mission SM**: Key confirmed read by ReadGeneral; Rust
  consumption site in `src/sim/miner/` not verified in this session. YELLOW.
