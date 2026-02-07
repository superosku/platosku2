use egui::ahash::HashMap;
use quad_snd::{AudioContext, PlaySoundParams, Sound as SndSound};
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
    Jump        => "jump.wav",
    Walk        => "walk.wav",
    CollectCoin => "collect_coin.wav",
    Swing       => "swing.wav",
    Throw       => "throw.wav",
    Clink       => "clink.wav",
}

fn load_sound(path: &str) -> std::io::Result<Vec<u8>> {
    fs::read(path)
}

pub struct SoundHandler {
    sounds: HashMap<Sound, SndSound>,
    audio_context: AudioContext,
}

impl SoundHandler {
    pub fn new() -> Self {
        let mut sounds: HashMap<Sound, SndSound> = HashMap::default();

        let audio_context = AudioContext::new();

        for sound in Sound::ALL {
            let file_name = sound.file_name();
            let full_path = format!("assets/sounds/{}", file_name);
            if let Ok(bytes) = load_sound(&full_path) {
                let click_sound = SndSound::load(&audio_context, &bytes);
                sounds.insert(*sound, click_sound);
            }
        }

        SoundHandler {
            sounds,
            audio_context,
        }
    }

    pub fn play(&self, sound: Sound) {
        let sound = self.sounds.get(&sound);
        if let Some(sound) = sound {
            sound.play(&self.audio_context, PlaySoundParams::default());
        }
    }
}
