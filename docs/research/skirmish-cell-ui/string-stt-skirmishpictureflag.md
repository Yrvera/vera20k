# String: "STT:SkirmishPictureFlag"

## Summary

Tooltip string key used by the picture-flag (country-flag image) static controls in
dialog 0x102. The tooltip dispatcher (`FUN_006040B0`, task #51) uses this key to look
up and display a tooltip when the cursor hovers over a flag static in any player slot
row (rows 0–7, control IDs 0x6DA–0x6E1).

## Address

`0x00835400` (verified via `read_memory 0x00835400`)

## String Content

`STT:SkirmishPictureFlag`

Confirmed bytes at `0x00835400`: `53 54 54 3a 53 6b 69 72 6d 69 73 68 50 69 63 74 75 72 65 46 6c 61 67 00`
(verified via `read_memory 0x00835400`, length 40)

## Active in YR

Yes. Referenced from `FUN_006040B0 @ 0x006045B5` (tooltip dispatcher, task #51).
(Confirmed via `get_xrefs_to 0x00835400`)

## Consumer

Sole consumer: `FUN_006040B0` (tooltip dispatcher). The flag statics (control IDs
0x6DA–0x6E1, one per row) display the selected country's flag image. The tooltip
provides a text description on hover. The `STT:` prefix is the StringTable convention;
`SkirmishPictureFlag` identifies the flag control set across all 8 rows.

## Out-of-scope refs

- `FUN_006040B0` (task #51) — tooltip dispatcher; decoded separately
- Flag static control IDs 0x6DA–0x6E1 — decoded in task #18

## Unverified (YELLOW)

- Visible tooltip text: the English string resolved by `STT:SkirmishPictureFlag`
  from `ra2md.csf` / `ra2.csf` has not been looked up in this session; likely
  something like "Country Flag" or "Selected Country".
