use std::collections::HashSet;
use miniquad::{FilterMode, MipmapFilterMode, TextureWrap, UniformsSource};
use crate::camera::Camera;
use crate::render::{Renderer, TextureIndexes, TextureInfo, Uniforms};
use crate::state::game_map::{GameMap, MapLike};

struct MinimapLocation {
    // top left in pixels
    x: f32,
    y: f32,
    // h and w in pixels
    size: f32,
    // What to show from the map
    uv_base: [f32; 4],
    uv_scale: [f32; 4],
}

fn minimap_location_mix(first: &MinimapLocation, second: &MinimapLocation, mix: f32) -> MinimapLocation {
    MinimapLocation {
        x: (first.x as f32 * mix + second.x as f32 * (1.0 - mix)),
        y: (first.y as f32 * mix + second.y as f32 * (1.0 - mix)),
        size: (first.size as f32 * mix + second.size as f32 * (1.0 - mix)),
        uv_base: [
            first.uv_base[0] * mix + second.uv_base[0] * (1.0 - mix),
            first.uv_base[1] * mix + second.uv_base[1] * (1.0 - mix),
            first.uv_base[2] * mix + second.uv_base[2] * (1.0 - mix),
            first.uv_base[3] * mix + second.uv_base[3] * (1.0 - mix),
        ],
        uv_scale: [
            first.uv_scale[0] * mix + second.uv_scale[0] * (1.0 - mix),
            first.uv_scale[1] * mix + second.uv_scale[1] * (1.0 - mix),
            first.uv_scale[2] * mix + second.uv_scale[2] * (1.0 - mix),
            first.uv_scale[3] * mix + second.uv_scale[3] * (1.0 - mix),
        ]
    }
}

pub struct Minimap {
    minimap_smooth_center: Option<(f32, f32)>, // Smooth the transition of the centered room
    visited_rooms: HashSet<usize>, // Define what rooms to show
    previous_room_index: Option<usize>, // Used for defining when to recreate the minimap texture
    location: Option<MinimapLocation>,
}

const MINIMAP_LOCATION_LERP: f32 = 0.13;
const MINIMAP_CENTER_LERP: f32 = 0.13;

impl Minimap {
    pub fn new() -> Minimap {
        let mut visited_rooms = HashSet::new();
        // for i in 0..100 {
        //     visited_rooms.insert(i);
        // }
        Minimap {
            minimap_smooth_center: None,
            // visited_rooms: HashSet::new(),
            visited_rooms,
            previous_room_index: None,
            location: None,
        }
    }

    fn update_minimap_size_and_location(&mut self, map: &GameMap,  camera: &Camera, draw_big: bool, smooth_center: (f32, f32), texture_width: u32, texture_height: u32) {
        let (start_x, start_y, map_width, map_height) = map.get_bounds();

        let desired_location = if draw_big {
            let max_dim = texture_width.max(texture_height);
            let scale_x = max_dim as f32 / texture_width as f32;
            let scale_y = max_dim as f32/ texture_height as f32;
            let uv_base = [
                (1.0 - scale_x) * 0.5,
                (1.0 - scale_y) * 0.5,
                0.0,
                0.0
            ];
            let uv_scale = [
                scale_x,
                scale_y,
                0.0,
                0.0
            ];

            let size = camera.screen_h * 0.6;
            MinimapLocation {
                y: (camera.screen_h - size) * 0.5,
                x: (camera.screen_w - size) * 0.5,
                size,
                uv_base,
                uv_scale,
            }
        } else {
            let minimap_show_size = 15; // How many tiles to show in each direction
            let uv_base = [
                (smooth_center.0 - start_x as f32 - minimap_show_size as f32) / texture_width as f32,
                (smooth_center.1 - start_y as f32 - minimap_show_size as f32) / texture_height as f32,
                0.0,
                0.0
            ];
            let uv_scale = [
                minimap_show_size as f32 * 2.0 / texture_width as f32,
                minimap_show_size as f32 * 2.0 / texture_height as f32,
                0.0,
                0.0
            ];

            let size = (camera.screen_h * 0.2) ;
            MinimapLocation {
                y: 40.0,
                x: camera.screen_w - size - 20.0,
                size,
                uv_base,
                uv_scale,
            }
        };

        self.location = Some(if let Some(location) = &self.location {
            minimap_location_mix(
                location,
                &desired_location,
                1.0 - MINIMAP_LOCATION_LERP
            )
        } else {
            desired_location
        })
    }

