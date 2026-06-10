//! AT-3 guard: real launches must construct the sim through
//! `ScenarioDescriptor` — never `Simulation::new()`/`Simulation::default()`.
//! Source-scan tripwire because `new()` must stay available to the 250+
//! test fixtures (integration tests compile without cfg(test), so the
//! constructor cannot be cfg-gated away).

const LAUNCH_PATH_SOURCES: &[&str] = &[
    "src/app_init_helpers.rs",
    "src/app_init.rs",
    "src/app_loading.rs",
];

#[test]
fn launch_path_never_uses_the_default_sim_seed() {
    for rel in LAUNCH_PATH_SOURCES {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
        let src =
            std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {rel}: {e}"));
        // Strip the in-file test module: everything from the first
        // `#[cfg(test)]` to EOF is fixture territory.
        let live = src.split("#[cfg(test)]").next().unwrap_or(&src);
        for needle in ["Simulation::new()", "Simulation::default()"] {
            assert!(
                !live.contains(needle),
                "{rel} uses {needle} on the launch path — construct via \
                 ScenarioDescriptor/Simulation::from_descriptor instead"
            );
        }
    }
}
