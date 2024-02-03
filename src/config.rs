use crate::game::units::{progress::Progress, quality::Quality};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Settings {
    pub max_cp: i32,
    pub max_durability: i32,
    pub max_progress: Progress,
    pub max_quality: Quality,
}
