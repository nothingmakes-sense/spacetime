use super::catalog::ItemId;
use super::recipe::{smelt_of, Recipe};
use super::stack::Stack;

pub fn insert_stack(slots: &mut [Stack], stack: Stack) -> Stack {
    let mut left = stack;
    if left.is_empty() {
        return left;
    }
    for s in slots.iter_mut() {
        if s.item == left.item {
            left = s.absorb(left);
            if left.is_empty() {
                return left;
            }
        }
    }
    for s in slots.iter_mut() {
        if s.is_empty() {
            left = s.absorb(left);
            if left.is_empty() {
                return left;
            }
        }
    }
    left
}

pub fn take_inputs(slots: &mut [Stack], recipe: &Recipe) -> bool {
    for ing in recipe.inputs {
        if count_item(slots, ing.item) < ing.count {
            return false;
        }
    }
    for ing in recipe.inputs {
        let mut need = ing.count;
        for s in slots.iter_mut() {
            if s.item != ing.item || need == 0 {
                continue;
            }
            let take = s.count.min(need);
            s.count -= take;
            need -= take;
            if s.count == 0 {
                *s = Stack::empty();
            }
        }
    }
    true
}

pub fn count_item(slots: &[Stack], item: ItemId) -> u16 {
    slots
        .iter()
        .filter(|s| s.item == item)
        .map(|s| s.count)
        .sum()
}

pub fn take_one(slots: &mut [Stack], index: usize) -> Option<Stack> {
    let slot = slots.get_mut(index)?;
    if slot.is_empty() {
        return None;
    }
    let drop = Stack::new(slot.item, 1);
    slot.count -= 1;
    if slot.count == 0 {
        *slot = Stack::empty();
    }
    Some(drop)
}

pub fn first_compatible(slots: &[Stack], stack: Stack) -> Option<usize> {
    if stack.is_empty() {
        return None;
    }
    slots
        .iter()
        .position(|s| s.item == stack.item && s.space() > 0)
        .or_else(|| slots.iter().position(|s| s.is_empty()))
}

pub fn first_nonempty(slots: &[Stack]) -> Option<usize> {
    slots.iter().position(|s| !s.is_empty())
}

/// One 50 ms furnace tick. Slots are `[input, fuel, output]`.
pub fn step_furnace(slots: &mut [Stack], fuel: &mut u32, cook: &mut u32) {
    if slots.len() < 3 {
        return;
    }
    if *fuel == 0 {
        let fuel_item = slots[1];
        let units = fuel_item.item.def().fuel;
        if units > 0 && fuel_item.count > 0 {
            slots[1].count -= 1;
            if slots[1].count == 0 {
                slots[1] = Stack::empty();
            }
            *fuel = units;
        }
    }
    if *fuel == 0 {
        *cook = 0;
        return;
    }
    *fuel -= 1;
    let Some(recipe) = smelt_of(slots[0].item) else {
        *cook = 0;
        return;
    };
    *cook += 1;
    if *cook < recipe.smelt_ticks {
        return;
    }
    let leftover = slots[2].absorb(recipe.output.into());
    if leftover.is_empty() {
        slots[0].count = slots[0].count.saturating_sub(1);
        if slots[0].count == 0 {
            slots[0] = Stack::empty();
        }
        *cook = 0;
    }
}
