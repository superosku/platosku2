use crate::state::BoundingBox;
use crate::state::GameMap;

const GRAVITY: f32 = 0.007;
const TERMINAL_VELOCITY: f32 = 0.90;

pub fn integrate_kinematic(
    map: &GameMap,
    bb: &BoundingBox,
) -> (BoundingBox, bool) {
    // Horizontal attempt first
    let mut out_x = bb.x;
    let attempted_x = bb.x + bb.vx;
    if !collides_with_map(map, attempted_x, bb.y, bb.w, bb.h) {
        out_x = attempted_x;
    }

    // Apply gravity
    let mut out_vy = (bb.vy + GRAVITY).min(TERMINAL_VELOCITY);
    let mut out_y = bb.y;
    let mut on_ground = false;

    // Vertical move and resolve
    let attempted_y = bb.y + out_vy;
    if !collides_with_map(map, out_x, attempted_y, bb.w, bb.h) {
        out_y = attempted_y;
        on_ground = false;
    } else {
        // Collision while moving vertically: place the body flush against blocking tiles
        let epsilon = 0.001f32;
        let left_tx = (out_x).floor() as i32;
        let right_tx = ((out_x + bb.w - epsilon)).floor() as i32;

        if out_vy > 0.0 {
            // Falling: snap to the top of the first blocking tile below
            let bottom_ty_attempted = ((bb.y + bb.h + out_vy - epsilon)).floor() as i32;
            let mut landed = false;
            for tx in left_tx..=right_tx {
                let (base, _overlay) = map.get_at(tx, bottom_ty_attempted);
                if base != 0 {
                    let tile_top = bottom_ty_attempted as f32;
                    out_y = tile_top - bb.h;
                    landed = true;
                    break;
                }
            }
            if !landed {
                out_y = (map.height() - bb.h).max(0.0);
            }
            out_vy = 0.0;
            on_ground = true;
        } else if out_vy < 0.0 {
            // Moving up: snap to the bottom of the first blocking tile above
            let top_ty_attempted = ((bb.y + out_vy)).floor() as i32;
            let mut hit_ceiling = false;
            for tx in left_tx..=right_tx {
                let (base, _overlay) = map.get_at(tx, top_ty_attempted);
                if base != 0 {
                    let tile_bottom = (top_ty_attempted + 1) as f32;
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
    let clamped_x = out_x.clamp(0.0, (map.width() - bb.w).max(0.0));
    let clamped_y = out_y.clamp(0.0, (map.height() - bb.h).max(0.0));

    (
        BoundingBox {
            x: clamped_x,
            y: clamped_y,
            w: bb.w,
            h: bb.h,
            vx: 0.0,
            vy: out_vy,
        },
        on_ground
    )
    // (clamped_x, clamped_y, out_vy, on_ground)
}

pub fn collides_with_map(map: &GameMap, x: f32, y: f32, w: f32, h: f32) -> bool {
    // Treat outside of map bounds as blocking
    if x < 0.0 || y < 0.0 { return true; }
    if x + w > map.width() || y + h > map.height() { return true; }

    let epsilon = 0.001f32;
    let left_tx = (x).floor() as i32;
    let right_tx = ((x + w - epsilon)).floor() as i32;
    let top_ty = (y).floor() as i32;
    let bottom_ty = ((y + h- epsilon)).floor() as i32;

    for ty in top_ty..=bottom_ty {
        for tx in left_tx..=right_tx {
            let (base, _overlay) = map.get_at(tx, ty);
            if base != 0 { return true; }
        }
    }
    false
}


