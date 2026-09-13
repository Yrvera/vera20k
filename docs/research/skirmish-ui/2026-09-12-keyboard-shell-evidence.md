# Keyboard A3 research — pending integration

Research and bounded prerequisite implementation, 2026-09-12. Shell integration,
native label changes, runtime acceptance and whole-shell parity remain pending. Read the original
`gamemd.exe`, SHA256
`1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c`,
through the pinned image loader/Capstone and read-only Ghidra. Prior names and
reports were leads; notably the old “sole caller” comment on 533D20 is false:
this dialog calls it directly for Cancel and Reset All.

## Active owner and result boundaries

- 5FBEF0 supplies ECX=A3, EDX=5FB320 and zero creation argument at
  5FBEF2..5FBF06 to the shared 622650 dialog creator. It registers a result
  pointer at HWND+8, services 623120 while negative, then unregisters/destroys
  through 622720. The callback lacks a Ghidra function boundary; its original
  bytes 5FB320..5FBEE6 were disassembled directly, without creating a function.
- Launcher owner 55FC80 handles its D5 result5CE by calling 5FBEF0 and reopening
  Options. Active BBB callback4E1FE0 handles 52C/notification0 by setting
  game-state4 and its own result1. Dispatcher48CA03 calls5FBEF0, then writes
  state5 at48CA0D for BBB. Both routes use A3; there is no separate in-game
  Keyboard template.
- Back686/notification0: 5FB900..5FB9D8 constructs a fresh INI, walks the
  current binding table, writes `[Hotkey]` using command vtable+4 names and
  encoded integer keys (5275C0), writes `KeyboardMD.ini` (8333AC string,
  474430), then sets result1. No explicit write-error prompt is present here.
- IDCANCEL2/notification0: 5FB6D4..5FB6EF calls533D20, then writes result2.
  Thus cancellation reloads the currently available file; it is not a cloned
  entry snapshot. A3 has no visible Cancel button and no ID1 handler.
