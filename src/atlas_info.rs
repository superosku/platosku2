use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

pub struct AtlasInfo {
    mapper: HashMap<(String, i32), (i32, i32)>,
}

impl AtlasInfo {
    pub fn load_from_file() -> AtlasInfo {
        let s = fs::read_to_string(Path::new("assets/atlas.json")).unwrap();
        let file_content: FrameJson = serde_json::from_str(&s)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
            .unwrap();

        let mut mapper = HashMap::new();

        for frame in file_content.frames {
            let without_ext = frame.file_name.strip_suffix(".aseprite").unwrap();
            let (name, number) = without_ext.rsplit_once(' ').unwrap();
            let index = number.parse::<i32>().ok().unwrap();
            mapper.insert((name.to_string(), index), (frame.frame.x, frame.frame.y));
        }

        AtlasInfo { mapper }
    }

    pub fn get_xy(&self, sprite: &str, frame_i: i32) -> (i32, i32) {
        let result = self.mapper.get(&(sprite.to_string(), frame_i));

        if let Some(xy) = result {
            *xy
        } else {
            println!("Sprite not found {} {}", sprite, frame_i);
            println!("Potential sprites {:?}", self.mapper.keys());
            (0, 0)
        }
    }
}
