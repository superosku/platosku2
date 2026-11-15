//! Game sounds.
//!
//! Sounds are loaded from `assets/sounds/src/<name>.wav`. To keep repeated
//! sounds from sounding identical, each one is pre-rendered at a few slightly
//! different pitches at startup (by resampling, so a higher pitch is also a bit
//! shorter, like speeding up a tape) and played at a slightly random volume.
//!
//! This is the audio layer, not game logic: it may use platform float math and
//! `rand::rng()`, because nothing here affects the game state.
#![allow(clippy::disallowed_methods)]

use quad_snd::{AudioContext, PlaySoundParams, Sound as SndSound};
use rand::Rng;
use rand::seq::IndexedRandom;
use std::cell::RefCell;
use std::io::Cursor;

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
    Swing       => "swipe__swipe_woosh_07",
    Throw       => "throw",
    Clink       => "clink",
    EnemyHit    => "enemy_hit__bong_clunk_hit_short_01",
    PlayerHit   => "player_hit__bong_clunk_hit_short_05",
}

/// How much a sound varies each time it plays.
struct Variation {
    /// Random pitch range, ± semitones.
    pitch_semitones: f32,
    /// Random volume range, ± dB.
    volume_db: f32,
}

impl Sound {
    fn variation(self) -> Variation {
        let (pitch_semitones, volume_db) = match self {
            Sound::Jump => (1.0, 1.0),
            Sound::Walk => (2.0, 2.0),
            Sound::CollectCoin => (0.5, 1.0),
            Sound::Swing => (2.0, 1.5),
            Sound::Throw => (2.0, 1.5),
            Sound::Clink => (1.5, 1.5),
            Sound::EnemyHit => (1.5, 1.0),
            Sound::PlayerHit => (1.0, 1.0),
        };
        Variation {
            pitch_semitones,
            volume_db,
        }
    }
}

/// Something that can play game sounds. The real game uses [`SoundHandler`];
/// tests use [`NullSound`] or [`RecordingSound`].
pub trait SoundPlayer {
    fn play(&self, sound: Sound);
}

/// Plays nothing.
pub struct NullSound;

impl SoundPlayer for NullSound {
    fn play(&self, _sound: Sound) {}
}

/// Plays nothing but remembers which sounds were requested, in order.
#[derive(Default)]
pub struct RecordingSound {
    played: std::cell::RefCell<Vec<Sound>>,
}

impl RecordingSound {
    pub fn played(&self) -> Vec<Sound> {
        self.played.borrow().clone()
    }

    pub fn clear(&self) {
        self.played.borrow_mut().clear();
    }
}

impl SoundPlayer for RecordingSound {
    fn play(&self, sound: Sound) {
        self.played.borrow_mut().push(sound);
    }
}

/// Number of pitch variants rendered for each sound, spread evenly over its
/// random pitch range.
const PITCH_VARIANTS: usize = 7;

/// A decoded sound: interleaved f32 samples.
struct Samples {
    data: Vec<f32>,
    channels: u16,
    sample_rate: u32,
}

fn load_wav(path: &str) -> Result<Samples, String> {
    let mut reader = hound::WavReader::open(path).map_err(|e| e.to_string())?;
    let spec = reader.spec();
    let data: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .collect::<Result<_, _>>()
            .map_err(|e| e.to_string())?,
        hound::SampleFormat::Int => {
            let scale = 1.0 / (1u32 << (spec.bits_per_sample - 1)) as f32;
            reader
                .samples::<i32>()
                .map(|s| s.map(|s| s as f32 * scale))
                .collect::<Result<_, _>>()
                .map_err(|e| e.to_string())?
        }
    };
    Ok(Samples {
        data,
        channels: spec.channels,
        sample_rate: spec.sample_rate,
    })
}

/// Plays the sound `ratio` times faster (linear interpolation), which raises
/// the pitch by `12 * log2(ratio)` semitones and shortens it.
fn resample(samples: &Samples, ratio: f32) -> Vec<f32> {
    let channels = samples.channels as usize;
    let in_frames = samples.data.len() / channels;
    if in_frames == 0 {
        return Vec::new();
    }
    let out_frames = (in_frames as f32 / ratio) as usize;
    let mut out = Vec::with_capacity(out_frames * channels);
    for i in 0..out_frames {
        let pos = i as f32 * ratio;
        let i0 = (pos as usize).min(in_frames - 1);
        let i1 = (i0 + 1).min(in_frames - 1);
        let t = pos - i0 as f32;
        for c in 0..channels {
            let a = samples.data[i0 * channels + c];
            let b = samples.data[i1 * channels + c];
            out.push(a + (b - a) * t);
        }
    }
    out
}

