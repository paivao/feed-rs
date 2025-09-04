mod entry;
mod entry2;
mod feed;

pub struct Window(i64, i64);

impl Window {
    pub fn new(last_inserted: i64, size: i64) -> Window {
        Window(last_inserted, size)
    }
}