    pub fn update_and_draw_minimap(
        &mut self,
        renderer: &mut Renderer,
        camera: &Camera,
        map: &GameMap,
        current_room_index: usize,
        draw_big: bool, // Controls if is drawn into the corner of as big one on the center of the screen
    ) {
        self.visited_rooms.insert(current_room_index);

        let smooth_center = self.update_and_get_minimap_smooth_center(map, current_room_index);

        // Redraw texture only when the current room changes
        if self.previous_room_index.is_none() || self.previous_room_index.unwrap() != current_room_index {
            self.previous_room_index = Some(current_room_index);
            let (pixels, texture_width, texture_height) = self.construct_minimap_image(map, current_room_index);
            self.update_minimap_texture_with_pixels(renderer, pixels, texture_width, texture_height);
        }

        let minimap_info = renderer.textures.get(&TextureIndexes::Minimap).unwrap();
        let current_size = renderer.ctx.texture_size(minimap_info.texture);

        self.update_minimap_size_and_location(map, camera, draw_big, smooth_center, current_size.0, current_size.1);

        self.draw_minimap(renderer, camera, map, smooth_center, current_size.0, current_size.1);
    }

    pub fn update_and_get_minimap_smooth_center(
        &mut self,
        map: &GameMap,
        current_room_index: usize,
    ) -> (f32, f32) {
        // Smoothing the current room center
        let room = &map.rooms[current_room_index];
        let (x, y) = room.get_pos();
        let cx = x as f32 + room.w as f32 / 2.0;
        let cy = y as f32 + room.h as f32 / 2.0;
        let desired_minimap_center = (cx, cy);

        let smooth_center = match self.minimap_smooth_center {
            None => desired_minimap_center,
            Some(sc) => (
                sc.0 + (desired_minimap_center.0 - sc.0) * MINIMAP_CENTER_LERP,
                sc.1 + (desired_minimap_center.1 - sc.1) * MINIMAP_CENTER_LERP,
            ),
        };
        self.minimap_smooth_center = Some(smooth_center);
        smooth_center
    }

    pub fn construct_minimap_image(
        &mut self,
        map: &GameMap,
        current_room_index: usize,
    ) -> (Vec<u8>, u32, u32) {
        // Construct the minimap image
        let (start_x, start_y, map_width, map_height) = map.get_bounds();

        let map_width = map_width.min(1024);
        let map_height = map_height.min(1024);

        // One pixel transparent padding around the map so clamped UVs show transparent at edges
        const PAD: u32 = 1;
        let tex_w_pad = map_width + 2 * PAD;
        let tex_h_pad = map_height + 2 * PAD;

        const MINIMAP_BORDER_COLOR: [u8; 4] = [255, 255, 255, 255]; // white (room outlines only)
        const MINIMAP_CURRENT_ROOM_COLOR: [u8; 4] = [173, 216, 230, 255]; // light blue (current room interior)
        const MINIMAP_OTHER_ROOM_COLOR: [u8; 4] = [200, 200, 205, 255]; // light gray (other room interior)
        // const MINIMAP_DOOR_COLOR: [u8; 4] = [230, 230, 230, 255]; // light gray (other room interior)
        const MINIMAP_DOOR_COLOR: [u8; 4] = [200, 200, 205, 255]; // light gray (other room interior)
        const TRANSPARENT: [u8; 4] = [0, 0, 0, 0];

        let mut pixels: Vec<u8> = Vec::with_capacity((tex_w_pad * tex_h_pad * 4) as usize);

        for py_pad in 0..tex_h_pad {
            for px_pad in 0..tex_w_pad {
                if px_pad < PAD || px_pad >= map_width + PAD || py_pad < PAD || py_pad >= map_height + PAD {
                    // pixels.extend_from_slice(&[255, 0, 0, 0]);
                    // pixels.extend_from_slice(&[255, 128, 0, 255]);
                    pixels.extend_from_slice(&TRANSPARENT);
                    continue
                }

                let tx = (px_pad - PAD) as i32 + start_x;
                let ty = (py_pad - PAD) as i32 + start_y;

                if map.is_room_border_for_some_room(tx, ty, &self.visited_rooms) {
                    if map.is_door_at_i(tx, ty) {
                        pixels.extend_from_slice(&MINIMAP_DOOR_COLOR);
                    } else {
                        pixels.extend_from_slice(&MINIMAP_BORDER_COLOR);
                    }
                } else if let Some((index, _room)) = map.get_room_at_i(tx, ty) {
                    let color = if !self.visited_rooms.contains(&index) {
                        &TRANSPARENT
                    } else if current_room_index == index {
                        &MINIMAP_CURRENT_ROOM_COLOR
                    } else {
                        &MINIMAP_OTHER_ROOM_COLOR
                    };
                    pixels.extend_from_slice(color);
                } else {
                    pixels.extend_from_slice(&TRANSPARENT);
                }
            }
        }

        (pixels, tex_w_pad, tex_h_pad)
    }