/// Encodes samples as a 16-bit WAV file in memory (quad-snd loads files).
fn encode_wav(data: &[f32], channels: u16, sample_rate: u32) -> Vec<u8> {
    let spec = hound::WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut bytes = Cursor::new(Vec::new());
    {
        let mut writer = hound::WavWriter::new(&mut bytes, spec).expect("in-memory WAV");
        for s in data {
            let v = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            writer.write_sample(v).expect("in-memory WAV");
        }
        writer.finalize().expect("in-memory WAV");
    }
    bytes.into_inner()
}

fn semitones_to_ratio(semitones: f32) -> f32 {
    2.0_f32.powf(semitones / 12.0)
}

fn db_to_gain(db: f32) -> f32 {
    10.0_f32.powf(db / 20.0)
}

struct LoadedSound {
    /// Pitch variants, picked at random on each play.
    variants: Vec<SndSound>,
    volume_db: f32,
}

/// When the same sound is requested more than once in a tick, the extra ones
/// are delayed by this many ticks each (about 67 ms), so they don't stack into
/// one loud sound.
const SAME_TICK_STAGGER_TICKS: u32 = 4;

/// Decides when sounds play so the same sound never starts twice in a tick.
/// Pure bookkeeping, no audio, so it can be unit tested.
#[derive(Default)]
struct Stagger {
    /// Sounds started during the current tick.
    started_this_tick: Vec<Sound>,
    /// Duplicates waiting for their turn, with ticks left until they play.
    queue: Vec<(Sound, u32)>,
}

impl Stagger {
    /// Returns true if the sound should play now; otherwise it is queued.
    fn request(&mut self, sound: Sound) -> bool {
        let last_queued = self
            .queue
            .iter()
            .filter(|(s, _)| *s == sound)
            .map(|(_, ticks)| *ticks)
            .max();
        match last_queued {
            None if !self.started_this_tick.contains(&sound) => {
                self.started_this_tick.push(sound);
                true
            }
            _ => {
                // Line up after the last one waiting.
                let ticks = last_queued.unwrap_or(0) + SAME_TICK_STAGGER_TICKS;
                self.queue.push((sound, ticks));
                false
            }
        }
    }

    /// Advances one tick and returns the queued sounds that should play now.
    fn end_tick(&mut self) -> Vec<Sound> {
        self.started_this_tick.clear();
        let mut due = Vec::new();
        self.queue.retain_mut(|(sound, ticks)| {
            *ticks -= 1;
            if *ticks == 0 {
                due.push(*sound);
            }
            *ticks > 0
        });
        self.started_this_tick.extend(&due);
        due
    }
}

pub struct SoundHandler {
    /// Indexed like [`Sound::ALL`]. `None` if the file couldn't be loaded.
    sounds: Vec<Option<LoadedSound>>,
    audio_context: AudioContext,
    stagger: RefCell<Stagger>,
}

impl SoundHandler {
    pub fn new() -> Self {
        let audio_context = AudioContext::new();
        let sounds = Sound::ALL
            .iter()
            .map(|sound| {
                let path = format!("assets/sounds/src/{}.wav", sound.file_name());
                match load_wav(&path) {
                    Ok(samples) => Some(Self::render_variants(&audio_context, *sound, &samples)),
                    Err(e) => {
                        eprintln!("Sound not loaded: {path}: {e}");
                        None
                    }
                }
            })
            .collect();

        SoundHandler {
            sounds,
            audio_context,
            stagger: RefCell::default(),
        }
    }

    fn render_variants(ctx: &AudioContext, sound: Sound, samples: &Samples) -> LoadedSound {
        let variation = sound.variation();
        let variants = (0..PITCH_VARIANTS)
            .map(|i| {
                // Evenly from -pitch_semitones to +pitch_semitones.
                let f = i as f32 / (PITCH_VARIANTS - 1) as f32 * 2.0 - 1.0;
                let ratio = semitones_to_ratio(f * variation.pitch_semitones);
                let data = resample(samples, ratio);
                let wav = encode_wav(&data, samples.channels, samples.sample_rate);
                SndSound::load(ctx, &wav)
            })
            .collect();
        LoadedSound {
            variants,
            volume_db: variation.volume_db,
        }
    }

    /// Call after every game tick: plays the queued sounds whose turn it is.
    pub fn end_tick(&self) {
        let due = self.stagger.borrow_mut().end_tick();
        for sound in due {
            self.play_now(sound);
        }
    }

    fn play_now(&self, sound: Sound) {
        let Some(loaded) = self.loaded(sound) else {
            return;
        };
        let mut rng = rand::rng();
        let Some(variant) = loaded.variants.choose(&mut rng) else {
            return;
        };
        let db = if loaded.volume_db > 0.0 {
            rng.random_range(-loaded.volume_db..loaded.volume_db)
        } else {
            0.0
        };
        variant.play(
            &self.audio_context,
            PlaySoundParams {
                looped: false,
                volume: db_to_gain(db),
            },
        );
    }

