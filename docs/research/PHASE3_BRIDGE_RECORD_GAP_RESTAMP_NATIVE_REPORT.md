# Inactive high-bridge restamp: bounded retail witness

Source baseline `6c7ccf92` plus parent documentation checkpoint `2c84fba7`. No production semantics changed. The added ignored diagnostic calls existing `headless_scenario::load` and the existing production bridge record factory. This proves a concrete retail selector and a difference between composed native leaves and the current Rust admission, **not complete native startup parity**.

## Retail selection and Rust result

`.local/restamp-retail-map-hashes.json` contains actual SHA256 receipts for four map inputs. `BayOPigs.mmx` produced two active records, `Hills.mmx` three active records, extracted `all01umd.map` zero records. `Deadman.mmx` (SHA256 `8d17f1937e7215ae2bd210506640172e954cc00efa2e620a64717b4da132890d`) produced four inactive non-Tube high records:

- `(54,41) -> (62,41)`
- `(71,41) -> (83,41)`
- `(54,90) -> (62,90)`
- `(71,90) -> (79,90)`

All have group0 and active=false. The ordinary in-grid diagnostic found 20 interior cells missing0x100 and 80 distinct transverse cells. This diagnostic deduplicates candidate cells and does not model native write order/aliases/dummy behavior.

Selected witness `(57,42)` is inside the playfield: Clear tile0/subtile0, land0, level0, slope0, flags0, no overlay, no Techno occupancy, no terrain object, ground-walkable. Production `can_place_new_tiberium`, with actual rules/terrain/overlay/occupancy, returns true. The diagnostic conservatively excludes every terrain-object cell from new placement; since this witness is empty, the stricter set does not weaken its positive result.

Export `.local/restamp-deadman-export.json` has9035 allocated scalar cells, source Size/LocalSize, actual838-entry theater tile AllowTiberium table, bridge bases, land Buildable values and complete records. Parent `.local/restamp-retail-gate-provenance.json` binds `[TileSet0000] AllowTiberium=true` and `[Clear] Buildable=yes` to extracted retail bytes and original reader stores. No invalid-tile bypass is used.

## Original executable comparison

[`bridge_restamp_retail.py`](../../tools/spatial_oracle/bridge_restamp_retail.py) runs hash-checked original56D6E0 (including clear; adequate record backing supplied afterward), retains its output records in place, runs586BF0, then4838E0 on the empty witness. Native producer output exactly matches the four record semantic fields above; raw record bytes remain unchanged through restamp. There are80 native flag writes and80 changed cells. At `(57,42)` the flag change is0→0xC00. Native4838E0 returns AL1 before restamp and AL0 afterward; pre-fix Rust admission is true. The retained [native receipt](../../tools/spatial_oracle/bridge_restamp_retail.json) can be checked with `python -B -m tools.spatial_oracle.bridge_restamp_retail --check` after generating the scalar export; no retail map bulk is committed.

Saved result/log: `.local/restamp-native-deadman.json` and `.local/restamp-native-deadman.log`, actual exit0. No executable patches or substituted returns. The actual original playfield and fixed-cell lookup execute. Ground object list is empty and therefore legitimately bypasses virtual object predicates.

**Boundary:** scalar input comes from the current Rust retail loader, not native loader execution. Native684C30's intervening connectivity and all-zone-level calls are omitted. A fresh independent preservation audit establishes the relevant boundary: 56C510/56CB90 write separate zone/adjacency/work allocations; record loop56C6E6 reads active+8 and56C6EF skips these inactive records. 581F50→581F90(2,1,0)→42C1C0 write separate hierarchy/pathfinder stores; loop582346 skips582D70 for inactive records. Flood5824A0 writes hierarchy/temp edges and calls578460 for Cell reads. Thus, under normal disjoint allocations, record endpoints/type/activity and real Cell100/400/800 survive the omitted calls. Dummy coordinates may change; this is not full-zone execution or terminal-dummy startup parity. This probe exercises horizontal OR0xC00 on ordinary allocated cells. Vertical clearing, retained dummy, overlaps/order and persistence require later comparisons. The script's map hash is linked to the separate actual-file receipt; its scalar export is directly hashed.

## Actual validation receipts

Both diagnostic commands used `cargo test -p vera20k --lib sim::movement::movement_bridge_retail_tests::bridge_restamp_retail_probe::retail_inactive_high_record_restamp_inventory -- --ignored --exact --nocapture` with a process-idle check. Four-map default:1passed0failed8721filtered,3.67s run/3m58 compile, exit0 (`restamp-retail-inventory.log`). Frozen extended export with `VERA20K_RESTAMP_MAPS=Deadman.mmx`:1passed0failed8721filtered,0.90s run/3m48 compile, exit0 (`restamp-deadman-export.log`). Existing58 lib-test warnings. No full suite or Clippy run for this research-only diagnostic.

Fresh independent critic reviewed selector and exact native setup, independently replayed the original comparison, and completed the relevant omitted-middle-call preservation proof. Next work is coherent flag authority/serialization and fresh-load production delivery; neither row46 nor Phase3 is complete.
