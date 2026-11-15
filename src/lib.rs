// Many constructors load files or create GPU/audio resources, so a `Default`
// impl would be misleading.
#![allow(clippy::new_without_default)]

pub mod atlas_info;
pub mod camera;
pub mod debug_menu;
pub mod minimap;
pub mod physics;
pub mod render;
pub mod script;
pub mod sim;
pub mod sound_handler;
pub mod state;
pub mod watch;
