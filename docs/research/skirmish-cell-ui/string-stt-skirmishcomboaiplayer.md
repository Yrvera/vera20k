# String: "STT:SkirmishComboAIPlayer"

## Summary

Tooltip string key used by the AI-type combo controls in dialog 0x102. The tooltip
dispatcher (`FUN_006040B0`, task #51) uses this key to look up and display a tooltip
when the cursor hovers over an AI-type combo on slots 1–7.

## Address

`0x008353E4` (verified via `read_memory 0x008353E4`)

## String Content

`STT:SkirmishComboAIPlayer`

Confirmed bytes at `0x008353E4`: `53 54 54 3a 53 6b 69 72 6d 69 73 68 43 6f 6d 62 6f 41 49 50 6c 61 79 65 72 00`
(verified via `read_memory 0x008353E4`, length 40)

## Active in YR

Yes. Referenced from `FUN_006040B0 @ 0x00604548` (tooltip dispatcher, task #51).
(Confirmed via `get_xrefs_to 0x008353E4`)

## Consumer

Sole consumer: `FUN_006040B0` (tooltip dispatcher) — called from the dialog's WM_MOUSEMOVE
or WM_NOTIFY handling path. The dispatcher maps control IDs to tooltip string keys and
calls a StringTable lookup with this key to produce the visible tooltip text.

The `STT:` prefix is the StringTable key convention used throughout the dialog; the
suffix `SkirmishComboAIPlayer` identifies the AI-type player-type combo (slots 1–7 only).
Slot 0 (human player row) does not use an AI-type combo so this tooltip does not
appear there.

## Out-of-scope refs

- `FUN_006040B0` (task #51) — tooltip dispatcher; decoded separately
- StringTable lookup mechanism — out of cell-UI scope; the `STT:` prefix is the
  established convention throughout the skirmish dialog

## Unverified (YELLOW)

- Visible tooltip text: the English string resolved by `STT:SkirmishComboAIPlayer`
  from `ra2md.csf` / `ra2.csf` has not been looked up in this session; likely
  something like "Select AI type" or "AI Player Type".
- Exact tooltip trigger conditions: which WM_ message causes the dispatcher to fire
  is decoded in task #51 and not re-examined here.
