#![allow(unused, clippy::all)]
use spacetimedb_sdk::__codegen::{self as __sdk, __lib, __sats, __ws};

macro_rules! impl_reducer {
    ($trait:ident, $method:ident, $then:ident, $Args:ident, $variant:ident { $($field:ident : $ty:ty),* $(,)? }) => {
        #[derive(__lib::ser::Serialize, __lib::de::Deserialize, Clone, PartialEq, Debug)]
        #[sats(crate = __lib)]
        pub(super) struct $Args {
            $(pub $field: $ty,)*
        }

        impl From<$Args> for super::Reducer {
            fn from(args: $Args) -> Self {
                Self::$variant { $($field: args.$field),* }
            }
        }

        impl __sdk::InModule for $Args {
            type Module = super::RemoteModule;
        }

        #[allow(non_camel_case_types)]
        pub trait $trait {
            fn $method(&self, $($field: $ty),*) -> __sdk::Result<()> {
                self.$then($($field,)* |_, _| {})
            }
            fn $then(
                &self,
                $($field: $ty,)*
                callback: impl FnOnce(&super::ReducerEventContext, Result<Result<(), String>, __sdk::InternalError>)
                    + Send
                    + 'static,
            ) -> __sdk::Result<()>;
        }

        impl $trait for super::RemoteReducers {
            fn $then(
                &self,
                $($field: $ty,)*
                callback: impl FnOnce(&super::ReducerEventContext, Result<Result<(), String>, __sdk::InternalError>)
                    + Send
                    + 'static,
            ) -> __sdk::Result<()> {
                self.imp.invoke_reducer_with_callback(
                    $Args { $($field),* },
                    callback,
                )
            }
        }
    };
}

impl_reducer!(pickup_loot, pickup_loot, pickup_loot_then, PickupLootArgs, PickupLoot { loot_id: u64 });
impl_reducer!(drop_selected, drop_selected, drop_selected_then, DropSelectedArgs, DropSelected {});
impl_reducer!(select_slot, select_slot, select_slot_then, SelectSlotArgs, SelectSlot { slot: u8 });
impl_reducer!(swap_slots, swap_slots, swap_slots_then, SwapSlotsArgs, SwapSlots { a: u8, b: u8 });
impl_reducer!(
    transfer_station,
    transfer_station,
    transfer_station_then,
    TransferStationArgs,
    TransferStation {
        station_id: u64,
        bag_slot: u8,
        st_slot: u8,
        to_station: bool,
    }
);
impl_reducer!(craft, craft, craft_then, CraftArgs, Craft { recipe_id: u16 });
