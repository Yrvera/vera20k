//! One-off diagnostic: try loading every map file in the RA2 directory and
//! report pass/fail with reason. Used to verify the MIX-vs-INI dispatch fix.

use std::path::Path;

use vera20k::map::map_file;

fn main() {
    let dir = Path::new("C:/Users/enok/Documents/Command and Conquer Red Alert II/");
    let mut by_ext: std::collections::BTreeMap<String, (u32, u32)> = Default::default();
    let mut failures: Vec<(String, String)> = Vec::new();

    let entries = std::fs::read_dir(dir).expect("read RA2 dir");
    let mut paths: Vec<_> = entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    paths.sort();

    for path in &paths {
        if !path.is_file() { continue; }
        let Some(ext) = path.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()) else { continue };
        if !matches!(ext.as_str(), "mmx" | "yro" | "map" | "mpr" | "yrm") { continue; }
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let entry = by_ext.entry(ext.clone()).or_insert((0, 0));
        match map_file::load_from_path(path) {
            Ok(m) => {
                entry.0 += 1;
                println!("OK   {:<22} theater={:<10} {}x{}  cells={}",
                    name, m.header.theater, m.header.width, m.header.height, m.cells.len());
            }
            Err(e) => {
                entry.1 += 1;
                let msg = format!("{}", e);
                println!("FAIL {:<22} {}", name, msg);
                failures.push((name, msg));
            }
        }
    }

    println!("\n=== summary ===");
    for (ext, (ok, fail)) in &by_ext {
        println!("  .{:<3}  {} ok, {} failed", ext, ok, fail);
    }
    if !failures.is_empty() {
        println!("\n=== failures ===");
        for (name, msg) in &failures {
            println!("  {:<22} {}", name, msg);
        }
    }
}
