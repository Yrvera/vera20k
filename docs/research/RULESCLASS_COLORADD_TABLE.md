---
title: RulesClass [ColorAdd] packed-RGB table
source_addr: 0x0066D480
rulesclass_field_offset: 0x1874
owner_report: RULESCLASS_GHIDRA_REPORT.md §2 (Class Layout)
yr_active_in_stock_game: YES
writes_to_rulesclass: YES (writes directly into the RulesClass instance at +0x1874)
verified_from: gamemd.exe live decompilation (Ghidra MCP, 2026-04-24); cross-checked against ini/rulesmd.ini §[ColorAdd]
---

# `[ColorAdd]` packed-RGB table

## Summary

`FUN_0066D480` ("ReadColorAdd") parses the `[ColorAdd]` section of
`rulesmd.ini` into a **packed 3-bytes-per-entry RGB array at
`RulesClass + 0x1874`**. Stock YR ships 14 entries (numbered 0–13).
The reader writes **only as many slots as the INI has** — unused tail
slots retain whatever the RulesClass constructor left at those bytes.

The plan's "16-entry / 48-byte" assumption was based on the ~0x30 bytes
of unassigned space after `+0x1874`; the actual per-slot write count is
bounded by `CCINIClass::Entry_Count(section)`, so table length is
INI-driven, not fixed.

## Call chain

```
CDFileClass__Constructor  @ 0x00668B50..              (outer orchestrator,
                                                       formerly labelled;
                                                       it's really `RulesClass::Process`)
  └─ FUN_0066D480(this=RulesClass*, undefined4)       @ 0x0052D111 caller, also
                                                        reachable from step 2 of
                                                        the inner dispatcher
                                                        FUN_00668BF0
```

Only one caller (`CDFileClass__Constructor @ 0x0052D111`), fired during
the rules-load pass.

## Function body

```c
undefined4 __thiscall FUN_0066D480(RulesClass* this, undefined4 /*unused*/) {
    if (CCINIClass__Find_Section("ColorAdd") == 0) return 0;

    int count = CCINIClass__Entry_Count("ColorAdd");
    uint8_t* dst = (uint8_t*)((char*)this + 0x1874);

    for (int i = 0; i < count; ++i) {
        const char* key = CCINIClass__Get_Entry_Key_Name("ColorAdd", i);
        uint8_t default_rgb[3] = {0, 0, 0};
        uint8_t* src = CCINIClass__ReadColorRGB(&local_rgb_buf,
                                                "ColorAdd",
                                                key,
                                                default_rgb);
        dst[0] = src[0];   // R
        dst[1] = src[1];   // G
        dst[2] = src[2];   // B
        dst += 3;          // next slot
    }
    return 1;
}
```

- `ReadColorRGB` returns a pointer to a 3-byte RGB triple stored in a
  stack-local buffer within the CCINIClass helper; the reader copies those
  3 bytes into the RulesClass instance.
- Iteration key is **entry *name*** read by enumeration (`FUN_00526CC0`),
  not a fixed pointer table. This means the section can be re-ordered or
  have keys renamed without breaking the reader — each key's position in
  the INI determines its slot index.
- The `undefined4 param_2` on the signature is a stale register argument
  tracked by the Ghidra signature recovery; it is not referenced by the
  function body.

## Stock slot layout (from `ini/rulesmd.ini` §[ColorAdd])

| Slot | INI key | R | G | B | Notes |
|---:|---|---:|---:|---:|---|
| 0 | `None` | 0 | 0 | 0 | all-zero sentinel |
| 1 | `StrongRed` | 31 | 0 | 0 | |
| 2 | `StrongGreen` | 0 | 63 | 0 | |
| 3 | `StrongBlue` | 0 | 0 | 31 | |
| 4 | `HighRed` | 24 | 0 | 0 | |
| 5 | `HighGreen` | 0 | 56 | 0 | |
| 6 | `HighBlue` | 0 | 0 | 24 | |
| 7 | `BrightWhite` | 31 | 63 | 31 | |
| 8 | `LowWhite` | 7 | 7 | 7 | |
| 9 | `HighWhite` | 24 | 56 | 24 | |
| 10 | `MidWhite` | 14 | 28 | 14 | |
| 11 | `Purple` | 15 | 0 | 15 | |
| 12 | `HighYellow` | 24 | 56 | 0 | |
| 13 | `TopYellow` | 16 | 32 | 0 | |

