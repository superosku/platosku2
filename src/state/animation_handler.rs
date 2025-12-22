pub struct AnimationConfigResult {
    start: u32,
    end: u32,
    dur: u32,
}

impl AnimationConfigResult {
    pub fn new(start: u32, end: u32, dur: u32) -> Self {
        Self { start, end, dur }
    }
}

pub trait AnimationConfig {
    fn get_config(&self) -> AnimationConfigResult;
}

pub struct AnimationHandler<T> {
    state: T,
    current_frame: u32,
}

impl<T: AnimationConfig + PartialEq> AnimationHandler<T> {
    pub fn new(initial_state: T) -> Self {
        AnimationHandler {
            state: initial_state,
            current_frame: 0,
        }
    }

    pub fn set_state(&mut self, new_state: T) {
        if self.state != new_state {
            self.current_frame = 0;
            self.state = new_state;
        }
    }

    pub fn increment_frame(&mut self) {
        self.current_frame += 1;
    }

    pub fn get_atlas_index(&self) -> u32 {
        let config = self.state.get_config();
        config.start + (self.current_frame / config.dur) % (config.end - config.start + 1)
    }
}
