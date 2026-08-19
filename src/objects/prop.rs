use crate::scene::{DrawRequest, GameObject, Object, ObjectKind, TickCtx};
use crate::vulkan::ModelHandle;

/// Static mesh (crates, ground decorations). Drop-in template for new props.
pub struct PropObject {
    pub base: Object,
    pub handle: ModelHandle,
}

impl PropObject {
    pub fn new(mut base: Object, handle: ModelHandle) -> Self {
        base.kind = ObjectKind::Prop;
        Self { base, handle }
    }
}

impl GameObject for PropObject {
    fn base(&self) -> &Object {
        &self.base
    }
    fn base_mut(&mut self) -> &mut Object {
        &mut self.base
    }

    fn tick(&mut self, _ctx: &mut TickCtx) {}

    fn draws(&self) -> Vec<DrawRequest> {
        vec![DrawRequest {
            handle: self.handle,
            model: self.base.transform.matrix(),
        }]
    }
}
