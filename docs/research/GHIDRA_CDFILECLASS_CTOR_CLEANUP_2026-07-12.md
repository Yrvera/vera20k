# Ghidra label cleanup — CDFileClass/CCFileClass__Constructor mislabel cluster

**Date:** 2026-07-12/13 · **Type:** read-only classification swarm + serialized parent rename · **Status:** APPLIED + SAVED

## What this was

A heuristic labeling script had stamped `CDFileClass__Constructor` onto **99 addresses** and
`CCFileClass__Constructor` onto **5** — but most were NOT constructors. The heuristic matched an
*internal idiom* (a function that anywhere in its body constructs a `CDFileClass`/`CCFileClass`
temporary, or sets `vtable__CDFileClass` on a local) rather than verifying the function *is* the
constructor. A genuine ctor is ~21–100 bytes and sets the class vtable on the passed-in `this` at
entry; the mislabels ranged 174 B – 10.8 KB (asset loaders, UI screens, INI parsers, MP session code).

Surfaced by the 2026-07-12 `GHIDRA_PROJECT_HEALTH_AUDIT` (lane 5). This is the biggest single
label-pollution cluster in the project.

## Method (evidence bar)

8 read-only Ghidra lanes classified all 104 addresses (`get_function_by_address` size +
`decompile_function` role, with `get_function_callers`/`search_strings` on borderline cases). Every
proposed name is defended from body evidence recorded in the lane files
(`scratchpad/ghidra-health/ctor-cleanup/lane-*.md`). Parent spot-checked before applying:
`0x00530460` decompiled directly = mounts CONQMD/GENERMD/CAMEOMD/MULTIMD/THEMEMD/MOVMD `.MIX`
archives → `Init_Mix_Files` (and it *calls* the real ctor internally — the exact mislabel mechanism).
Renames applied serially by the single parent writer (scripts were gated off), then `save_program`.

## Outcome

- **11 GENUINE constructors — kept** as `CDFileClass__Constructor` (8: 0x00401950, 0x0047a9d0,
  0x0047aa00, 0x0047aa30, 0x00535a60, 0x00535a70, 0x0069e430, 0x00759510) / `CCFileClass__Constructor`
  (3: 0x004739f0, 0x00473a30, 0x00473a80).
- **91 MISLABELED_KNOWN — renamed** to verified roles (table below).
- **2 MISLABELED_UNKNOWN — reverted** to honest placeholders: 0x00473e50 →
  `CDFileClass__VtableSlot0xE_Unresolved`, 0x00473f00 → `CDFileClass__VtableSlot0xF_Unresolved`
  (verified CDFileClass vtable-slot methods via DATA xref 0x007e16e8/0x007e16ec; exact op unresolved).

Post-cleanup verification: `search_functions CDFileClass__Constructor` returns only the 8 genuine CD
ctors; `CCFileClass__Constructor` only the 3 genuine CC ctors.

## Rename table (address → applied name)