    fn loaded(&self, sound: Sound) -> Option<&LoadedSound> {
        let index = Sound::ALL.iter().position(|s| *s == sound)?;
        self.sounds[index].as_ref()
    }
}

impl Default for SoundHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl SoundPlayer for SoundHandler {
    fn play(&self, sound: Sound) {
        if self.stagger.borrow_mut().request(sound) {
            self.play_now(sound);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ramp(frames: usize) -> Samples {
        Samples {
            data: (0..frames).flat_map(|i| [i as f32, -(i as f32)]).collect(),
            channels: 2,
            sample_rate: 44100,
        }
    }

    #[test]
    fn resample_at_ratio_one_keeps_the_samples() {
        let s = ramp(10);
        assert_eq!(resample(&s, 1.0), s.data);
    }

    #[test]
    fn resample_twice_as_fast_halves_the_length_and_keeps_channels_apart() {
        let s = ramp(10);
        let out = resample(&s, 2.0);
        assert_eq!(
            out,
            vec![0.0, 0.0, 2.0, -2.0, 4.0, -4.0, 6.0, -6.0, 8.0, -8.0]
        );
    }

    #[test]
    fn resample_interpolates_between_frames() {
        let s = ramp(4);
        let out = resample(&s, 0.5);
        assert_eq!(out.len(), 16);
        assert_eq!(&out[..6], &[0.0, 0.0, 0.5, -0.5, 1.0, -1.0]);
    }

    #[test]
    fn semitone_and_db_conversions() {
        assert!((semitones_to_ratio(12.0) - 2.0).abs() < 1e-6);
        assert!((semitones_to_ratio(-12.0) - 0.5).abs() < 1e-6);
        assert!((db_to_gain(0.0) - 1.0).abs() < 1e-6);
        assert!((db_to_gain(20.0) - 10.0).abs() < 1e-5);
    }

    #[test]
    fn every_sound_file_loads() {
        for sound in Sound::ALL {
            let path = format!("assets/sounds/src/{}.wav", sound.file_name());
            let samples = load_wav(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
            assert!(!samples.data.is_empty(), "{path} is empty");
        }
    }

    /// Runs `ticks` ticks, returning the sounds played on each one.
    fn run_stagger(stagger: &mut Stagger, requests: &[&[Sound]]) -> Vec<Vec<Sound>> {
        let mut out = Vec::new();
        for tick_requests in requests {
            let mut played = stagger.end_tick();
            for sound in *tick_requests {
                if stagger.request(*sound) {
                    played.push(*sound);
                }
            }
            out.push(played);
        }
        out
    }

    #[test]
    fn stagger_spreads_same_tick_duplicates() {
        use Sound::*;
        let mut stagger = Stagger::default();
        let mut requests: Vec<&[Sound]> = vec![&[CollectCoin, CollectCoin, Jump, CollectCoin]];
        requests.resize(10, &[]);
        let played = run_stagger(&mut stagger, &requests);
        let n = SAME_TICK_STAGGER_TICKS as usize;
        assert_eq!(played[0], vec![CollectCoin, Jump]);
        assert_eq!(played[n], vec![CollectCoin]);
        assert_eq!(played[2 * n], vec![CollectCoin]);
        assert_eq!(
            played.iter().flatten().count(),
            4,
            "nothing lost: {played:?}"
        );
    }

    #[test]
    fn stagger_plays_different_or_later_sounds_right_away() {
        use Sound::*;
        let mut stagger = Stagger::default();
        let played = run_stagger(&mut stagger, &[&[Jump, Swing], &[Jump], &[], &[Jump]]);
        assert_eq!(
            played,
            vec![vec![Jump, Swing], vec![Jump], vec![], vec![Jump]]
        );
    }

    #[test]
    fn stagger_queues_behind_a_waiting_duplicate() {
        use Sound::*;
        let mut stagger = Stagger::default();
        // The second coin waits; a third one a tick later lines up behind it
        // instead of playing before it.
        let mut requests: Vec<&[Sound]> = vec![&[CollectCoin, CollectCoin], &[CollectCoin]];
        requests.resize(12, &[]);
        let played = run_stagger(&mut stagger, &requests);
        let ticks: Vec<usize> = (0..played.len())
            .filter(|t| !played[*t].is_empty())
            .collect();
        let n = SAME_TICK_STAGGER_TICKS as usize;
        assert_eq!(ticks, vec![0, n, 2 * n]);
    }

    #[test]
    fn encoded_wav_decodes_back() {
        let data = vec![0.0, 0.5, -0.5, 0.25];
        let bytes = encode_wav(&data, 2, 44100);
        let mut reader = hound::WavReader::new(Cursor::new(bytes)).unwrap();
        assert_eq!(reader.spec().channels, 2);
        let back: Vec<i16> = reader.samples::<i16>().map(|s| s.unwrap()).collect();
        assert_eq!(back, vec![0, 16383, -16383, 8191]);
    }
}
