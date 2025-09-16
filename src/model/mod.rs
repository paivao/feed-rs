use serde::Deserialize;

//mod entry;
pub mod entry;
pub mod feed;
pub mod user;

#[derive(Deserialize)]
pub(crate) struct Window {
    pub(crate) pos: i32,
    pub(crate) size: i32,
}