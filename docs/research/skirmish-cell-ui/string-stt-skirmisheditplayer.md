# String: "STT:SkirmishEditPlayer"

## Summary

Tooltip string key used by the player-name edit control in dialog 0x102. The tooltip
dispatcher (`FUN_006040B0`, task #51) uses this key to look up and display a tooltip
when the cursor hovers over the player-name edit box in slot 0 (the human player row).

## Address

`0x008355C8` (verified via `read_memory 0x008355C8`)

## String Content

`STT:SkirmishEditPlayer`

Confirmed bytes at `0x008355C8`: `53 54 54 3a 53 6b 69 72 6d 69 73 68 45 64 69 74 50 6c 61 79 65 72 00`
(verified via `read_memory 0x008355C8`, length 40)

## Active in YR

Yes. Referenced from `FUN_006040B0 @ 0x006042B1` (tooltip dispatcher, task #51).
(Confirmed via `get_xrefs_to 0x008355C8`)

## Consumer

Sole consumer: `FUN_006040B0` (tooltip dispatcher). This tooltip appears only for
the player-name edit control in slot 0. Slots 1–7 (AI rows) do not have a player-name
edit box and do not use this key.

The `STT:` prefix is the StringTable convention used throughout the skirmish dialog;
`SkirmishEditPlayer` identifies the human-player name input field specifically.

## Dispatcher Position

The xref is at `0x006042B1` inside `FUN_006040B0`. The other slot-row tooltip keys are
referenced at `0x006045AC`, `0x006045A3`, `0x006045B5`, `0x006045BE` — all in the
`0x6045xx` range. The `0x6042B1` address is earlier in the dispatcher's dispatch table,
consistent with slot 0 (human row) controls being handled in a dedicated leading branch
before the 8-row loop that covers slots 1–7.

## Structural Note

All five tooltip string keys used by dialog 0x102 share the `STT:Skirmish` prefix:
- `STT:SkirmishEditPlayer`     — `0x008355C8` — player-name edit (slot 0)
- `STT:SkirmishComboAIPlayer`  — `0x008353E4` — AI-type combo (slots 1–7)
- `STT:SkirmishPictureFlag`    — `0x00835400` — country flag static (all rows)
- `STT:SkirmishComboCountry`   — `0x00835418` — country combo (all rows)
- `STT:SkirmishComboColor`     — `0x00835434` — color combo (all rows)

All five are sole-referenced from `FUN_006040B0`.

## Out-of-scope refs

- `FUN_006040B0` (task #51) — tooltip dispatcher; decoded separately

## Unverified (YELLOW)

- Visible tooltip text: the English string resolved by `STT:SkirmishEditPlayer`
  from `ra2md.csf` / `ra2.csf` has not been looked up in this session; likely
  something like "Enter player name" or "Player Name".
- Exact control ID for the player-name edit: the edit box control ID for slot 0 is
  not decoded in this session (confirmed the tooltip key, not the control ID it
  maps to in the dispatcher).
