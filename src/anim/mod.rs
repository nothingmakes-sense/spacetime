//! Skeletal animation: clips, playback, retarget-by-name, CPU LBS.

mod clip;
mod library;
mod player;
mod skeleton;
mod skin;

pub use clip::{load_clips, AnimClip};
pub use library::AnimLibrary;
pub use player::{Animator, ClipId};
pub use skeleton::apply_sit;
pub use skin::skin_primitive;
