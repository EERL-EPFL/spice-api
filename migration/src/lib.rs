pub use sea_orm_migration::prelude::*;

mod m20250326_152449_create_campaign_table;
mod m20250618_114538_modify_units_move_background_key;
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20250326_152449_create_campaign_table::Migration),
            Box::new(m20250618_114538_modify_units_move_background_key::Migration),
        ]
    }
}