    pub fn update_minimap_texture_with_pixels(
        &mut self,
        renderer: &mut Renderer,
        pixels: Vec<u8>,
        texture_width: u32,
        texture_height: u32,
    ) {
        // Update the texture
        let minimap_info = renderer.textures.get(&TextureIndexes::Minimap).unwrap();
        let current_size = renderer.ctx.texture_size(minimap_info.texture);

        let need_new_texture =
            current_size.0 != texture_width || current_size.1 != texture_height;

        if need_new_texture {
            let new_tex = renderer.ctx.new_texture_from_rgba8(
                texture_width as u16,
                texture_height as u16,
                &pixels,
            );
            renderer.ctx.texture_set_filter(new_tex, FilterMode::Nearest, MipmapFilterMode::None);
            renderer.ctx.texture_set_wrap(new_tex, TextureWrap::Clamp, TextureWrap::Clamp);
            renderer.textures.insert(
                TextureIndexes::Minimap,
                TextureInfo::new(texture_width as f32, texture_height as f32, new_tex),
            );
        } else {
            renderer.ctx.texture_update(minimap_info.texture, &pixels);
        }
    }

    pub fn draw_minimap(
        &mut self,
        renderer: &mut Renderer,
        camera: &Camera,
        map: &GameMap,
        smooth_center: (f32, f32),
        texture_width: u32,
        texture_height: u32,
    ) {
        // Draw the minimap
        let (start_x, start_y, map_width, map_height) = map.get_bounds();

        let minimap_info = renderer.textures.get(&TextureIndexes::Minimap).unwrap();
        let background = renderer.textures.get(&TextureIndexes::TileBackground).unwrap();

        const MINIMAP_VIEW_TILES: i32 = 30;
        const MINIMAP_VIEW_HALF: i32 = MINIMAP_VIEW_TILES / 2; // 15

        renderer.bindings.images[0] = minimap_info.texture;
        renderer.bindings.images[1] = background.texture;
        renderer.ctx.apply_bindings(&renderer.bindings);


        if let Some(location) = &self.location {
            // TODO: These into a function. Make them non pub and two functions for getting the mvp depending on if game or hud.
            let proj = Renderer::ortho_mvp(camera);
            let model = Renderer::mat4_mul(
                Renderer::mat4_translation(location.x as f32, location.y as f32),
                Renderer::mat4_scale(location.size, location.size)
            );
            let mvp = Renderer::mat4_mul(proj, model);

            // Where on the screen to draw the minimap
            let world_base = [location.x as f32, location.y as f32, 0.0, 0.0];
            let world_scale = [location.size as f32, location.size as f32, 0.0, 0.0];

            let uniforms = Uniforms {
                mvp,
                color: [1.0, 1.0, 1.0, 1.0],
                uv_base: location.uv_base,
                uv_scale: location.uv_scale,
                world_base,
                world_scale,
                color_key: [1.0, 0.0, 1.0, 0.01],
                bg_tile_size: [64.0, 64.0, 0.0, 0.0],
                bg_region_origin: [0.0, 0.0, 0.0, 0.0],
                bg_tex_size: [background.w, background.h, 0.0, 0.0],
            };

            renderer.ctx.apply_uniforms(UniformsSource::table(&uniforms));
            renderer.ctx.draw(0, 6, 1);
        }
    }
}


