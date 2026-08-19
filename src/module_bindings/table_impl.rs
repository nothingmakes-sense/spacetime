//! Boilerplate for a public PK table. Keep in sync with the generated
//! `player_table.rs` style so `spacetime generate` output can replace it.

#![allow(unused, clippy::all)]

#[macro_export]
macro_rules! impl_spacetime_table {
    (
        row = $Row:ident,
        table = $table:literal,
        accessor = $accessor:ident,
        handle = $Handle:ident,
        access_trait = $Access:ident,
        query_trait = $Query:ident,
        pk = $pk:ident : $Pk:ty,
        unique = $Unique:ident
    ) => {
        pub struct $Handle<'ctx> {
            imp: __sdk::TableHandle<$Row>,
            ctx: std::marker::PhantomData<&'ctx super::RemoteTables>,
        }

        pub trait $Access {
            #[allow(non_snake_case)]
            fn $accessor(&self) -> $Handle<'_>;
        }

        impl $Access for super::RemoteTables {
            fn $accessor(&self) -> $Handle<'_> {
                $Handle {
                    imp: self.imp.get_table::<$Row>($table),
                    ctx: std::marker::PhantomData,
                }
            }
        }

        pub struct InsertCallbackId(__sdk::CallbackId);
        pub struct DeleteCallbackId(__sdk::CallbackId);
        pub struct UpdateCallbackId(__sdk::CallbackId);

        impl<'ctx> __sdk::TableLike for $Handle<'ctx> {
            type Row = $Row;
            type EventContext = super::EventContext;
            fn count(&self) -> u64 {
                self.imp.count()
            }
            fn iter(&self) -> impl Iterator<Item = $Row> + '_ {
                self.imp.iter()
            }
        }

        impl<'ctx> __sdk::Table for $Handle<'ctx> {
            type Row = $Row;
            type EventContext = super::EventContext;
            fn count(&self) -> u64 {
                self.imp.count()
            }
            fn iter(&self) -> impl Iterator<Item = $Row> + '_ {
                self.imp.iter()
            }
            type InsertCallbackId = InsertCallbackId;
            fn on_insert(
                &self,
                callback: impl FnMut(&Self::EventContext, &Self::Row) + Send + 'static,
            ) -> InsertCallbackId {
                InsertCallbackId(self.imp.on_insert(Box::new(callback)))
            }
            fn remove_on_insert(&self, callback: InsertCallbackId) {
                self.imp.remove_on_insert(callback.0)
            }
            type DeleteCallbackId = DeleteCallbackId;
            fn on_delete(
                &self,
                callback: impl FnMut(&Self::EventContext, &Self::Row) + Send + 'static,
            ) -> DeleteCallbackId {
                DeleteCallbackId(self.imp.on_delete(Box::new(callback)))
            }
            fn remove_on_delete(&self, callback: DeleteCallbackId) {
                self.imp.remove_on_delete(callback.0)
            }
        }

        impl<'ctx> __sdk::WithInsert for $Handle<'ctx> {
            type InsertCallbackId = InsertCallbackId;
            fn on_insert(
                &self,
                callback: impl FnMut(&super::EventContext, &$Row) + Send + 'static,
            ) -> InsertCallbackId {
                InsertCallbackId(self.imp.on_insert(Box::new(callback)))
            }
            fn remove_on_insert(&self, callback: InsertCallbackId) {
                self.imp.remove_on_insert(callback.0)
            }
        }

        impl<'ctx> __sdk::WithDelete for $Handle<'ctx> {
            type DeleteCallbackId = DeleteCallbackId;
            fn on_delete(
                &self,
                callback: impl FnMut(&super::EventContext, &$Row) + Send + 'static,
            ) -> DeleteCallbackId {
                DeleteCallbackId(self.imp.on_delete(Box::new(callback)))
            }
            fn remove_on_delete(&self, callback: DeleteCallbackId) {
                self.imp.remove_on_delete(callback.0)
            }
        }

        impl<'ctx> __sdk::TableWithPrimaryKey for $Handle<'ctx> {
            type UpdateCallbackId = UpdateCallbackId;
            fn on_update(
                &self,
                callback: impl FnMut(&super::EventContext, &$Row, &$Row) + Send + 'static,
            ) -> UpdateCallbackId {
                UpdateCallbackId(self.imp.on_update(Box::new(callback)))
            }
            fn remove_on_update(&self, callback: UpdateCallbackId) {
                self.imp.remove_on_update(callback.0)
            }
        }

        impl<'ctx> __sdk::WithUpdate for $Handle<'ctx> {
            type UpdateCallbackId = UpdateCallbackId;
            fn on_update(
                &self,
                callback: impl FnMut(&super::EventContext, &$Row, &$Row) + Send + 'static,
            ) -> UpdateCallbackId {
                UpdateCallbackId(self.imp.on_update(Box::new(callback)))
            }
            fn remove_on_update(&self, callback: UpdateCallbackId) {
                self.imp.remove_on_update(callback.0)
            }
        }

        pub struct $Unique<'ctx> {
            imp: __sdk::UniqueConstraintHandle<$Row, $Pk>,
            phantom: std::marker::PhantomData<&'ctx super::RemoteTables>,
        }

        impl<'ctx> $Handle<'ctx> {
            pub fn $pk(&self) -> $Unique<'ctx> {
                $Unique {
                    imp: self.imp.get_unique_constraint::<$Pk>(stringify!($pk)),
                    phantom: std::marker::PhantomData,
                }
            }
        }

        impl<'ctx> $Unique<'ctx> {
            pub fn find(&self, col_val: &$Pk) -> Option<$Row> {
                self.imp.find(col_val)
            }
        }

        pub(super) fn register_table(client_cache: &mut __sdk::ClientCache<super::RemoteModule>) {
            let table = client_cache.get_or_make_table::<$Row>($table);
            table.add_unique_constraint::<$Pk>(stringify!($pk), |row| &row.$pk);
        }

        pub(super) fn parse_table_update(
            raw_updates: __ws::v2::TableUpdate,
        ) -> __sdk::Result<__sdk::TableUpdate<$Row>> {
            __sdk::TableUpdate::parse_table_update(raw_updates).map_err(|e| {
                __sdk::InternalError::failed_parse(
                    concat!("TableUpdate<", stringify!($Row), ">"),
                    "TableUpdate",
                )
                .with_cause(e)
                .into()
            })
        }

        #[allow(non_camel_case_types)]
        pub trait $Query {
            #[allow(non_snake_case)]
            fn $accessor(&self) -> __sdk::__query_builder::Table<$Row>;
        }

        impl $Query for __sdk::QueryTableAccessor {
            fn $accessor(&self) -> __sdk::__query_builder::Table<$Row> {
                __sdk::__query_builder::Table::new($table)
            }
        }
    };
}
