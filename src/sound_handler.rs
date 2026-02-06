use egui::ahash::HashMap;
use quad_snd::{AudioContext, PlaySoundParams, Sound as SndSound};
use rand::seq::IndexedRandom;
use std::fs;

macro_rules! define_sounds {
    ($($variant:ident => $file:literal),+ $(,)?) => {
        #[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
        pub enum Sound {
            $($variant),+
        }

        impl Sound {
            pub const ALL: &'static [Sound] = &[
                $(Sound::$variant),+
            ];

            pub const fn file_name(self) -> &'static str {
                match self {
                    $(Sound::$variant => $file),+
                }
            }
        }
    };
}

define_sounds! {
    Jump        => "jump",
    Walk        => "walk",
    CollectCoin => "collect_coin",
    Swing       => "swing",
    Throw       => "throw",
    Clink       => "clink",
}

fn load_sound(path: &str) -> std::io::Result<Vec<u8>> {
    fs::read(path)
}

pub struct SoundHandler {
    #[allow(dead_code)]
    sounds: HashMap<Sound, SndSound>,
    sound_variants: HashMap<Sound, Vec<SndSound>>,
    audio_context: AudioContext,
}

impl SoundHandler {
    pub fn new() -> Self {
        let audio_context = AudioContext::new();

        let mut sounds: HashMap<Sound, SndSound> = HashMap::default();
        let mut sound_variants: HashMap<Sound, Vec<SndSound>> = HashMap::default();
        for sound in Sound::ALL {
            let file_name = sound.file_name();

            // Non variant sound
            let full_path = format!("assets/sounds/{}", file_name);
            if let Ok(bytes) = load_sound(&full_path) {
                let click_sound = SndSound::load(&audio_context, &bytes);
                sounds.insert(*sound, click_sound);
            }

            // Variant sounds
            // 1. List file names such as "assets/sounds/dest/{}__v01.wav"
            let mut variant_sounds: Vec<SndSound> = Vec::new();
            for variant_i in 1..9 {
                let full_path = format!("assets/sounds/dest/{}__v{:02}.wav", file_name, variant_i);
                if let Ok(bytes) = load_sound(&full_path) {
                    let click_sound = SndSound::load(&audio_context, &bytes);
                    variant_sounds.push(click_sound);
                }
            }
            sound_variants.insert(*sound, variant_sounds);
        }

        SoundHandler {
            sounds,
            sound_variants,
            audio_context,
        }
    }

    pub fn play(&self, sound: Sound) {
        let sound_variants = self.sound_variants.get(&sound).unwrap();

        if sound_variants.is_empty() {
            return;
        }

        let mut rng = rand::rng();
        let sns_sound = sound_variants.choose(&mut rng).unwrap();

        sns_sound.play(&self.audio_context, PlaySoundParams::default());
    }
}
