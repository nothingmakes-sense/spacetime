use std::collections::HashMap;

use glam::Mat4;

#[derive(Clone, Debug)]
pub struct Joint {
    pub name: String,
    pub parent: Option<usize>,
    pub rest_local: Mat4,
}

#[derive(Clone, Debug)]
pub struct Skeleton {
    pub joints: Vec<Joint>,
    pub inverse_bind: Vec<Mat4>,
    pub by_name: HashMap<String, usize>,
}

impl Skeleton {
    pub fn new(joints: Vec<Joint>, inverse_bind: Vec<Mat4>) -> Self {
        let by_name = joints
            .iter()
            .enumerate()
            .map(|(i, j)| (j.name.clone(), i))
            .collect();
        Self {
            joints,
            inverse_bind,
            by_name,
        }
    }

    pub fn index(&self, name: &str) -> Option<usize> {
        self.by_name.get(name).copied()
    }

    pub fn rest_locals(&self) -> Vec<Mat4> {
        self.joints.iter().map(|j| j.rest_local).collect()
    }

    pub fn joint_worlds(&self, locals: &[Mat4]) -> Vec<Mat4> {
        let mut worlds = vec![Mat4::IDENTITY; self.joints.len()];
        for (i, joint) in self.joints.iter().enumerate() {
            let local = locals.get(i).copied().unwrap_or(joint.rest_local);
            worlds[i] = match joint.parent {
                Some(p) => worlds[p] * local,
                None => local,
            };
        }
        worlds
    }

    pub fn palette(&self, locals: &[Mat4]) -> Vec<Mat4> {
        self.joint_worlds(locals)
            .iter()
            .zip(self.inverse_bind.iter())
            .map(|(w, ibm)| *w * *ibm)
            .collect()
    }

    pub fn socket(&self, locals: &[Mat4], name: &str) -> Option<Mat4> {
        let i = self.index(name)?;
        Some(self.joint_worlds(locals)[i])
    }
}
