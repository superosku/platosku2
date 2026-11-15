//! The two shaders: sprites (also rectangles and the HUD) and tiles.

use miniquad::{ShaderMeta, UniformBlockLayout, UniformDesc, UniformType};

/// Sprites, colored rectangles (a white texture) and HUD images:
/// texture color times vertex color.
pub mod sprite {
    use super::*;

    pub const VERTEX: &str = r#"#version 100
attribute vec2 pos;
attribute vec2 uv;
attribute vec4 color;
uniform mat4 mvp;
varying lowp vec4 v_color;
varying vec2 v_uv;
void main() {
    gl_Position = mvp * vec4(pos, 0.0, 1.0);
    v_color = color;
    v_uv = uv;
}
"#;

    pub const FRAGMENT: &str = r#"#version 100
precision mediump float;
varying lowp vec4 v_color;
varying vec2 v_uv;
uniform sampler2D tex;
void main() {
    gl_FragColor = texture2D(tex, v_uv) * v_color;
}
"#;

    #[repr(C)]
    pub struct Uniforms {
        pub mvp: [f32; 16],
    }

    pub fn meta() -> ShaderMeta {
        ShaderMeta {
            images: vec!["tex".to_string()],
            uniforms: UniformBlockLayout {
                uniforms: vec![UniformDesc::new("mvp", UniformType::Mat4)],
            },
        }
    }
}

/// Dual-grid tiles. Magenta pixels in the tile image show a repeating
/// 64x64 region of the background texture instead, anchored to the world.
pub mod tile {
    use super::*;

    pub const VERTEX: &str = r#"#version 100
attribute vec2 pos;
attribute vec2 uv;
attribute vec4 color;
uniform mat4 mvp;
varying lowp vec4 v_color;
varying vec2 v_uv;
varying vec2 v_world;
void main() {
    gl_Position = mvp * vec4(pos, 0.0, 1.0);
    v_color = color;
    v_uv = uv;
    v_world = pos;
}
"#;

    pub const FRAGMENT: &str = r#"#version 100
precision mediump float;
varying lowp vec4 v_color;
varying vec2 v_uv;
varying vec2 v_world;
uniform sampler2D tex;
uniform sampler2D bg_tex;
uniform vec4 bg_region; // xy: top-left of the region in pixels, zw: its size
uniform vec4 bg_tex_size; // xy: background texture size in pixels
void main() {
    vec4 texel = texture2D(tex, v_uv);
    float is_key = step(distance(texel.rgb, vec3(1.0, 0.0, 1.0)), 0.01);
    vec2 region_uv = fract(v_world / bg_region.zw);
    vec2 uv_bg = (bg_region.xy + region_uv * bg_region.zw) / bg_tex_size.xy;
    vec4 bg = texture2D(bg_tex, uv_bg);
    gl_FragColor = mix(texel, bg, is_key) * v_color;
}
"#;

    #[repr(C)]
    pub struct Uniforms {
        pub mvp: [f32; 16],
        pub bg_region: [f32; 4],
        pub bg_tex_size: [f32; 4],
    }

    pub fn meta() -> ShaderMeta {
        ShaderMeta {
            images: vec!["tex".to_string(), "bg_tex".to_string()],
            uniforms: UniformBlockLayout {
                uniforms: vec![
                    UniformDesc::new("mvp", UniformType::Mat4),
                    UniformDesc::new("bg_region", UniformType::Float4),
                    UniformDesc::new("bg_tex_size", UniformType::Float4),
                ],
            },
        }
    }
}
