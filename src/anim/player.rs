use glam::Mat4;

use super::library::AnimLibrary;
use super::skeleton::apply_sit;
use crate::assets::Skeleton;

#[derive(Clone, Debug)]
struct Playing {
    name: String,
    time: f32,
    speed: f32,
    looping: bool,
}

/// One-shot / loop / cross-fade clip player.
#[derive(Clone, Debug)]
pub struct Animator {
    current: Option<Playing>,
    previous: Option<Playing>,
    fade: f32,
    fade_dur: f32,
    pub sit_amount: f32,
}

impl Default for Animator {
    fn default() -> Self {
        Self {
            current: None,
            previous: None,
            fade: 1.0,
            fade_dur: 0.12,
            sit_amount: 0.0,
        }
    }
}

impl Animator {
    pub fn play(&mut self, name: &str, looping: bool, speed: f32, fade: f32) {
        let already = self
            .current
            .as_ref()
            .is_some_and(|p| p.name == name && p.looping == looping && (p.speed - speed).abs() < 0.01);
        if already {
            return;
        }
        self.previous = self.current.take();
        self.fade = 0.0;
        self.fade_dur = fade.max(0.001);
        self.current = Some(Playing {
            name: name.to_string(),
            time: if speed < 0.0 { f32::MAX } else { 0.0 },
            speed,
            looping,
        });
    }

    pub fn tick(&mut self, dt: f32, lib: &AnimLibrary) {
        tick_playing(&mut self.current, dt, lib);
        tick_playing(&mut self.previous, dt, lib);
        self.fade = (self.fade + dt / self.fade_dur).min(1.0);
        if self.fade >= 1.0 {
            self.previous = None;
        }
    }

    pub fn finished(&self, lib: &AnimLibrary) -> bool {
        let Some(p) = &self.current else {
            return true;
        };
        if p.looping {
            return false;
        }
        lib.get(&p.name)
            .map(|c| p.time >= c.duration - 1.0 / 30.0)
            .unwrap_or(true)
    }

    pub fn current_name(&self) -> Option<&str> {
        self.current.as_ref().map(|p| p.name.as_str())
    }

    pub fn sample(&self, lib: &AnimLibrary, skeleton: &Skeleton) -> Vec<Mat4> {
        let rest = skeleton.rest_locals();
        let a = self
            .current
            .as_ref()
            .and_then(|p| lib.get(&p.name).map(|c| c.sample_locals(skeleton, p.time)))
            .unwrap_or(rest.clone());
        let mut locals = if let Some(prev) = &self.previous {
            if let Some(c) = lib.get(&prev.name) {
                let b = c.sample_locals(skeleton, prev.time);
                blend_locals(&b, &a, self.fade)
            } else {
                a
            }
        } else {
            a
        };
        apply_sit(skeleton, &mut locals, self.sit_amount);
        locals
    }
}

fn tick_playing(slot: &mut Option<Playing>, dt: f32, lib: &AnimLibrary) {
    let Some(p) = slot.as_mut() else {
        return;
    };
    let dur = lib.get(&p.name).map(|c| c.duration).unwrap_or(1.0);
    if p.time == f32::MAX {
        p.time = dur;
    }
    p.time += dt * p.speed;
    if p.looping {
        if dur > 1e-4 {
            p.time = p.time.rem_euclid(dur);
        }
    } else {
        p.time = p.time.clamp(0.0, dur);
    }
}

fn blend_locals(a: &[Mat4], b: &[Mat4], t: f32) -> Vec<Mat4> {
    a.iter()
        .zip(b.iter())
        .map(|(ma, mb)| {
            let (sa, ra, ta) = ma.to_scale_rotation_translation();
            let (sb, rb, tb) = mb.to_scale_rotation_translation();
            Mat4::from_scale_rotation_translation(
                sa.lerp(sb, t),
                ra.slerp(rb, t),
                ta.lerp(tb, t),
            )
        })
        .collect()
}

/// Logical clip names used by gameplay. Mapped onto KayKit file names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipId {
    Idle,
    Walk,
    Run,
    JumpStart,
    JumpAir,
    JumpLand,
    JumpFull,
    Attack,
    Interact,
    Hit,
}

impl ClipId {
    pub fn key(self) -> &'static str {
        match self {
            Self::Idle => "Idle_A",
            Self::Walk => "Walking_A",
            Self::Run => "Running_A",
            Self::JumpStart => "Jump_Start",
            Self::JumpAir => "Jump_Idle",
            Self::JumpLand => "Jump_Land",
            Self::JumpFull => "Jump_Full_Short",
            Self::Attack => "Use_Item",
            Self::Interact => "Interact",
            Self::Hit => "Hit_A",
        }
    }
}
