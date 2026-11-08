//! Carrying out what this module asked for.
//!
//! A move is an instant cut: the workspace slide is switched off for the
//! dispatch and put back after. The frames either side of it are identical, so
//! the cut is invisible — left on, the slide would show the user sliding onto a
//! workspace they never asked for.
//!
//! This compositor is configured in Lua, which refuses `hyprctl keyword`
//! ("keyword can't work with non-legacy parsers. Use eval."), so the animation
//! changes go through `eval` and the moves through a Lua dispatch expression.
//! The restored values mirror the `workspaces` animation in `hyprland.conf`;
//! there is no way to ask the compositor what they were.

use crate::action::ActionSpaces;
use crate::event::EventSpaces;
use crate::model::MoveTarget;
use crate::model::Rect;
use crate::model::WindowAddr;

pub fn perform(action: &ActionSpaces) -> Vec<EventSpaces> {
    match action {
        ActionSpaces::Look { id } => crate::probe::look(*id),
        ActionSpaces::Move { id, addr, to } => {
            move_window(addr, to);
            vec![EventSpaces::Finished { id: *id }]
        }
        // The capture has to happen here rather than as an action of its own:
        // actions are carried out on a thread each, so two of them would race
        // and the loser would photograph a window that is already gone.
        ActionSpaces::Minimize {
            id,
            addr,
            geometry,
            thumb,
            to,
        } => {
            let mut out = Vec::new();
            if capture(*geometry, thumb) {
                out.push(EventSpaces::Captured { addr: addr.clone() });
            }
            move_window(addr, to);
            out.push(EventSpaces::Finished { id: *id });
            out
        }
        ActionSpaces::Focus { workspace } => {
            dispatch(&format!("hl.dsp.focus({{ workspace = '{workspace}' }})"));
            Vec::new()
        }
        // Closing produces no event of its own: the compositor announces it,
        // and that is what asks for the next look.
        ActionSpaces::Close { addr } => {
            dispatch(&format!(
                "hl.dsp.window.close({{ window = 'address:{addr}' }})"
            ));
            Vec::new()
        }
        ActionSpaces::Forget { thumb } => {
            let _ = std::fs::remove_file(thumb);
            Vec::new()
        }
    }
}

fn move_window(addr: &WindowAddr, to: &MoveTarget) {
    let workspace = match to {
        MoveTarget::Empty => "empty".to_string(),
        MoveTarget::Workspace(w) => w.to_string(),
        MoveTarget::Hidden(name) => name.clone(),
    };
    // Following a window out of sight would leave the user staring at the
    // hidden workspace, which is the one place the compositor will not draw
    // anything. Every other destination is somewhere they meant to end up.
    let follow = !matches!(to, MoveTarget::Hidden(_));
    slide(false);
    dispatch(&format!(
        "hl.dsp.window.move({{ workspace = '{workspace}', follow = {follow}, window = 'address:{addr}' }})"
    ));
    slide(true);
}

/// Photograph a window, while there is still something on screen to photograph.
///
/// grim captures a rectangle of the *screen*, not a window, so anything on top
/// of this one is in the shot. Capturing a window's own buffer would mean
/// speaking `hyprland-toplevel-export-v1`, and the window being minimized is
/// almost always the focused one and so on top of everything anyway.
fn capture(geometry: Rect, thumb: &str) -> bool {
    if geometry.width <= 0 || geometry.height <= 0 {
        return false;
    }
    if let Some(dir) = std::path::Path::new(thumb).parent() {
        if let Err(e) = std::fs::create_dir_all(dir) {
            tracing::warn!(error = %e, "could not make room for a window thumbnail");
            return false;
        }
    }
    let Rect {
        x,
        y,
        width,
        height,
    } = geometry;
    if exec::ok("grim", &["-g", &format!("{x},{y} {width}x{height}"), thumb]) {
        return true;
    }
    tracing::warn!(
        thumb,
        "could not photograph a window on its way to the dock"
    );
    false
}

fn dispatch(lua: &str) {
    if !exec::ok("hyprctl", &["dispatch", lua]) {
        tracing::warn!(dispatch = lua, "the compositor refused a dispatch");
    }
}

/// Whether switching workspaces animates.
fn slide(on: bool) {
    let leaf = if on {
        "hl.animation({ leaf = 'workspaces', enabled = true, speed = 8, bezier = 'default' })"
    } else {
        "hl.animation({ leaf = 'workspaces', enabled = false })"
    };
    let _ = exec::ok("hyprctl", &["eval", leaf]);
}
