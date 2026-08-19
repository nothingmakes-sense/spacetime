use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use glam::{Mat4, Quat, Vec3};
use gltf::animation::util::ReadOutputs;
use gltf::animation::Property;

use crate::assets::Skeleton;

#[derive(Clone, Debug, Default)]
pub struct JointTrack {
    pub t_times: Vec<f32>,
    pub t_vals: Vec<Vec3>,
    pub r_times: Vec<f32>,
    pub r_vals: Vec<Quat>,
    pub s_times: Vec<f32>,
    pub s_vals: Vec<Vec3>,
}

#[derive(Clone, Debug)]
pub struct AnimClip {
    pub name: String,
    pub duration: f32,
    pub tracks: HashMap<String, JointTrack>,
}

impl AnimClip {
    pub fn sample_locals(&self, skeleton: &Skeleton, time: f32) -> Vec<Mat4> {
        let t = if self.duration > 1e-4 {
            time.rem_euclid(self.duration)
        } else {
            0.0
        };
        let mut locals = skeleton.rest_locals();
        for (i, joint) in skeleton.joints.iter().enumerate() {
            let Some(track) = self.tracks.get(&joint.name) else {
                continue;
            };
            let rest = decompose(joint.rest_local);
            let p = sample_vec3(&track.t_times, &track.t_vals, t).unwrap_or(rest.0);
            let r = sample_quat(&track.r_times, &track.r_vals, t).unwrap_or(rest.1);
            let s = sample_vec3(&track.s_times, &track.s_vals, t).unwrap_or(rest.2);
            locals[i] = Mat4::from_scale_rotation_translation(s, r, p);
        }
        locals
    }
}

pub fn load_clips(path: impl AsRef<Path>) -> Result<Vec<AnimClip>> {
    let path = path.as_ref();
    let (doc, buffers, _images) =
        gltf::import(path).with_context(|| format!("import clips {}", path.display()))?;

    let mut clips = Vec::new();
    for animation in doc.animations() {
        let name = animation
            .name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("clip_{}", animation.index()));
        let mut tracks: HashMap<String, JointTrack> = HashMap::new();
        let mut duration = 0.0f32;

        for channel in animation.channels() {
            let reader = channel.reader(|b| Some(buffers[b.index()].as_ref()));
            let Some(inputs) = reader.read_inputs() else {
                continue;
            };
            let times: Vec<f32> = inputs.collect();
            if let Some(&last) = times.last() {
                duration = duration.max(last);
            }
            let node_name = channel
                .target()
                .node()
                .name()
                .unwrap_or("")
                .to_string();
            if node_name.is_empty() {
                continue;
            }
            let track = tracks.entry(node_name).or_default();
            let Some(outputs) = reader.read_outputs() else {
                continue;
            };
            match (channel.target().property(), outputs) {
                (Property::Translation, ReadOutputs::Translations(it)) => {
                    track.t_times = times;
                    track.t_vals = it.map(Vec3::from_array).collect();
                }
                (Property::Rotation, ReadOutputs::Rotations(rots)) => {
                    track.r_times = times;
                    track.r_vals = rots
                        .into_f32()
                        .map(|q| Quat::from_xyzw(q[0], q[1], q[2], q[3]).normalize())
                        .collect();
                }
                (Property::Scale, ReadOutputs::Scales(it)) => {
                    track.s_times = times;
                    track.s_vals = it.map(Vec3::from_array).collect();
                }
                _ => {}
            }
        }

        clips.push(AnimClip {
            name,
            duration: duration.max(1.0 / 30.0),
            tracks,
        });
    }
    Ok(clips)
}

fn decompose(m: Mat4) -> (Vec3, Quat, Vec3) {
    let (_, r, t) = m.to_scale_rotation_translation();
    let s = {
        let x = m.x_axis.truncate().length();
        let y = m.y_axis.truncate().length();
        let z = m.z_axis.truncate().length();
        Vec3::new(x, y, z)
    };
    (t, r.normalize(), s)
}

fn sample_vec3(times: &[f32], vals: &[Vec3], t: f32) -> Option<Vec3> {
    if times.is_empty() || vals.len() != times.len() {
        return None;
    }
    if times.len() == 1 || t <= times[0] {
        return Some(vals[0]);
    }
    let last = times.len() - 1;
    if t >= times[last] {
        return Some(vals[last]);
    }
    let i = times.partition_point(|x| *x <= t).saturating_sub(1);
    let j = (i + 1).min(last);
    let span = (times[j] - times[i]).max(1e-6);
    let u = ((t - times[i]) / span).clamp(0.0, 1.0);
    Some(vals[i].lerp(vals[j], u))
}

fn sample_quat(times: &[f32], vals: &[Quat], t: f32) -> Option<Quat> {
    if times.is_empty() || vals.len() != times.len() {
        return None;
    }
    if times.len() == 1 || t <= times[0] {
        return Some(vals[0]);
    }
    let last = times.len() - 1;
    if t >= times[last] {
        return Some(vals[last]);
    }
    let i = times.partition_point(|x| *x <= t).saturating_sub(1);
    let j = (i + 1).min(last);
    let span = (times[j] - times[i]).max(1e-6);
    let u = ((t - times[i]) / span).clamp(0.0, 1.0);
    Some(vals[i].slerp(vals[j], u))
}
