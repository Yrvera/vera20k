# FUN_004E5700 - Start-Position CBN_SELCHANGE Handler

## Summary

WM_COMMAND CBN_SELCHANGE handler for start-position combo controls. When the
user changes a start-position combo selection, this function (1) releases the
old start-position claim from the assignment table, (2) reads the new selection
via CB_GETCURSEL + CB_GETITEMDATA and writes the new claim, then (3) refreshes
all 8 start-position combos.

Structurally parallel to FUN_004E5480 (task 41) but simpler: instead of
scanning items for a target value, it reads the current selection directly.

## Address

0x004E5700 (verified via decompile_function 0x004E5700)

## Active in YR

Yes. Primary in-scope caller is FUN_006ACEE0 (0x006ACEE0, WM_COMMAND dispatcher,
task 2, YR-active). Out-of-scope callers also exist.
(Confirmed via get_function_callers 0x004E5700)

## Signature / Parameters

void __fastcall FUN_004e5700(HWND param_1, int param_2)
  param_1 = dialog 0x102 HWND
  param_2 = start-position combo control ID (0x6A3..0x6A8, 0x6AA, 0x6AB)

(verified via decompile_function 0x004E5700)

## Control-ID to Slot-Index Mapping

Same mapping as FUN_004E4E60 (task 36) and FUN_004E5480 (task 41):

  0x6A3 -> slot 0
  0x6A4 -> slot 1
  0x6A5 -> slot 2
  0x6A6 -> slot 3
  0x6A7 -> slot 4
  0x6A8 -> slot 5
  0x6AA -> slot 6   (note: 0x6A9 is skipped)
  0x6AB -> slot 7

## Behavioral Analysis

### Phase 1 - Release old start-position claim

Map param_2 to slot index iVar1. Walk DAT_008B3F38 (start-position assignment
table, 9 entries, stride 3 ints, upper bound 0x8B3FA4) to find the entry owned
by this slot index and reset it to 0xFFFFFFFF (-1 = unclaimed).

(verified via decompile_function 0x004E5700)

### Phase 2 - Read selection and claim new start position

wParam = SendDlgItemMessageA(param_1, param_2, 0x147, 0, 0);  // CB_GETCURSEL
LVar2  = SendDlgItemMessageA(param_1, param_2, 0x150, wParam, 0);  // CB_GETITEMDATA

if (LVar2 != -2) {
    // re-map param_2 to iVar1 (same switch as phase 1)
    (&DAT_008B3F38)[LVar2 * 3] = iVar1;  // claim: table[startpos_idx * 3] = slot_idx
}

CB_GETCURSEL returns the currently selected item index; CB_GETITEMDATA returns
the start-position index (0-8) or -2 (random sentinel). If not -2, writes the
slot index into the assignment table at entry LVar2.

(verified via decompile_function 0x004E5700)

### Phase 3 - Refresh all start-position combos

Same 8-row dispatch loop as FUN_004E5480 and FUN_004E49A0:
  if spectator/observer mode AND (slot is absent or closed): FUN_004e5260
  else: FUN_004e50c0

(verified via decompile_function 0x004E5700)

## Difference from FUN_004E5480

FUN_004E5480 (task 41) takes an explicit target item-data value (param_3) and
scans all combo items via CB_GETCOUNT/CB_GETITEMDATA loop to find it. This
function uses CB_GETCURSEL to read the already-selected item -- no scan needed.
FUN_004E5480 is used at init/restore time; this function is the live selection
change handler.

## Globals Accessed

  DAT_008B3F38 (0x008B3F38) - Start-position assignment table; slot_owner read/written
  g_GameMode   (symbolic)   - Mode gate for phase 3 dispatch
  DAT_00A8DA90 (0x00A8DA90) - Player-slot pointer array (phase 3)
  DAT_00AC11B4 (0x00AC11B4) - Null/absent slot sentinel (phase 3)

## Callees

Confirmed via get_function_callees 0x004E5700:
  FUN_004E50C0 (0x004E50C0) - Normal start-pos combo population
  FUN_004E5260 (0x004E5260) - Start-pos sentinel loader
  SendDlgItemMessageA       - Win32

## Callers (in scope)

  FUN_006ACEE0 (0x006ACEE0) - WM_COMMAND dispatcher (task 2)

Out-of-scope: FUN_005E9CE0, FUN_005ED5A0, FUN_005EE6A0.
(Confirmed via get_function_callers 0x004E5700)

## Out-of-scope refs

  FUN_004E50C0 (0x004E50C0) - normal start-pos population; out of scope
  FUN_004E5260 (0x004E5260) - start-pos sentinel loader; out of scope

## Unverified (YELLOW)

  FUN_004E5260 as start-pos sentinel loader: inferred from spectator-mode dispatch
  mirroring FUN_004E4770 (color sentinel); not independently decompiled here.
  FUN_004E50C0 as normal start-pos population: inferred from else-branch dispatch
  mirroring FUN_004E45A0 (color population); not independently decompiled here.
