//! Renders the frames of a sprite from the texture atlas into one enlarged PNG,
//! for reviewing sprite work. Needs no window or GPU.
//!
//!     cargo run --example sprite_sheet -- list             # sprite names and frame counts
//!     cargo run --example sprite_sheet -- character        # all frames of one sprite
//!     cargo run --example sprite_sheet -- character 0-7    # a frame range (inclusive)
//!     cargo run --example sprite_sheet -- --all            # one sheet per sprite
//!
//! Options: --scale N (default 8). Output: target/claude/sprites/<name>.png.
//! Frames are laid out left to right in index order on a checkerboard (which shows
//! transparency), with a gap between frames. Re-export the atlas with aseprite
//! first if you edited the .aseprite sources (command in README.md).

use image::{Rgba, RgbaImage};
use platosku2::atlas_info::AtlasInfo;
use std::path::PathBuf;

const GAP: u32 = 2; // in source pixels

fn checker(x: u32, y: u32, scale: u32) -> Rgba<u8> {
    let cell = (scale * 2).max(4);
    if ((x / cell) + (y / cell)).is_multiple_of(2) {
        Rgba([60, 60, 70, 255])
    } else {
        Rgba([90, 90, 100, 255])
    }
}

fn render_sheet(
    atlas: &AtlasInfo,
    atlas_img: &RgbaImage,
    name: &str,
    frames: &[i32],
    scale: u32,
) -> RgbaImage {
    let rects: Vec<_> = frames.iter().map(|f| atlas.get_rect(name, *f)).collect();
    let max_h = rects.iter().map(|r| r.h).max().unwrap_or(1);
    let total_w: u32 = rects.iter().map(|r| r.w + GAP).sum::<u32>() + GAP;
    let total_h = max_h + 2 * GAP;

    let mut out = RgbaImage::from_fn(total_w * scale, total_h * scale, |x, y| {
        checker(x, y, scale)
    });
    // Separator columns between frames, so frame boundaries are visible.
    let mut x0 = GAP;
    for rect in &rects {
        for sy in 0..rect.h {
            for sx in 0..rect.w {
                let src = atlas_img.get_pixel(rect.x + sx, rect.y + sy);
                if src[3] == 0 {
                    continue;
                }
                for dy in 0..scale {
                    for dx in 0..scale {
                        let px = (x0 + sx) * scale + dx;
                        let py = (GAP + sy) * scale + dy;
                        // Simple alpha blend over the checkerboard.
                        let bg = *out.get_pixel(px, py);
                        let a = src[3] as u32;
                        let blend =
                            |s: u8, b: u8| ((s as u32 * a + b as u32 * (255 - a)) / 255) as u8;
                        out.put_pixel(
                            px,
                            py,
                            Rgba([
                                blend(src[0], bg[0]),
                                blend(src[1], bg[1]),
                                blend(src[2], bg[2]),
                                255,
                            ]),
                        );
                    }
                }
            }
        }
        // Thin marker line under each frame.
        for sx in 0..rect.w * scale {
            for t in 0..(scale / 4).max(1) {
                out.put_pixel(
                    x0 * scale + sx,
                    (GAP + max_h) * scale + scale / 2 + t,
                    Rgba([200, 160, 40, 255]),
                );
            }
        }
        x0 += rect.w + GAP;
    }
    out
}

fn parse_range(s: &str) -> Option<(i32, i32)> {
    match s.split_once('-') {
        Some((a, b)) => Some((a.parse().ok()?, b.parse().ok()?)),
        None => {
            let n = s.parse().ok()?;
            Some((n, n))
        }
    }
}

fn main() {
    let mut scale = 8;
    let mut positional = Vec::new();
    let mut all = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--scale" => {
                scale = args
                    .next()
                    .and_then(|s| s.parse().ok())
                    .expect("--scale needs a number")
            }
            "--all" => all = true,
            _ => positional.push(arg),
        }
    }

    let atlas = AtlasInfo::load_from_file();
    let sprites = atlas.sprites();

    if positional.first().map(String::as_str) == Some("list") {
        for (name, frames) in &sprites {
            println!(
                "{name}: {} frames ({}-{})",
                frames.len(),
                frames[0],
                frames[frames.len() - 1]
            );
        }
        return;
    }

    let atlas_img = image::open("assets/atlas.png")
        .expect("assets/atlas.png")
        .to_rgba8();
    let out_dir = PathBuf::from("target/claude/sprites");
    std::fs::create_dir_all(&out_dir).unwrap();

    let jobs: Vec<(String, Vec<i32>)> = if all {
        sprites.into_iter().collect()
    } else {
        let Some(name) = positional.first() else {
            eprintln!("usage: sprite_sheet list | <name> [from-to] | --all  [--scale N]");
            std::process::exit(2);
        };
        let Some(frames) = sprites.get(name) else {
            eprintln!("unknown sprite {name:?}; run with 'list' to see all sprites");
            std::process::exit(1);
        };
        let frames = match positional.get(1).map(|r| parse_range(r)) {
            Some(Some((from, to))) => frames
                .iter()
                .copied()
                .filter(|f| (from..=to).contains(f))
                .collect(),
            Some(None) => {
                eprintln!("bad frame range {:?}, expected e.g. 0-7", positional[1]);
                std::process::exit(2);
            }
            None => frames.clone(),
        };
        vec![(name.clone(), frames)]
    };

    for (name, frames) in jobs {
        let sheet = render_sheet(&atlas, &atlas_img, &name, &frames, scale);
        let path = out_dir.join(format!("{name}.png"));
        sheet.save(&path).expect("saving sprite sheet");
        println!("{} ({} frames: {:?})", path.display(), frames.len(), frames);
    }
}