- Ordinary Escape cancellation is established by the full shared route:
  622650 creates withCreateDialogIndirectParamA and registers through5D4E70;
  5FBEF0 services623120, which calls5D4D50; that body iterates registered
  dialogs withIsDialogMessageA before normal dispatch (5D4DDB..5D4DEE).
  Windows translates Escape toWM_COMMAND/IDCANCEL2, which A3 handles above.
  Thus ordinary Escape reloads saved bindings and returns to its parent.
  Enter has no global Assign/Back alias. An open combo dropdown may consume
  Escape to close the dropdown first; test that focused journey separately.
  Sources: [Microsoft dialog keyboard interface](https://learn.microsoft.com/en-us/windows/win32/dlgbox/dlgbox-programming-considerations)
  and [IsDialogMessageA](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-isdialogmessagea).
- ResetAll4D0/notification0: 5FBA05..5FBA41 constructs a file for
  `KEYBOARDMD.INI` (827B88), calls65D190 to delete the loose override, calls
  533D20 to reload through the normal asset path, and sends467 to rebuild the
  categories. No confirmation is called. This changes disk immediately, so a
  later Cancel reloads defaults, not the deleted customization.

## Exact original resource

PE RT_DIALOG A3/language1033 is **DLGTEMPLATEEX**, version1/signatureFFFF,
at BEF560, 1134 bytes, SHA256
`10c5890ce31a2365828aba7e3beb11f331c4769fa2dc054b1074a084f940c925`.
Header style40000040, exstyle0, 19 children, bounds0,0,533,369 DLU;
font8pt MS Sans Serif, weight0/italic0/charset1. The existing ordinary
`saved_game_layout.resource` parser rejects this template correctly: an
extended resource reader is used by the new `keyboard_shell_layout` fixture.

| ID | Class | Style | Caption | DLU x,y,w,h |
|---|---|---|---|---|
|686|Button|5000000B|GUI:Back|425,346,108,23|
|694|Static|50020001|GUI:KeyboardOptions|425,1,108,10|
|FFFFFFFF|Static|50000200|GUI:Category|63,68,146,9|
|4C7|ComboBox|50000313|empty|63,83,138,146|
|FFFFFFFF|Static|50000200|GUI:Commands|224,68,146,8|
|4C8|ListBox|50000153|empty|224,82,136,110|
|FFFFFFFF|Static|50000200|GUI:PressShortcut|64,233,124,9|
|6E9|Button/group|50000007|GUI:Description|63,98,138,95|
|4CB|Static|50000000|empty|70,109,127,42|
|4CD|msctls_hotkey32|50800000|HotKey1|64,245,124,14|
|4CF|Button|5000000B|GUI:Assign|224,245,83,15|
|FFFFFFFF|Static|50000201|GUI:CustomizeKeyboard|69,45,292,11|
|FFFFFFFF|Static|50000200|GUI:CurAssignedTo|63,266,134,11|
|4D0|Button|5000000B|GUI:ResetAll|224,278,83,15|
|63B|Static|50000200|GUI:Blank|63,278,138,10|
|63C|Static|50000200|GUI:Blank|63,215,138,10|
|FFFFFFFF|Static|50000200|GUI:CurrentShortcut|63,203,138,11|
|63D|Static|50000000|GUI:Blank|224,203,146,22|
|695|Static|50000200|GUI:Blank|2,355,303,12|

There is **no separate Unassign button**. The combo's146-DLU height includes
its dropdown extent, not a permanently visible category list of that height.

## Initialization, selection and assignment

- Custom init497 sends467 (5FBE1C..26). Handler467 resets4C7, walks the
  command registry87F65C/count87F668, calls each command's vtable+0C localized
  category getter, uses4BE to search and4C2 to add absent categories, selects
  combo index0 with14E, resets cached category832D8C to−1 and sends464.
  Read actual combo insertion handler617250 before declaring final category
  sorting; resource includes CBS_SORT.
- Category change4C7/CBN_SELCHANGE1 sends464 (5FB6A9..6BF). Handler464 reads
  selected category and its wide caption (64-wide buffer), resets command
  list4C8, walks the registry in native registration order, compares localized
  categories using7DD0F8, inserts matching localized command names from
  vtable+8 with4CD, and stores command pointers usingLB_SETITEMDATA19A.
  Description/current shortcut/current owner are cleared. At5FB60E..18 the
  apparent186 “select0” is sent to **63B**, the static current-owner control,
  not4C8. Do not silently fix this apparent native mistake or assume command0
  is initially selected; first-selection behavior needs the list handler check.
- Command selection4C8/LBN_SELCHANGE1 sends465, then focuses4CD
  (5FB663..694). Handler465 reads list selection188/itemdata199; absent row
  returns. It displays the command's vtable+10 description in4CB. It scans
  key-sorted table87F680/count87F684 for the selected command, formats its
  first binding into63C, clears the hot-key control viaHKM_SETHOTKEY401=0,
  and clears63B. A selected command plus this empty capture is the explicit
  ordinary path to unassign.
- Capture4CD/EN_CHANGE300 clears63D, retrievesHKM_GETHOTKEY402, converts
  HOTKEYF_SHIFT/CONTROL/ALT1/2/4 into game bits100/200/400 plus low-byte VK.
  HOTKEYF_EXT8 forces encoded zero (5FB73A..76C). It looks up that exact key
  and displays its currently assigned command's vtable+8 caption in63B or
  blank; it does not mutate bindings yet.
- Assign4CF sends466 then465 (5FBA86..BAA2), without a notification-code
  check in this branch. Handler466 requires a selected command, obtains the
  same encoded capture and calls48BB40 then48BB60. Rejection writes localized
  `Error:CannotMap` (833360) or `Error:CannotRemap` (83334C) into63D, without
  a popup. These are modifier-policy checks, not a general collision prompt.
- 48BB40 rejects a modified mapping if the **selected command**'s
  AcceptsModifiers(+14) accepts that modifier combination;48BB60 checks the
  command at the unmodified base key similarly. This preserves native
  two-pass gameplay lookup precedence. Existing Rust accepts_base_modifiers
  already contains the relevant ordinary policies but is currently private.
- Successful466 removes only the selected command's first binding in ascending key order
  (5FBBAC..5FBCC7). Encoded0 then returns: unassigned. Otherwise it removes
  any existing binding at the requested encoded key (5FBCCF..5FBD92) and
  inserts the new key/selected command pair (5FBDC0..5FBDEA). A normal
  conflict is therefore reassigned immediately with no confirmation. The
  following465 refreshes Current Shortcut and clears the capture/current
  owner. These are live table changes, persisted only on Back.

## Control capture, paint and placement

- Original hot-key class registration in60F9A0 selects61ECA0 at60FEEA.
  The61ECA0 body intercepts paint/erase/nonclient paint; other messages call
  the saved original child procedure at61ED2A..3D. It is a hot-key capture
  control, not the614B30 text editor used by saved-file descriptions.
- Paint retrieves402 at61EEA1..AB and formats with61EF70. It restores cached
  background, calls6208F0 with border mode2, and uses an inset4 rectangle,
  font context+64, colorAC18A4, flags24 at61EF01..4A. Display formatter61EF70
  joins Alt, Ctrl, Shift, base key in that order usingMapVirtualKeyA and
  GetKeyNameTextA; zero/ext-key display handling follows its actual input.
- Windows documentation establishes EN_CHANGE, default focus acquisition,
  modifier handling and default cleared rules. It delegates Enter/Tab/Space/
  Delete/Escape/Backspace keydown toDefWindowProc. This is **not proof that
  Backspace clears the field**. Do not invent a text-editing shortcut. See
  [Microsoft hot-key control processing](https://learn.microsoft.com/en-us/windows/win32/controls/hot-key-controls)
  and [HKM_SETRULES](https://learn.microsoft.com/en-us/windows/win32/controls/hkm-setrules).
- A3 is admitted by common mode1 initializer60C540 and ordinary layout
  dispatcher60C0C0.608500 has no special A3 rectangle.608CD0 identifies
  title694;609730 identifies Back686;60C0C0 identifies footer695 and enrolls
  remaining A3 controls in60B7A0. Reuse the already verified signed ordinary
  offsets(−80,−60)/(0,0)/(112,84) for640/800/1024, with supplied6x13 DLU
  conversion; title/Back/footer use their existing special helpers. This is
  resource/call-path evidence, not a new A3 pixel oracle or live capture.
- 609EBD..609ED5 identifies4CF/4D0 for type3 MNBTTN via60A532..53F.
  Back follows the existing active SIDEBTTN versus launcher button path.
  Reuse the shared active InGameShellFrame and launcher frame selection;
  A3 background mode is shared, not another bespoke panel.
- Statics use their resource low alignment bits: centered instruction/title,
  ordinary left captions/values;4CB and63D allow multiline content. Group6E9
  needs the shared group-box renderer, not a clickable button. The existing
  standard list and combo owner procedures are618D40 and617250. Their detailed
  dropdown, row sorting and initial selection behavior remain to be verified
  for this screen before renderer acceptance.
- Footer6048B1..604960 maps:4C7 STT:KeyboardComboCategory;6E9
  STT:KeyboardGroupDescription;4C8 STT:KeyboardListCommands;63C
  STT:KeyboardLabelShortcut;63D STT:KeyboardLabelError;4CD
  STT:KeyboardEditEntry;4CF STT:KeyboardButtonAssign;63B
  STT:KeyboardLabelAssigned;4D0 STT:KeyboardButtonResetAll;686
  STT:KeyboardButtonBack. Use existing common footer ownership.

## Rust implementation boundary and acceptance

`src/app/input/hotkeys.rs` owns `HotkeyBindings` and its private
`BTreeMap<u16, HotkeyCommand>`. `load` reads only AssetManager's KEYBOARDMD.INI;
`from_ini_bytes` parsesHotkey then always installsDelete/Escape/Space.
`src/app/initialize.rs` loads it once; input/state.rs stores it. There is no
editing, disk reload/reset/write or command display catalog. Launcher boundary
`src/app/shell_main_menu.rs::route_keyboard` currently logs unimplementedA3.
BBB control identity is already inui/shell/in_game_options.rs.

Proposed coherent design: one editable command/binding authority used by both
shell entry points and existing gameplay dispatch; a distinct A3 view state
for category/row/capture/error/focus; shared list/combo/group/frame/button
paint. Persist by canonical command identity, with localized name/category/
description metadata. Do not derive the visible catalog from only nonzero INI
bindings: native iterates all registered commands, including unbound commands.
`HealthNav` and `CursorPosition` are visible registry leads missing from the
current Rust semantic enum; their exact metadata and reachable behavior need
fresh completion. Prior HOTKEY_SYSTEM report's catalog is only a lead.

Separate startup forced bindings from533D20 reload: native533D20 does not add
Delete/Escape/Space;532150 adds them afterwards only in registration. Reusing
currentRust `from_ini_bytes` blindly for Cancel/Reset would change behavior.
Original `langmd.mix` contains KEYBOARDMD.INI,1869bytes,SHA256
`26c4f8952f7d926e8eb3ecf3bdd3f9db47306d268b7afee620e7a2ed15c43652`.
Fresh direct archive extraction confirms ordinary defaults such as
CenterView12,Options27,CenterOnRadarEvent32,DeployObject68,StopObject83,
TypeSelect84,PlanningMode90,SidebarDown98,SidebarUp104,Delete110,
ScreenCapture339; numbered teams and taunts are present. It also contains
unregistered legacy/debug names which533D20 skips. In particular native
startup forces Delete46 in addition to the file'sDelete110; reload alone
does not force46. Do not equate the raw INI key count to visible commands.
Disk reset must reveal archive defaults through asset authority; a cached
loose override must not survive the delete in AssetManager. Preserve native
case-sensitive command identities and existing modifier-sensitive dispatch.

Acceptance for the increment: both entry routes render A3 at640/800/1024;
category changes populate the actual stock command catalog and clear dependent
fields; selecting a command shows its description/current binding and focuses
capture; entering an ordinary key/chord shows its current owner; Assign handles
free key, collision reassignment, modifier rejection and empty unassignment;
Back writes/reopen/restart preserve assignments; Cancel reloads saved bindings;
Reset All restores archive defaults immediately and survives later Cancel;
resumed gameplay invokes the edited semantic command with no shell-key leakage.

Remaining bounded evidence before integration: exact stock registered metadata
and ordering/first selection, and initial/capture focused keyboard handling.
A3 has a real IDCANCEL handler,
so the earlier B8/B6 “Escape stays because ID2 is ignored” proof does not apply.
No full native UI capture or tests were run in this research pass.

## Follow-up: complete ordinary registry and localized metadata

The original registration body532150 was executed to the entry of533D20 in
Unicorn with debug flagA8B8B4=0, a supplied writable registry array of128slots,
and a bounded bump allocator for original malloc7C8E17. Original constructors
and registry insertion code executed; no registration or getter instruction
was patched. It produced **87 objects**, covering44 distinct vtables. Execution
stopped before INI I/O and startup's forced bindings. The getter metadata below
was then read from the original vtable+4/+8/+C/+10 instruction operands.
Parameterized constructors supply1..10 for each team family and1..8 for
taunts. Thus this is exact native registration order under those stated
preconditions; visible rows are separately sorted by the Windows controls.

Direct original langmd.mix/ra2md.csf input:573269bytes,SHA256
`f3382231d7eb3bd713bf3908ecdba175a1b2e20d9d80fa2702e9ab84c3a79deb`.
All listed name/category/description keys exist in that file. Use
`CsfFile::text` and `format_csf` for localized metadata; parameterized display
names use **%2d** while canonical INI names use%d. Preserve the leading space
for single-digit team/taunt captions. The native wide formatter is7CA564.

| Registry index | Vtable | INI name or format | Display CSF key | Category CSF key | Description CSF key |
|---|---|---|---|---|---|
|0|7EBDC4|Follow|TXT_FOLLOW|TXT_INTERFACE|TXT_FOLLOW_DESC|
|1|7EBD9C|View1|TXT_VIEW_BOOKMARK1|TXT_INTERFACE|TXT_VIEW_BOOKMARK1_DESC|
|2|7EBD74|View2|TXT_VIEW_BOOKMARK2|TXT_INTERFACE|TXT_VIEW_BOOKMARK2_DESC|
|3|7EBD4C|View3|TXT_VIEW_BOOKMARK3|TXT_INTERFACE|TXT_VIEW_BOOKMARK3_DESC|
|4|7EBD24|View4|TXT_VIEW_BOOKMARK4|TXT_INTERFACE|TXT_VIEW_BOOKMARK4_DESC|
|5|7EBCFC|SetView1|TXT_SET_BOOKMARK1|TXT_INTERFACE|TXT_SET_BOOKMARK1_DESC|
|6|7EBCD4|SetView2|TXT_SET_BOOKMARK2|TXT_INTERFACE|TXT_SET_BOOKMARK2_DESC|
|7|7EBCAC|SetView3|TXT_SET_BOOKMARK3|TXT_INTERFACE|TXT_SET_BOOKMARK3_DESC|
|8|7EBC84|SetView4|TXT_SET_BOOKMARK4|TXT_INTERFACE|TXT_SET_BOOKMARK4_DESC|
|9|7EBC5C|Options|TXT_OPTIONS|TXT_INTERFACE|TXT_OPTIONS_DESC|
|10|7EBC34|SidebarUp|TXT_SIDEBAR_UP|TXT_INTERFACE|TXT_SIDEBAR_UP_DESC|
|11|7EBC0C|SidebarDown|TXT_SIDEBAR_DOWN|TXT_INTERFACE|TXT_SIDEBAR_DOWN_DESC|
|12|7EBBE4|CenterOnRadarEvent|TXT_RADAR_EVENT|TXT_INTERFACE|TXT_RADAR_EVENT_DESC|
|13|7EBBBC|PlaceBeacon|TXT_PLACE_BEACON|TXT_INTERFACE|TXT_PLACE_BEACON_DESC|
|14|7EBB94|ToggleSell|TXT_SELL_MODE|TXT_INTERFACE|TXT_SELL_MODE_DESC|
|15|7EBB6C|ToggleRepair|TXT_REPAIR_MODE|TXT_INTERFACE|TXT_REPAIR_MODE_DESC|
|16|7EBB44|ToggleAlliance|TXT_ALLIANCE|TXT_CONTROL|TXT_ALLIANCE_DESC|
|17|7EBB1C|CenterBase|TXT_CENTER_BASE|TXT_SELECTION|TXT_CENTER_BASE_DESC|
|18|7EBAF4|CenterView|TXT_CENTER_VIEW|TXT_SELECTION|TXT_CENTER_VIEW_DESC|
|19|7EBACC|ScatterObject|TXT_SCATTER|TXT_CONTROL|TXT_SCATTER_DESC|
|20|7EBAA4|GuardObject|TXT_GUARD|TXT_CONTROL|TXT_GUARD_DESC|
|21|7EBA7C|StopObject|TXT_STOP_OBJECT|TXT_CONTROL|TXT_STOP_OBJECT_DESC|
|22|7EBA54|AllToCheer|Cmnd:AllCheer|TXT_CONTROL|Cmnd:AllCheerDesc|
|23|7EBA2C|DeployObject|TXT_DEPLOY_OBJECT|TXT_CONTROL|TXT_DEPLOY_OBJECT_DESC|
|24|7EBA04|PreviousObject|TXT_PREV_OBJECT|TXT_SELECTION|TXT_PREV_OBJECT_DESC|
|25|7EB9DC|NextObject|TXT_NEXT_OBJECT|TXT_SELECTION|TXT_NEXT_OBJECT_DESC|
|26|7EB9B4|PlanningMode|Cmnd:PlanningMode|TXT_CONTROL|Cmnd:PlanningModeDesc|
|27|7EB98C|CombatantSelect|Cmnd:CombatantSelect|TXT_SELECTION|Cmnd:CombatantSelectDesc|
|28|7EB964|TypeSelect|Cmnd:TypeSelect|TXT_SELECTION|Cmnd:TypeSelectDesc|
|29|7EB93C|HealthNav|Cmnd:HealthNavigation|TXT_SELECTION|Cmnd:HealthNavigationDesc|
|30|7EB914|VeterancyNav|Cmnd:VetNavigation|TXT_SELECTION|Cmnd:VetNavigationDesc|
|31|7EB8EC|StructureTab|TXT_STRUCTURE_TAB|TXT_INTERFACE|TXT_STRUCTURE_TAB_DESC|
|32|7EB8C4|DefenseTab|TXT_DEFENSE_TAB|TXT_INTERFACE|TXT_DEFENSE_TAB_DESC|
|33|7EB89C|UnitTab|TXT_UNIT_TAB|TXT_INTERFACE|TXT_UNIT_TAB_DESC|
|34|7EB874|InfantryTab|TXT_INFANTRY_TAB|TXT_INTERFACE|TXT_INFANTRY_TAB_DESC|
|35–44|7EB84C|TeamCreate_%d|TXT_CREATE_TEAM|TXT_TEAM|TXT_CREATE_TEAM_DESC|
|45–54|7EBE64|TeamSelect_%d|TXT_SELECT_TEAM|TXT_TEAM|TXT_SELECT_TEAM_DESC|
|55–64|7EBE8C|TeamAddSelect_%d|TXT_ADD_SELECT_TEAM|TXT_TEAM|TXT_ADD_SELECT_TEAM_DESC|
|65–74|7EBEB4|TeamCenter_%d|TXT_CENTER_TEAM|TXT_TEAM|TXT_CENTER_TEAM_DESC|
|75–82|7EBEDC|Taunt_%d|TXT_TAUNT_NUMBER|TXT_TAUNT|TXT_TAUNT_DESC|
|83|7EBF04|ScreenCapture|TXT_SCRNCAP|TXT_INTERFACE|TXT_SCRNCAP_DESC|
|84|7EBF2C|PageUser|TXT_PAGEUSER|TXT_INTERFACE|TXT_PAGEUSER_DESC|
|85|7EBF54|CursorCheat|GUI:CursorCheat|GUI:Debug|DESC:CursorCheat|
|86|7EBF7C|Delete|Cmnd:Delete|TXT_INTERFACE|Cmnd:DeleteDesc|

The exact missing ordinary identities are **HealthNav** and **CursorCheat**.
The latter class was calledCursorPosition in older notes; its actual GetName
isCursorCheat. English entries are Health Navigation / Selection and
Coordinates / Information. HealthNav description is “Navigates across the
last selection by health level.” Coordinates description is “Displays
coordinates at the cursor.” CursorCheat is registered unconditionally even
though its category CSF key isGUI:Debug; do not hide it as a developer-only
command because of that key's spelling. Debugflag gates only the preceding
two MultiplayerDebug/MultiplayerSync objects in this bounded registry.

### Initial selection and ordinary sorted order

Fresh617250 body4C2 converts the wide caption to the native low-byte narrow
form and sendsCB_ADDSTRING143 to the original control
(618705..618721). A3 style50000313 includesCBS_SORT andCBS_HASSTRINGS.
Fresh618D40 body4CD similarly sendsLB_ADDSTRING180 after narrowing
(61B4FC..61B516);50000153 includesLBS_SORT andLBS_HASSTRINGS.
Neither common60F760 nor60F9A0 clears these sort/string style bits.
Their returned insertion indices preserve the correct per-row command
identity through the custom string metadata plus19A.

Therefore the ordinary English categories are **Control, Information,
Interface, Selection, Taunt, Team**, withControl selected initially by the
callback'sCB_SETCURSEL0. WithinControl the visible English order is
**Alliance, Cheer, Deploy Object, Guard, Scatter, Stop Object, Waypoints**.
These category/row order conclusions combine original message/style evidence,
original English CSF bytes and documented Windows sorting; they are not a
captured native pixel list. Other locales need locale-aware Windows-equivalent
collation, not sorting canonical INI names. Sources:
[CB_ADDSTRING](https://learn.microsoft.com/en-us/windows/win32/controls/cb-addstring)
and [LB_ADDSTRING](https://learn.microsoft.com/en-us/windows/win32/controls/lb-addstring).

No command row is selected on initialization or category change.
618D40 reset184 clearsselectioncontext+F4 to−1 at619F34, and sends the
parentLBN_SELCHANGE; insertion4CD does not select a row. Init497 also sets
selection−1 at61ACF1. As established above, the callback's final186 goes to
static63B, so its follow-up465 sees−1 and leaves current shortcut/description
blank. Reset184's selection notification can ask the callback to focus4CD
even though no row is selected (5FB663..694); selecting a command explicitly
does focus the capture. This explains why blank details are correct rather
than evidence of a failed first-row selection.

### Focused capture and ordinary keys

Common wrapper610CA0 restores prior focus onWM_SETFOCUS only for the exact
procedures612B70(button) and618D40(list), at6118C4..6118DE. It does **not**
apply that rejection to hot-key owner61ECA0 or combo617250. Capture accepts
focus and delegates normal key handling to the real Windows hot-key class.
The modal routing guard in610CA0 admits descendants of the top modal; capture
keystrokes must not reach gameplay or launcher shortcuts.

61ECA0 intercepts onlypaint0F/erase14/nonclientpaint85 and delegates the rest
at61ED2A..61ED3D. A3 does not sendHKM_SETRULES403; Windows creates the hot-key
control with cleared rules. Ordinary letters/F-keys and modifier combinations
are captured through that original control and triggerEN_CHANGE300.
Microsoft's documentedDLGC_WANTCHARS|DLGC_WANTARROWS does not demand Enter or
Escape. Hence normal closed-dropdown Escape routesIDCANCEL and reloads,
while Enter does not becomeAssign. Tab follows dialog focus rules; A3's
resource contains noWS_TABSTOP children, and custom buttons/lists reject
focus, so do not invent a tab cycle across Assign/Reset/Back or focusedSpace
activation for those buttons. Click a command or the capture to edit.
An open custom category dropdown owns its own interaction and must be allowed
to close without interpreting that input as a parent action. No ordinary
Backspace/Delete-to-unassign shortcut is established: the direct proven
unassign journey is selectcommand (capturecleared) thenAssign.

### Editable owner and actual Rust consumers

Integration now keeps the existing process-retained HotkeyBindings field in
match_state.input: startup creates it once, A3 mutates it and handler dispatch
reads it. Match replacement preserves bindings and the CursorCheat toggle while
resetting selection navigation. Catalog names, metadata and parser lookup have
one87-command authority. A3 has no duplicate binding map.

Platform live modifiers are separate from the gameplay paused-input snapshot;
A3 capture precedes global bindings/Escape dispatch. Current-file reload reads
disk before registered archive fallback, bypassing immutable startup loose bytes.
HealthNav and CursorCheat now have dedicated gameplay integrations described in
[keyboard command evidence](2026-09-12-keyboard-command-evidence.md). Existing
VeterancyNav, PreviousObject, NextObject, CombatantSelect and PlanningMode remain
required whole-scope execution gaps, not certified by editor persistence.

Terminal5FBF37 setsresult2 and destroys A3 without running IDCANCEL's533D20 reload.
The synchronous55FD1E caller next recreatesD5 at55FCA0, whose final transaction
applies55FAA0 and writes5FAD10. The app preserves its existing OS-close-to-terminal
adapter by removing A3 without binding IO and completing the suspended launcher
parent once. This establishes coherent app lifecycle, not native WM_CLOSE parity:
623120's game-termination return is broader than an OS message-pump exit.

### Reproduce the bounded stock metadata extraction

Run the following Python from the owned checkout with VERA20K_GAMEMD_EXE set
to the pinned original. It performs no file writes. This preserves the exact
bounded registration experiment and original input extraction for continuation;
it is not a production oracle/test or a full callable Windows dialog.

```python
from unicorn import *
from unicorn.x86_const import *
from tools.native_oracle import *
from capstone import *
import struct,json
u=Uc(UC_ARCH_X86,UC_MODE_32);load_image(u)
u.mem_map(STACK_BASE,STACK_SIZE);u.mem_map(SCRATCH,0x100000)
def put(a,v):u.mem_write(a,struct.pack('<I',v))
def get(a):return struct.unpack('<I',u.mem_read(a,4))[0]
heap=SCRATCH+0x1000
def hook(uc,a,n,d):
 global heap
 if a==0x7c8e17:
  sp=uc.reg_read(UC_X86_REG_ESP);size=get(sp+4);p=heap;heap+=(size+15)&~15;uc.reg_write(UC_X86_REG_EAX,p);uc.reg_write(UC_X86_REG_ESP,sp+4);uc.reg_write(UC_X86_REG_EIP,get(sp))
u.hook_add(UC_HOOK_CODE,hook)
put(0x87f65c,SCRATCH);put(0x87f660,128);put(0x87f668,0)
sp=STACK_BASE+STACK_SIZE-0x1000;u.reg_write(UC_X86_REG_ESP,sp)
run_checked(u,0x532150,0x533d20,count=100000)

def asc(a):return bytes(u.mem_read(a,200)).split(b'\0')[0].decode('cp1252')
cs=Cs(CS_ARCH_X86,CS_MODE_32);seen=set();rows=[]
for i in range(get(0x87f668)):
 obj=get(SCRATCH+i*4);vt=get(obj)
 if vt in seen:continue
 seen.add(vt);keys=[]
 for slot in [4,8,12,16]:
  f=get(vt+slot)
  ins=list(cs.disasm(bytes(u.mem_read(f,60)),f))
  if slot==4:
   lead=ins[0]
   if lead.op_str.startswith('eax, 0x'):a=int(lead.op_str.split(', ')[1],16)
   else:a=next(int(x.op_str,16) for x in ins if x.mnemonic=='push' and x.op_str.startswith('0x82'))
  else:a=next(int(x.op_str.split(', ')[1],16) for x in ins if x.mnemonic=='mov' and x.op_str.startswith('ecx, 0x82'))
  keys.append(asc(a))
 rows.append((i,vt,keys))

from tools.sidebar_oracle.stock import mix,mix_hash
import hashlib
b=mix((configured_gamemd().parent/'langmd.mix').read_bytes())[mix_hash('ra2md.csf')]
o=24;csf={}
while b[o:o+4]==b' LBL':
 n,z=struct.unpack_from('<II',b,o+4);o+=12;k=b[o:o+z].decode().upper();o+=z
 for j in range(n):
  tag=b[o:o+4];z=struct.unpack_from('<I',b,o+4)[0];o+=8;t=bytes(x^255 for x in b[o:o+z*2]).decode('utf-16-le');o+=z*2
  if tag==b'WRTS':z=struct.unpack_from('<I',b,o)[0];o+=4+z
  csf[k]=t
for i,vt,keys in rows:print(i,keys[0],repr(csf.get(keys[1].upper())),repr(csf.get(keys[2].upper())),repr(csf.get(keys[3].upper())))
```

Research remains pending integration. The ordinary catalog, firstselection and
capture/delegation boundary above now supersede those specific earlier open
items. A focused native UI check remains useful for initial caret timing and
custom dropdown dismissal; this pass does not certify complete tab traversal
or non-English collation.

## Preserved executable comparisons and bounded Rust prerequisite

`tools/storage_oracle/keyboard_bindings.py` now preserves original532150
registration through entry533D20, stopping before file I/O, with only bounded
malloc storage supplied. Its87 registered objects produce the full metadata
catalog, including original getters/CSF keys and parameterized command names.
The original assignment checks5FBB38 and mutation5FBBAC execute for73 supplied,
already sorted ordinary tables. The preserved `.json` and `.meta.json` record
binary identity, substitutions and limits; default read-only replay passed.

The duplicate-binding cases independently settle a visible startup interaction:
Delete has keys46 and110. Unassign removes46 and retains110. Assign88 removes46
and retains both88 and110. This is first-match removal, not clearing every
shortcut of the selected command. The current-shortcut label similarly stops at
its first object match (5FB408..5FB44A). Normal target-key collisions replace that
key's prior owner without removing its other keys. Modifier failures preserve
the entire table. All8 modifier combinations are exercised for ordinary,
Shift-sensitive and any-modifier command policies, including HealthNav.

`HotkeyBindings` remains the sole mutable key-to-command store. The new methods
`command_at`, `first_key`, `assign`, `reload_from_ini_bytes`, and `to_ini_string`
support the editor without a second binding table. `catalog::registered_commands`
is immutable registered metadata; HealthNav and CursorCheat now have command
variants. Existing gameplay resolution remains the same two-pass operation.
The assignment predicate is deliberately the exact native virtual predicate;
the existing dispatch convenience defaults to admitting bare keys and must not
be reused as the native assignment predicate.

Startup still reloads and then forces Delete46, Options27 and CenterOnRadarEvent32.
This was rechecked in original533C43..533D1D after the533D20 call; dialog reload
has no such forcing. Back's fresh INI serialization follows ascending keys and
later values overwrite repeated command names, so the highest duplicate binding
survives a disk round trip. Rust regression tests cover this distinction and
compare all87 catalog rows plus all73 original assignment results. Rust test
execution belongs to the integrating owner; it was not run by this evidence pass.

`tools/storage_oracle/keyboard_shell_layout.py` locally decodes the original A3
DLGTEMPLATEEX, retaining all19 children and their32-bit IDs. It executes full
60B7A0 for16 ordinary controls at640x480/800x600/1024x768:48 cases. Default
read-only replay passed. Match repeated anonymous static IDs by `resource_index`.
Supplied6x13 dialog font base units and the existing geometry API substitutions
are explicit; title/Back/footer, dropdown collapsed height, owner-paint artwork,
focus, file I/O, GUI event admission and non-English collation are outside these
executable comparisons. Shell integration and independent criticism remain open.

The bounded parser refactor now uses the immutable registered catalog as the
single canonical command-name authority. Original533DE4..533E29 obtains each
registered object's vtable+4 name and compares bytes at533DF4..533E1D; it does
not parse numeric suffixes or fold case. Former Rust aliases such as TeamSelect_0,
View01 and SetView+1 are therefore rejected. The registered TeamSelect_10 still
maps to engine team slot0. Added tests reload and serialize all87 preserved
native names, and reject seven former numeric aliases. Original comparison
instructions were also executed read-only for those seven mismatches and seven
matching canonical controls. This last bounded run was an ephemeral check;
the persistent catalog/assignment fixtures retain the registered-name evidence.
The modifier-event ownership and existing dispatch route were preserved.
