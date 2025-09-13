use crate::state::GameMap;

pub struct PhysicsParams {
    pub gravity: f32,
    pub terminal_velocity: f32,
}

impl Default for PhysicsParams {
    fn default() -> Self {
        PhysicsParams { gravity: 0.25, terminal_velocity: 12.0 }
    }
}

pub fn integrate_kinematic(
    map: &GameMap,
    x: f32,
    y: f32,
    size: f32,
    vy: f32,
    dx: f32,
    params: &PhysicsParams,
) -> (f32, f32, f32, bool) {
    // Horizontal attempt first
    let mut out_x = x;
    let attempted_x = x + dx;
    if !collides_with_map(map, attempted_x, y, size) {
        out_x = attempted_x;
    }

    // Apply gravity
    let mut out_vy = (vy + params.gravity).min(params.terminal_velocity);
    let mut out_y = y;
    let mut on_ground = false;

    // Vertical move and resolve
    let attempted_y = y + out_vy;
    if !collides_with_map(map, out_x, attempted_y, size) {
        out_y = attempted_y;
        on_ground = false;
    } else {
        // Collision while moving vertically: place the body flush against blocking tiles
        let epsilon = 0.001f32;
        let tile_size = map.tile_size;
        let left_tx = (out_x / tile_size).floor() as i32;
        let right_tx = ((out_x + size - epsilon) / tile_size).floor() as i32;

        if out_vy > 0.0 {
            // Falling: snap to the top of the first blocking tile below
            let bottom_ty_attempted = ((y + size + out_vy - epsilon) / tile_size).floor() as i32;
            let mut landed = false;
            for tx in left_tx..=right_tx {
                let (base, _overlay) = map.get_at(tx, bottom_ty_attempted);
                if base != 0 {
                    let tile_top = bottom_ty_attempted as f32 * tile_size;
                    out_y = tile_top - size;
                    landed = true;
                    break;
                }
            }
            if !landed {
                out_y = (map.height_px() - size).max(0.0);
            }
            out_vy = 0.0;
            on_ground = true;
        } else if out_vy < 0.0 {
            // Moving up: snap to the bottom of the first blocking tile above
            let top_ty_attempted = ((y + out_vy) / tile_size).floor() as i32;
            let mut hit_ceiling = false;
            for tx in left_tx..=right_tx {
                let (base, _overlay) = map.get_at(tx, top_ty_attempted);
                if base != 0 {
                    let tile_bottom = (top_ty_attempted + 1) as f32 * tile_size;
                    out_y = tile_bottom;
                    hit_ceiling = true;
                    break;
                }
            }
            if !hit_ceiling {
                out_y = 0.0;
            }
            out_vy = 0.0;
        }
    }

    // Final clamp to map bounds
    let clamped_x = out_x.clamp(0.0, (map.width_px() - size).max(0.0));
    let clamped_y = out_y.clamp(0.0, (map.height_px() - size).max(0.0));

    (clamped_x, clamped_y, out_vy, on_ground)
}

pub fn collides_with_map(map: &GameMap, x: f32, y: f32, size: f32) -> bool {
    // Treat outside of map bounds as blocking
    if x < 0.0 || y < 0.0 { return true; }
    if x + size > map.width_px() || y + size > map.height_px() { return true; }

    let epsilon = 0.001f32;
    let left_tx = (x / map.tile_size).floor() as i32;
    let right_tx = ((x + size - epsilon) / map.tile_size).floor() as i32;
    let top_ty = (y / map.tile_size).floor() as i32;
    let bottom_ty = ((y + size - epsilon) / map.tile_size).floor() as i32;

    for ty in top_ty..=bottom_ty {
        for tx in left_tx..=right_tx {
            let (base, _overlay) = map.get_at(tx, ty);
            if base != 0 { return true; }
        }
    }
    false
}


