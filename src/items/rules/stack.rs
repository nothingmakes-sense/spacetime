use super::catalog::{ItemId, BAG_SLOTS};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stack {
    pub item: ItemId,
    pub count: u16,
}

impl Stack {
    pub fn empty() -> Self {
        Self {
            item: ItemId::EMPTY,
            count: 0,
        }
    }

    pub fn new(item: ItemId, count: u16) -> Self {
        if item.is_empty() || count == 0 {
            Self::empty()
        } else {
            Self { item, count }
        }
    }

    pub fn is_empty(self) -> bool {
        self.item.is_empty() || self.count == 0
    }

    pub fn space(self) -> u16 {
        if self.is_empty() {
            0
        } else {
            self.item.def().stack.saturating_sub(self.count)
        }
    }

    /// Merge `other` into self. Returns leftover.
    pub fn absorb(&mut self, other: Stack) -> Stack {
        if other.is_empty() {
            return Stack::empty();
        }
        if self.is_empty() {
            let cap = other.item.def().stack;
            let take = other.count.min(cap);
            *self = Stack::new(other.item, take);
            return Stack::new(other.item, other.count - take);
        }
        if self.item != other.item {
            return other;
        }
        let take = other.count.min(self.space());
        self.count += take;
        Stack::new(other.item, other.count - take)
    }
}

pub fn encode_slots(slots: &[Stack]) -> String {
    slots
        .iter()
        .map(|s| format!("{}:{}", s.item.0, s.count))
        .collect::<Vec<_>>()
        .join("|")
}

pub fn decode_slots(packed: &str, len: usize) -> Vec<Stack> {
    let mut out = vec![Stack::empty(); len];
    if packed.is_empty() {
        return out;
    }
    for (i, part) in packed.split('|').enumerate() {
        if i >= len {
            break;
        }
        let mut it = part.split(':');
        let item = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        let count = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        out[i] = Stack::new(ItemId(item), count);
    }
    out
}

pub fn empty_bag() -> Vec<Stack> {
    vec![Stack::empty(); BAG_SLOTS]
}
