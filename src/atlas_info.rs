use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;
use std::{fs, io};

#[derive(Serialize, Deserialize, Debug)]
struct FrameJsonItemFrame {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

#[derive(Serialize, Deserialize, Debug)]
struct FrameJsonItem {
    #[serde(rename = "filename")]
    file_name: String,
    frame: FrameJsonItemFrame,
}

#[derive(Serialize, Deserialize, Debug)]
struct FrameJson {
    frames: Vec<FrameJsonItem>,
}

pub struct AtlasRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// Drawn for a sprite that is not in the atlas.
const MISSING_RECT: AtlasRect = AtlasRect {
    x: 0,
    y: 0,
    w: 1,
    h: 1,
};

/// Where each sprite frame is in `assets/atlas.png`, from `assets/atlas.json`.
/// Frames are named `"<name> <n>.aseprite"` (or `"<name>.aseprite"` for frame 0).
pub struct AtlasInfo {
    /// Sprite name -> frame index -> rect.
    sprites: HashMap<String, HashMap<i32, AtlasRect>>,
    /// Missing sprites that were already reported, so they are logged only once.
    reported_missing: RefCell<HashSet<(String, i32)>>,
}

impl AtlasInfo {
    pub fn load_from_file() -> AtlasInfo {
        let s = fs::read_to_string(Path::new("assets/atlas.json")).unwrap();
        let file_content: FrameJson = serde_json::from_str(&s)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
            .unwrap();

        let mut sprites: HashMap<String, HashMap<i32, AtlasRect>> = HashMap::new();

        for frame in file_content.frames {
            let without_ext = frame.file_name.strip_suffix(".aseprite").unwrap();

            let (name, number) = without_ext.rsplit_once(' ').unwrap_or((without_ext, "0"));
            let index = number.parse::<i32>().unwrap();

            sprites.entry(name.to_string()).or_default().insert(
                index,
                AtlasRect {
                    x: frame.frame.x as u32,
                    y: frame.frame.y as u32,
                    w: frame.frame.w as u32,
                    h: frame.frame.h as u32,
                },
            );
        }

        AtlasInfo {
            sprites,
            reported_missing: RefCell::new(HashSet::new()),
        }
    }

    /// All sprite names with their frame indices, sorted.
    pub fn sprites(&self) -> BTreeMap<String, Vec<i32>> {
        self.sprites
            .iter()
            .map(|(name, frames)| {
                let mut indices: Vec<i32> = frames.keys().copied().collect();
                indices.sort();
                (name.clone(), indices)
            })
            .collect()
    }

    pub fn get_rect(&self, sprite: &str, frame_i: i32) -> &AtlasRect {
        if let Some(rect) = self.sprites.get(sprite).and_then(|f| f.get(&frame_i)) {
            return rect;
        }
        if self
            .reported_missing
            .borrow_mut()
            .insert((sprite.to_string(), frame_i))
        {
            println!("Sprite not found {} {}", sprite, frame_i);
            println!("Potential sprites {:?}", self.sprites().keys());
        }
        &MISSING_RECT
    }
}
