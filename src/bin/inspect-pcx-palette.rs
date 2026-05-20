//! Diagnostic: inspect shell-button PCX palettes and dump their rendered
//! RGBA to PNG so the actual on-disk color content can be examined. Also
//! dumps the SDTP / SDBTNBKGD / SDBTM right-panel chrome SHPs rendered with
//! SHELL.PAL / SHELL2.PAL to confirm what colors the chrome behind the
//! buttons contains.

use std::path::Path;
use vera20k::assets::asset_manager::AssetManager;
use vera20k::assets::pal_file::Palette;
use vera20k::assets::pcx_file::PcxFile;
use vera20k::assets::shp_file::ShpFile;

fn save_rgba(path: &Path, rgba: &[u8], w: u32, h: u32) {
    let img =
        image::RgbaImage::from_raw(w, h, rgba.to_vec()).expect("rgba buffer length matches w*h*4");
    img.save(path).expect("save png");
    println!("    wrote {}", path.display());
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let ra2_dir = Path::new("C:/Users/enok/Documents/Command and Conquer Red Alert II/");
    let out_dir = Path::new("target/pcx-dump");
    std::fs::create_dir_all(out_dir).expect("create target/pcx-dump");
    let assets = AssetManager::new(ra2_dir).expect("Failed to load MIX archives");

    println!("=== Button PCXs ===");
    for name in [
        "bue_li30.pcx",
        "bue_mi30.pcx",
        "bue_ri30.pcx",
        "bde_li30.pcx",
        "bde_mi30.pcx",
        "bde_ri30.pcx",
    ] {
        let Some(bytes) = assets.get_ref(name) else {
            println!("{name}: NOT FOUND in any MIX archive");
            continue;
        };
        let pcx = match PcxFile::from_bytes(bytes) {
            Ok(p) => p,
            Err(e) => {
                println!("{name}: parse error: {e}");
                continue;
            }
        };
        let mut max_r = 0u8;
        let mut max_g = 0u8;
        let mut max_b = 0u8;
        let mut greyscale_only = true;
        for [r, g, b] in pcx.palette {
            max_r = max_r.max(r);
            max_g = max_g.max(g);
            max_b = max_b.max(b);
            if r != g || g != b {
                greyscale_only = false;
            }
        }
        println!(
            "{name}: {}x{} px, palette max R={max_r} G={max_g} B={max_b}{}",
            pcx.width,
            pcx.height,
            if greyscale_only {
                " (all greyscale)"
            } else {
                " (contains color)"
            }
        );
        // Dump the rendered RGBA with index 0 = transparent (matches the
        // shell-chrome atlas's PCX loader) AND with no transparency, so we
        // can see whether the inside of the button is index-0 (transparent)
        // or filled with non-zero greyscale values.
        let rgba_with_transparency = pcx.to_rgba(Some(0));
        save_rgba(
            &out_dir.join(format!("{name}.transparent_idx0.png")),
            &rgba_with_transparency,
            pcx.width as u32,
            pcx.height as u32,
        );
        let rgba_opaque = pcx.to_rgba(None);
        save_rgba(
            &out_dir.join(format!("{name}.opaque.png")),
            &rgba_opaque,
            pcx.width as u32,
            pcx.height as u32,
        );
    }

    println!("\n=== Right-panel chrome SHPs ===");
    let shell_pal_bytes = assets.get_ref("SHELL.PAL").expect("SHELL.PAL must exist");
    let shell_pal = Palette::from_bytes(shell_pal_bytes).expect("SHELL.PAL parse");
    let shell2_pal_bytes = assets.get_ref("SHELL2.PAL").expect("SHELL2.PAL must exist");
    let shell2_pal = Palette::from_bytes(shell2_pal_bytes).expect("SHELL2.PAL parse");

    let sdbtnanm_pal = assets
        .get_ref("SDBTNANM.PAL")
        .and_then(|b| Palette::from_bytes(b).ok())
        .or_else(|| Some(shell2_pal.clone()))
        .expect("SDBTNANM palette");
    let chrome: &[(&str, &Palette)] = &[
        ("SDTP.SHP", &shell_pal),
        ("SDBTNBKGD.SHP", &shell2_pal),
        ("SDBTM.SHP", &shell_pal),
        ("LWSCRNS.SHP", &shell_pal),
        ("LWSCRNL.SHP", &shell_pal),
        ("SDBTNANM.SHP", &sdbtnanm_pal),
    ];
    for (name, palette) in chrome {
        let Some(bytes) = assets.get_ref(name) else {
            println!("{name}: NOT FOUND");
            continue;
        };
        let shp = match ShpFile::from_bytes(bytes) {
            Ok(s) => s,
            Err(e) => {
                println!("{name}: parse error: {e}");
                continue;
            }
        };
        println!(
            "{name}: canvas {}x{}, {} frames",
            shp.width,
            shp.height,
            shp.frames.len()
        );
        let frames_to_dump: Vec<usize> = if *name == "SDBTNANM.SHP" {
            (0..shp.frames.len()).collect()
        } else if shp.frames.len() > 10 {
            vec![0, 10]
        } else {
            vec![0]
        };
        for fi in frames_to_dump {
            let Some(rgba) = shp.frame_to_rgba(fi, palette).ok() else {
                continue;
            };
            let f = &shp.frames[fi];
            save_rgba(
                &out_dir.join(format!("{name}.frame{fi}.png")),
                &rgba,
                f.frame_width as u32,
                f.frame_height as u32,
            );
        }
    }
}
