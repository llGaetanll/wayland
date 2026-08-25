//! One dock entry. The registry and the order are the same list, so there is no
//! second file to keep in sync.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DockApp {
    pub id: String,
    pub cmd: String,
    pub icon: String,
}
