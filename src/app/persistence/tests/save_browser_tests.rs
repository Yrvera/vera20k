use super::*;

#[test]
fn browser_admission_keeps_empty_descriptions_and_uses_file_time() {
    let directory = std::env::temp_dir().join(format!(
        "vera-browser-metadata-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let repository = SaveRepository::at(&directory);
    let mut sim = Simulation::new();
    sim.session.map_name = "BROWSER.MAP".into();
    sim.session.game_mode_nonzero = true;
    let bytes = GameSnapshot::save_validated(&sim, 1, 2, "", 3);
    let path = repository.write_named("valid.bin", &bytes).unwrap();
    let modified = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000);
    std::fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_times(std::fs::FileTimes::new().set_modified(modified))
        .unwrap();
    let mut header = GameSnapshot::read_header(&bytes).unwrap();
    header.version = 0;
    repository
        .write_named("truncated.bin", &bincode::serialize(&header).unwrap())
        .unwrap();
    assert_eq!(repository.panel_entries_by_embedded_time().len(), 1);
    let entries = repository.browser_entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].0.path, path);
    assert!(entries[0].0.header.description.is_empty());
    assert_eq!(
        entries[0].1,
        crate::map::rmg::saved_seeds::system_time_to_file_time(modified)
    );
    std::fs::remove_dir_all(directory).unwrap();
}