| Addr | Name | Addr | Name |
|---|---|---|---|
| 004019a0 | CDFileClass__ScalarDeletingDestructor | 0065f520 | ScenarioClass__ShowMissionRestateBriefing |
| 0045fa90 | BuildingTypeClass__ResolveTheaterVisualFiles | 006686c0 | ScenarioClass__ResetTypeRegistriesAndReloadRules |
| 0049c0d0 | CoopSaveHeader__Read | 00679d90 | CalcCoopCampaignDataSize |
| 0049c5a0 | CoopSaveHeader__Write | 00679ee0 | CalcArtIniDataSize |
| 0049d120 | CoopSaveEntry__SectionExists | 00687ce0 | Save_Scenario_Map_File |
| 0049d390 | CoopSaveEntry__MapMatchesCurrent | 00690640 | ScoreScreen__LoadNarrationAudio |
| 0049db00 | CoopCampaignDefs__LazyLoad | 00694760 | SessionClass__SendFileToClients |
| 004a38d0 | Load_Alloc_Data | 006951f0 | SessionClass__VerifyRandomMapDigest |
| 004a3970 | Load_Alloc_Data_Named | 00697840 | SessionClass__ClearInternalLists |
| 004b6c30 | Dropship__ShowSequence | 006980c0 | SessionClass__ReadMultiPlayerSettings |
| 004c3e30 | Show_Credits | 006994f0 | SessionClass__ParseMultiplayerMapHeader |
| 004f1b80 | GraphicMenu__TryCreate | 00699980 | SessionClass__ScanMultiplayerMapFiles |
| 004f1ca0 | GraphicMenu__Constructor | 0069a3b0 | MPGameFileEntry__Constructor |
| 004f2f10 | GraphicMenuAnimItem__Constructor | 0069e090 | Overlay_LazyLoadIconResource |
| 0052ba60 | Init_Game | 006b5490 | SmudgeTypeClass__ReloadTheaterIcons |
| 0052cb90 | Load_Campaigns | 006b9d00 | CD_ConvertSurfaceAndCacheEntry |
| 0052cd70 | Load_Game_Rules | 006c9b40 | Subtitle_LookupByKey |
| 00530460 | Init_Mix_Files | 0071dca0 | TerrainTypeClass__ReloadTheaterIcons |
| 005312a0 | Init_DropPodAssets | 007207f0 | VoiceClip_ComputeDurations |
| 00531680 | Show_Loading_Screen | 0072ade0 | ConvertClass__BuildFromPaletteFile |
| 00533d20 | Load_Hotkey_Bindings | 007346a0 | CSF_LoadHeaderAndLanguage |
| 00552d60 | ScenarioClass__DrawLoadingScreen | 00734990 | CSF_ParseLabelStringChunks |
| 00558dd0 | LoadSaveDialog__RunModalLoop | 00764b90 | Campaign_WorldDominationTour__Constructor |
| 005b3c20 | MixFileClass__Constructor_Registered | 00765000 | Campaign_WorldDominationTour__RecordHistoryEntry |
| 005b82f0 | ModemHost__InitDialog | 00765410 | WDTHistory__LoadOrCreate |
| 005ba660 | GameOptions__SendPacket | 007681e0 | Selection__WorldDominationTour__InitScreen |
| 005bfaa0 | VQMovie__CreateDisplayContext | 00768f50 | Selection__WorldDominationTour__CreateOverlayFromEntry |
| 005c0640 | MovieFile__ResolveExtension | 0076c290 | Selection__WorldDominationTour__Constructor |
| 005cb590 | ConvertClass__BuildFrom256RGB | 0076d4d0 | Selection__WorldDominationTour__AnimateNodePath |
| 005ccc30 | MSVQAnim__Constructor | 0076fb90 | Selection__WorldDominationTour__CreateSideBarOverlayAnim |
| 005ce4a0 | MSPCXAnim__Constructor_Centered | 0076fc50 | Selection__WorldDominationTour__CreateHelpBarOverlayAnim |
| 005ce640 | MSPCXAnim__Constructor_Positioned | 00778210 | BSurface__CaptureAndConvertFromViewPort |
| 005ceef0 | MapSelect__LoadStagesFromINI | 00778710 | WOLPlayerInfo__LoadDefaultRecord |
| 005d2e90 | MSFont__AllocLoaded | 00778a20 | WOLPlayerInfo__LoadRecord |
| 005d67b0 | GetOrCreate_CCINIClassMember | 00779730 | WOLSettings__GetAutoLogin |
| 005d7590 | MPGameOptions__LoadGameModeFromINI | 00779830 | WOLSettings__SetAutoLogin |
| 005e3d10 | MPGameOptions__ParsePacket | 00779940 | WOLSettings__GetAddMatchBuddies |
| 005e6520 | MPGameOptions__GetScenarioPlayerCount | 00779b50 | WOLInfo__GetAutoLogin |
| 005e74e0 | MPGameStart__WaitForScenarioFile | 00779c90 | WOLInfo__SetAutoLogin |
| 005e7bf0 | MPGameOptions__SelectScenario | 00779dd0 | WOLInfo__GetLastValid |
| 005f7a90 | Load_Turret_Shape_ForHouse | 00779f10 | WOLInfo__SetLastValid |
| 005f7db0 | Load_Barrel_Shape_ForHouse | 0077a080 | WOLInfo__SaveProfileToINI |
| 005f8110 | Load_Turret_Barrel_Shape_Variants | 00788180 | WDT__FindTerritoryIndex |
| 005f8ce0 | TechnoTypeClass__LoadTurretBarrelShapes | 007afb90 | WorldDominationTour__Launch |
| 00641db0 | Load_Bitmap_Into_DSurface | 007b0490 | FactionSelectDialogControl__WDT__Constructor |
| 00641ee0 | MPGameOptions__ComputeScenarioDigest | | |

## Caveats

- Names are lane-proposed from this-session body evidence; parent spot-checked `0x00530460` +
  `0x006951f0` directly and applied the rest on the lanes' cited evidence. They are all strictly
  better than the affirmatively-wrong "Constructor" (which was wrong for all 91). A future pass may
  refine any individual name — cite the address, not just the name.
- Two `ConvertClass__BuildFrom*` and two `MSPCXAnim__Constructor_*` names were distinguished with
  suffixes to satisfy the labeler's token-uniqueness guard; the split reflects real overload/variant
  differences noted in the lane files but the exact distinction for the ConvertClass pair is
  low-confidence.
- Relates to `reference_ghidra_dtm_and_label_pollution` (memory) and `GHIDRA_PROJECT_HEALTH_AUDIT`.
