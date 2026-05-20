//! Headless diagnostic: load one or more map files through the full
//! render-prep pipeline (`map_file::load_from_path` → `load_theater` →
//! `load_tile_images`) with env_logger at info level, so the in-pipeline
//! diagnostic logs (IsoMapPack5 distribution, theater INI/MIX hits, tile
//! loading summary + sample tile_ids) print to stderr.
//!
//! Used to compare how .yro maps resolve tiles vs .mmx maps without
//! launching the GUI.
//!
//! Subcommand `assets` probes specific filenames against the loaded asset
//! manager and reports whether each is present + which archive holds it.

use std::path::PathBuf;

use vera20k::assets::asset_manager::AssetManager;
use vera20k::map::map_file;
use vera20k::map::theater;

const RA2_DIR: &str = "C:/Users/enok/Documents/Command and Conquer Red Alert II";

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn"))
        .target(env_logger::Target::Stderr)
        .init();

    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.first().map(|s| s.as_str()) == Some("assets") {
        run_asset_probe();
        return;
    }

    run_map_pipeline(&args);
}

fn run_asset_probe() {
    let ra2_dir = PathBuf::from(RA2_DIR);
    let asset_manager = match AssetManager::new(&ra2_dir) {
        Ok(am) => am,
        Err(e) => {
            eprintln!("AssetManager bootstrap failed: {:#}", e);
            return;
        }
    };
    let probes: &[&str] = &[
        // LUNAR
        "lunarmd.ini",
        "lunar.ini",
        "isolun.mix",
        "isolunmd.mix",
        "lun.mix",
        "lunar.mix",
        "isolun.pal",
        "lunar.pal",
        "unitlun.pal",
        // DESERT
        "desertmd.ini",
        "desert.ini",
        "isodes.mix",
        "isodesmd.mix",
        "des.mix",
        "desert.mix",
        "isodes.pal",
        "desert.pal",
        "unitdes.pal",
        // NEWURBAN
        "urbannmd.ini",
        "urbann.ini",
        "isoubn.mix",
        "isoubnmd.mix",
        "isourbnmd.mix",
        "ubn.mix",
        "urbann.mix",
        "isoubn.pal",
        "urbann.pal",
        "isourbn.pal",
        "unitubn.pal",
        "unitn.pal",
    ];
    for &name in probes {
        match asset_manager.get_with_source(name) {
            Some((data, source)) => {
                println!(
                    "FOUND   {:<22} {:>8} bytes  from {}",
                    name,
                    data.len(),
                    source
                );
            }
            None => {
                println!("missing {}", name);
            }
        }
    }
}

fn run_map_pipeline(args: &[String]) {
    let names: Vec<String> = if args.is_empty() {
        vec![
            "Transylv.yro".to_string(),
            "DeepFrze.yro".to_string(),
            "MoonPatr.yro".to_string(),
        ]
    } else {
        args.to_vec()
    };

    let ra2_dir = PathBuf::from(RA2_DIR);
    let mut asset_manager = match AssetManager::new(&ra2_dir) {
        Ok(am) => am,
        Err(e) => {
            eprintln!("AssetManager bootstrap failed: {:#}", e);
            return;
        }
    };

    // Bump logger filter to info for map-pipeline runs so diagnostic info
    // lines show up. (Asset probe stays at warn so its output is clean.)
    log::set_max_level(log::LevelFilter::Info);

    for name in &names {
        eprintln!(
            "\n\n========================= {} =========================",
            name
        );
        let path = ra2_dir.join(name);
        let map = match map_file::load_from_path(&path) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("LOAD FAIL: {:?}", e);
                continue;
            }
        };
        eprintln!(
            "Map loaded: theater={} size={}x{} cells={} entities={} overlays={}",
            map.header.theater,
            map.header.width,
            map.header.height,
            map.cells.len(),
            map.entities.len(),
            map.overlays.len()
        );

        let theater_data = match theater::load_theater(&mut asset_manager, &map.header.theater) {
            Some(t) => t,
            None => {
                eprintln!("THEATER LOAD FAIL for '{}'", map.header.theater);
                continue;
            }
        };

        let cell_pairs: Vec<(i32, u8)> = map
            .cells
            .iter()
            .map(|c| (c.tile_index, c.sub_tile))
            .collect();
        let needed = theater::collect_used_tiles(&cell_pairs);
        eprintln!("Needed TileKeys: {}", needed.len());

        let images = theater::load_tile_images(
            &asset_manager,
            &theater_data.lookup,
            &theater_data.iso_palette,
            &needed,
        );
        eprintln!("Loaded TileImages: {}", images.len());
    }
}
