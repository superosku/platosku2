use crate::state::BoundingBox;
use crate::state::Dir;
use crate::state::Pos;
use crate::state::game_map::MapLike;

const GRAVITY: f32 = 0.0070;
const TERMINAL_VELOCITY: f32 = 0.90;

pub struct KinematicResult {
    pub new_bb: BoundingBox,
    pub on_bottom: bool,
    pub on_top: bool,
    pub on_left: bool,
    pub on_right: bool,
}

impl KinematicResult {
    pub fn on_something(&self) -> bool {
        self.on_bottom || self.on_top || self.on_left || self.on_right
    }
}

pub fn integrate_kinematic(
    world: &dyn MapLike,
    bb: &BoundingBox,
    gravity: bool,
) -> KinematicResult {
    let mut on_bottom = false;
    let mut on_top = false;
    let mut on_left = false;
    let mut on_right = false;

    // Horizontal sweep
    let (out_x, hit_x) = sweep_axis(world, bb.x, bb.y, bb.w, bb.h, bb.vx, Axis::X);

    if hit_x {
        if bb.vx > 0.0 {
            on_right = true;
        }
        if bb.vx < 0.0 {
            on_left = true;
        }
    }

    // Gravity
    let mut vy = if gravity {
        (bb.vy + GRAVITY).min(TERMINAL_VELOCITY)
    } else {
        bb.vy
    };

    // Vertical sweep
    let (out_y, hit_y) = sweep_axis(world, out_x, bb.y, bb.w, bb.h, vy, Axis::Y);

    if hit_y {
        if vy > 0.0 {
            on_bottom = true;
        }
        if vy < 0.0 {
            on_top = true;
        }
        vy = 0.0;
    }

    KinematicResult {
        new_bb: BoundingBox {
            x: out_x,
            y: out_y,
            w: bb.w,
            h: bb.h,
            vx: bb.vx,
            vy,
        },
        on_bottom,
        on_top,
        on_left,
        on_right,
    }
}

#[derive(Copy, Clone)]
enum Axis {
    X,
    Y,
}

pub const EPS: f32 = 0.0001;

fn sweep_axis(
    world: &dyn MapLike,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    delta: f32,
    axis: Axis,
) -> (f32, bool) {
    if delta == 0.0 {
        return (
            match axis {
                Axis::X => x,
                Axis::Y => y,
            },
            false,
        );
    }

    // Helper to test overlap at a candidate position along this axis.
    let overlaps_at = |t: f32| -> bool {
        match axis {
            Axis::X => world.overlaps_solid(t, y, w, h),
            Axis::Y => world.overlaps_solid(x, t, w, h),
        }
    };

    let start = match axis {
        Axis::X => x,
        Axis::Y => y,
    };
    let target = start + delta;

    // If we can move the full delta, do it.
    if !overlaps_at(target) {
        return (target, false);
    }

    // Otherwise, find the maximum safe movement with bisection.
    // We search distance in [0, |delta|].
    let dir = delta.signum();
    let max_dist = delta.abs();

    let mut lo = 0.0f32; // safe
    let mut hi = max_dist; // blocked (or assumed blocked since full move collides)

    // If we're already overlapping at the start, we can't really "resolve" here
    // without a depenetration pass. We'll just not move.
    if overlaps_at(start) {
        return (start, true);
    }

    // Bisection iterations: 12–20 is usually plenty for platformers.
    for _ in 0..16 {
        let mid = (lo + hi) * 0.5;
        let candidate = start + dir * mid;

        if overlaps_at(candidate) {
            hi = mid;
        } else {
            lo = mid;
        }
    }

    // Stop a tiny bit before the surface to avoid floating-point jitter.
    let final_dist = (lo - EPS).max(0.0);
    let final_pos = start + dir * final_dist;

    (final_pos, true)
}

pub fn check_and_snap_platforms(
    old_bb: &BoundingBox,
    new_bb: &mut BoundingBox,
    map: &dyn MapLike,
) -> bool {
    if (new_bb.y + new_bb.h).floor() > (old_bb.y + old_bb.h).floor()
        && (map.is_platform_at(
            new_bb.x.floor() as i32,
            (new_bb.y + new_bb.h).floor() as i32,
        ) || map.is_platform_at(
            (new_bb.x + new_bb.w).floor() as i32,
            (new_bb.y + new_bb.h).floor() as i32,
        ))
    {
        new_bb.vy = 0.0;
        let new_y = (old_bb.y + old_bb.h).floor() - old_bb.h - EPS * 8.0 + 1.0;
        new_bb.y = new_y;
        true
    } else {
        false
    }
}

pub fn check_and_snap_hang(
    old_bb: &BoundingBox,
    new_bb: &BoundingBox,
    map: &dyn MapLike,
    dir: Dir,
) -> Option<Pos> {
    // Must be moving downward
    if new_bb.y <= old_bb.y {
        return None;
    }

    let front_x = match dir {
        Dir::Right => new_bb.x + new_bb.w,
        Dir::Left => new_bb.x,
    };

    let wall_tx = match dir {
        Dir::Right => (front_x + EPS * 2.0).floor() as i32,
        Dir::Left => (front_x + EPS * 2.0).floor() as i32 - 1,
    };

    let dist_to_wall = match dir {
        Dir::Right => {
            let wall_left = wall_tx as f32;
            (wall_left - (new_bb.x + new_bb.w)).abs()
        }
        Dir::Left => {
            let wall_right = (wall_tx + 1) as f32;
            (new_bb.x - wall_right).abs()
        }
    };

    // Not close to the wall
    if dist_to_wall > EPS * 3.0 {
        return None;
    }

    // Check if we passed ledge
    let old_top = old_bb.y;
    let new_top = new_bb.y;
    if old_top.floor() == new_top.floor() {
        return None;
    }

    // Check if this is ledge
    let ledge_ty = new_top.floor() as i32;
    let ledge_y = ledge_ty as f32;
    // TODO: Use the non tile format to hang on bounding box or others as well?
    let solid_here = map.is_solid_at_tile(wall_tx, ledge_ty);
    let solid_above = map.is_solid_at_tile(wall_tx, ledge_ty - 1);
    if !solid_here || solid_above {
        return None;
    }

    // Snap in place
    let snapped_x = match dir {
        Dir::Right => (wall_tx as f32 - EPS) - new_bb.w,
        Dir::Left => (wall_tx + 1) as f32 + EPS,
    };
    let snapped_y = ledge_y; // top aligned to tile top edge

    // TODO: Maybe reject if snapped pose overlaps with solids?

    Some(Pos {
        x: snapped_x,
        y: snapped_y,
    })
}
