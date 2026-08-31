//! Carrying out what the Spaces module asked for.
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

pub fn perform(action: &ActionSpaces) -> Vec<EventSpaces> {
    match action {
        ActionSpaces::Look { id } => crate::probe::look(*id),
        ActionSpaces::Move { id, addr, to } => {
            let workspace = match to {
                MoveTarget::Empty => "empty".to_string(),
                MoveTarget::Workspace(w) => w.to_string(),
            };
            slide(false);
            dispatch(&format!(
                "hl.dsp.window.move({{ workspace = '{workspace}', follow = true, window = 'address:{addr}' }})"
            ));
            slide(true);
            vec![EventSpaces::Finished { id: *id }]
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
    }
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