Values are stored **raw** as written in the INI. Note the G channel is
6-bit (0..63) while R and B are 5-bit (0..31); matches the RGB565
palette-remap convention used elsewhere in the engine. The consumer must
apply the same scaling when blending.

Layout in memory:

```
RulesClass + 0x1874 : slot 0 R
RulesClass + 0x1875 : slot 0 G
RulesClass + 0x1876 : slot 0 B
RulesClass + 0x1877 : slot 1 R
...
RulesClass + 0x189D : slot 13 B
RulesClass + 0x189E .. +0x18A3 : 6 bytes (slots 14–15), zeroed by the
                                 outer `RulesClass::Process` before
                                 `ReadColorAdd` runs; a mod extending
                                 `[ColorAdd]` to 16 entries would fill
                                 them.
```

`0x189E = 0x1874 + 14*3`. The physical allocation is 16 slots (`48 B`,
through `0x18A4`) — the outer orchestrator (`RulesClass::Process` at
`0x006686C0`, step 4) zero-fills all 48 bytes before `ReadColorAdd` is
called, so slots 14–15 read as `(0,0,0)` rather than garbage when stock
YR uses only the first 14.

## Consumers — **unresolved by this investigation**

Direct xrefs from the RulesClass singleton (`g_RulesClass_Instance @
0x008871E0`) through offset `0x1874` are not surfaced by
`get_field_access_context` because the singleton is dynamically allocated
(fixed only via the pointer at `0x008871E0`, not at a known struct-base
address). Locating consumers requires scanning for the two-step access
pattern `MOV reg, [0x008871E0]; ... + 0x1874 + index*3` across all
functions, which is out of scope for this deliverable.

Candidate consumers (educated guess, **not verified**):
- Iron Curtain flash colouring (red tint)
- Psychic Dominator / Yuri mind-control tint (purple)
- Chrono warp animations (white variants)
- Unit-damage health-bar colour blending

No `ColorAdd=<name>` key was found in `ini/rulesmd.ini` or
`ini/artmd.ini`, so consumers index the table **by hard-coded slot
number**, not by key name. Determining the per-consumer slot usage is a
follow-up task.

## YR-active status — **live**

The table is populated on every game start (`RulesClass::Process`
dispatcher step that invokes `FUN_0066D480`). Read side is definitely
active in stock YR because the ColorAdd section *is* present and filled
in `rulesmd.ini` — but pending consumer verification, it is possible one
or two slots (e.g. `TopYellow`, `HighYellow`) are TS-legacy and never
indexed at runtime. Flag the whole table as `yr_active=yes` with the
caveat that individual slots need consumer-tracing before trusting them
for parity.

## Confidence

- **Reader + offset + slot layout:** HIGH — decomp is short, unambiguous,
  and cross-checked against the 14 stock entries.
- **Slot count:** HIGH for stock YR (14). The "16-entry" claim from the
  source report is NOT supported by the binary — the reader writes
  exactly `Entry_Count(section)` slots. The trailing 6 bytes before
  `0x18A4` are ctor-default padding unless a mod extends the section.
- **Consumer semantics / per-slot usage:** LOW. Requires a separate
  xref-scanning pass on `[g_RulesClass_Instance] + 0x1874`.

## Cross-refs

- Existing layout table in `RULESCLASS_GHIDRA_REPORT.md` §2 — update in
  Task 14 to remove the "16-entry" wording and replace with the
  "INI-driven, 14 slots in stock YR" formulation.
- `ADDRESS_MAP.md` — add `FUN_0066D480` → `RulesClass::ReadColorAdd` in
  Task 14.
