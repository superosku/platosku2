use crate::state::BoundingBox;
use crate::state::Dir;
use crate::state::GameMap;
use crate::state::Pos;

const GRAVITY: f32 = 0.0070;
const TERMINAL_VELOCITY: f32 = 0.90;

pub struct KinematicResult {
    pub new_bb: BoundingBox,
    pub on_bottom: bool,
    pub on_top: bool,
    pub on_left: bool,
    pub on_right: bool,
}

pub fn integrate_kinematic(map: &GameMap, bb: &BoundingBox, gravity: bool) -> KinematicResult {
    let mut on_bottom = false;
    let mut on_top = false;
    let mut on_left = false;
    let mut on_right = false;

    // Horizontal attempt first
    let mut out_x = bb.x;
    let attempted_x = bb.x + bb.vx;
    if !collides_with_map(map, attempted_x, bb.y, bb.w, bb.h) {
        out_x = attempted_x;
    } else if bb.vx > 0.0 {
        on_right = true;
    } else if bb.vx < 0.0 {
        on_left = true;
    }

    // Apply gravity
    let mut out_vy = if gravity {
        (bb.vy + GRAVITY).min(TERMINAL_VELOCITY)
    } else {
        bb.vy
    };
    let mut out_y = bb.y;

    // Vertical move and resolve
    let attempted_y = bb.y + out_vy;
    if !collides_with_map(map, out_x, attempted_y, bb.w, bb.h) {
        out_y = attempted_y;
        on_bottom = false;
    } else {
        // Collision while moving vertically: place the body flush against blocking tiles
        let epsilon = 0.001f32;
        let left_tx = (out_x).floor() as i32;
        let right_tx = (out_x + bb.w - epsilon).floor() as i32;

        if out_vy > 0.0 {
            // Falling: snap to the top of the first blocking tile below
            let bottom_ty_attempted = (bb.y + bb.h + out_vy - epsilon).floor() as i32;
            let mut landed = false;
            for tx in left_tx..=right_tx {
                let is_solid = map.is_solid_at(tx, bottom_ty_attempted);
                if is_solid {
                    let tile_top = bottom_ty_attempted as f32;
                    out_y = tile_top - bb.h;
                    landed = true;
                    break;
                }
            }
            if !landed {
                // out_y = (map.height() - bb.h).max(0.0);
            }
            out_vy = 0.0;
            on_bottom = true;
        } else if out_vy < 0.0 {
            // Moving up: snap to the bottom of the first blocking tile above
            let top_ty_attempted = (bb.y + out_vy).floor() as i32;
            let mut hit_ceiling = false;
            for tx in left_tx..=right_tx {
                let is_solid = map.is_solid_at(tx, top_ty_attempted);
                if is_solid {
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
            on_top = true;
        }
    }

    // // Final clamp to map bounds
    // let clamped_x = out_x.clamp(0.0, (map.width() - bb.w).max(0.0));
    // let clamped_y = out_y.clamp(0.0, (map.height() - bb.h).max(0.0));
    let clamped_x = out_x;
    let clamped_y = out_y;

    KinematicResult {
        new_bb: BoundingBox {
            x: clamped_x,
            y: clamped_y,
            w: bb.w,
            h: bb.h,
            vx: 0.0,
            vy: out_vy,
        },
        on_bottom,
        on_top,
        on_left,
        on_right,
    }
}

pub fn collides_with_map(map: &GameMap, x: f32, y: f32, w: f32, h: f32) -> bool {
    // Treat outside of map bounds as blocking
    // if x < 0.0 || y < 0.0 {
    //     return true;
    // }
    // if x + w > map.width() || y + h > map.height() {
    //     return true;
    // }

    let epsilon = 0.001f32;
    let left_tx = (x).floor() as i32;
    let right_tx = (x + w - epsilon).floor() as i32;
    let top_ty = (y).floor() as i32;
    let bottom_ty = (y + h - epsilon).floor() as i32;

    for ty in top_ty..=bottom_ty {
        for tx in left_tx..=right_tx {
            let is_solid = map.is_solid_at(tx, ty);
            if is_solid {
                return true;
            }
        }
    }
    false
}

pub fn check_and_snap_hang(
    bb: &BoundingBox,
    new_bb: &BoundingBox,
    map: &GameMap,
    dir: Dir,
) -> Option<Pos> {
    // Check if top of bb is above a tile and new_bb is below the tile
    if bb.y.floor() == new_bb.y.floor() {
        return None;
    }

    let tile_y = new_bb.y.floor() as i32; // tile row at player's top
    let tile_x = new_bb.x.floor() as i32; // tile column at player's left

    // // Determine tile row at the player's top
    // let ty = self.y.floor() as i32;

    // Horizontal adjacency check and side tile to test
    let eps_side = 0.10;

    let tile_x_check = if dir == Dir::Right {
        let dist_to_right = (new_bb.x + new_bb.w) - (tile_x as f32 + 1.0);
        if dist_to_right > eps_side {
            return None;
        }
        tile_x + 1
    } else {
        let dist_to_left = new_bb.x - (tile_x as f32);
        if dist_to_left > eps_side {
            return None;
        }
        tile_x - 1
    };

    // if !touching_side { return None; }

    // Ledge condition: side tile is blocked at ty, but open above (ty-1)
    let solid_here = map.is_solid_at(tile_x_check, tile_y);
    let solid_above = map.is_solid_at(tile_x_check, tile_y - 1);
    if !solid_here || solid_above {
        return None;
    }

    // Snap Y to sit slightly below the tile top
    // let snapped_y = ty as f32 + 0.02;
    Some(Pos {
        x: if dir == Dir::Left {
            new_bb.x.floor()
        } else {
            new_bb.x.floor() + (1.0 - new_bb.w)
        },
        y: new_bb.y.floor(),
    })
}
