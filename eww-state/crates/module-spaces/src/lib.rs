//! Where every window lives: fullscreen Spaces, and minimizing into the dock.
//!
//! A window that goes fullscreen is whisked onto a workspace of its own and
//! comes back to where it was when it leaves — so fullscreen reads as a Space
//! you switch to, not as a window that grew. The bar and dock hide while one is
//! focused, which is the one thing about this the bar asks about.
//!
//! A minimized window goes the other way, onto a *special* workspace the
//! compositor never draws, having been photographed on the way out so the dock
//! has a tile to show. Clicking the tile sends it home again. There is no
//! minimize in Hyprland; this is the whole of it.
//!
//! This used to be a Lua closure in the compositor's config, two shell scripts
//! and a directory of files under `$XDG_RUNTIME_DIR`. The state those kept is
//! [`StateSpaces::origins`](state::StateSpaces) here, and the three timers they
//! needed are gone: see [`state`] for what replaced the debounce.

pub mod action;
pub mod config;
pub mod event;
pub mod model;
pub mod perform;
pub mod probe;
pub mod state;

pub use action::ActionSpaces;
pub use config::ConfigSpaces;
pub use event::EventSpaces;
pub use model::Client;
pub use model::Minimized;
pub use model::MoveTarget;
pub use model::Rect;
pub use model::Snapshot;
pub use model::WindowAddr;
pub use model::WorkspaceId;
pub use state::StateSpaces;
