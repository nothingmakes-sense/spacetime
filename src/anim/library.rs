use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use log::info;

use super::clip::{load_clips, AnimClip};
use super::player::ClipId;

#[derive(Default)]
pub struct AnimLibrary {
    clips: HashMap<String, AnimClip>,
}

impl AnimLibrary {
    pub fn new() -> Self {
        Self {
            clips: HashMap::new(),
        }
    }

    pub fn load_file(&mut self, path: impl AsRef<Path>) -> Result<usize> {
        let clips = load_clips(path.as_ref())?;
        let n = clips.len();
        for c in clips {
            info!("anim clip '{}' ({:.2}s)", c.name, c.duration);
            self.clips.insert(c.name.clone(), c);
        }
        Ok(n)
    }

    pub fn get(&self, name: &str) -> Option<&AnimClip> {
        self.clips.get(name).or_else(|| {
            // tolerate missing KayKit variants
            match name {
                "Walking_A" => self.clips.get("Walking_B").or_else(|| self.clips.get("Walking_C")),
                "Running_A" => self.clips.get("Running_B"),
                "Idle_A" => self.clips.get("Idle_B").or_else(|| self.clips.get("T-Pose")),
                "Use_Item" => self.clips.get("Throw").or_else(|| self.clips.get("Interact")),
                "Jump_Idle" => self.clips.get("Jump_Full_Short"),
                _ => None,
            }
        })
    }

    pub fn get_id(&self, id: ClipId) -> Option<&AnimClip> {
        self.get(id.key())
    }

    pub fn contains(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    pub fn len(&self) -> usize {
        self.clips.len()
    }
}
