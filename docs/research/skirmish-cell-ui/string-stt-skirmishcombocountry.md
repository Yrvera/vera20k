# String: "STT:SkirmishComboCountry"

## Summary

Tooltip string key used by the country combo controls in dialog 0x102. The tooltip
dispatcher (`FUN_006040B0`, task #51) uses this key to look up and display a tooltip
when the cursor hovers over a country combo on any player slot row (rows 0–7, control
IDs 0x6A1, 0x510, 0x513, 0x51E, 0x514, 0x51F, 0x520, 0x521).

## Address

`0x00835418` (verified via `read_memory 0x00835418`)

## String Content

`STT:SkirmishComboCountry`

Confirmed bytes at `0x00835418`: `53 54 54 3a 53 6b 69 72 6d 69 73 68 43 6f 6d 62 6f 43 6f 75 6e 74 72 79 00`
(verified via `read_memory 0x00835418`, length 40)

## Active in YR

Yes. Referenced from `FUN_006040B0 @ 0x006045AC` (tooltip dispatcher, task #51).
(Confirmed via `get_xrefs_to 0x00835418`)

## Consumer

Sole consumer: `FUN_006040B0` (tooltip dispatcher). All 8 country combos (rows 0–7)
share this single tooltip key — the same tooltip text appears regardless of which row
is hovered.

## Out-of-scope refs

- `FUN_006040B0` (task #51) — tooltip dispatcher; decoded separately

## Unverified (YELLOW)

- Visible tooltip text: the English string resolved by `STT:SkirmishComboCountry`
  from `ra2md.csf` / `ra2.csf` has not been looked up in this session; likely
  something like "Select Country" or "Country".
