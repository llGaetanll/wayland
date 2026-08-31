//! macOS-style fullscreen Spaces.
//!
//! A window that goes fullscreen is whisked onto a workspace of its own and
//! comes back to where it was when it leaves — so fullscreen reads as a Space
//! you switch to, not as a window that grew. The bar and dock hide while one is
//! focused, which is the one thing about this the bar asks about.
//!
//! This used to be a Lua closure in the compositor's config, two shell scripts
//! and a directory of files under `$XDG_RUNTIME_DIR`. The state those kept is
//! [`StateSpaces::origins`](state::StateSpaces) here, and the three timers they
//! needed are gone: see [`state`] for what replaced the debounce.

pub mod action;
pub mod event;
pub mod model;
pub mod perform;
pub mod probe;
pub mod state;

pub use action::ActionSpaces;
pub use event::EventSpaces;
pub use model::Client;
pub use model::MoveTarget;
pub use model::Snapshot;
pub use model::WindowAddr;
pub use model::WorkspaceId;
pub use state::StateSpaces;
