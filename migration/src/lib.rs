pub use sea_orm_migration::prelude::*;

mod m20250624_000000_create_spice_schema;
mod m20250704_120000_condensed_post_v0_2_0;
mod m20250708_160000_harmonize_integer_types;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20250624_000000_create_spice_schema::Migration),
            Box::new(m20250704_120000_condensed_post_v0_2_0::Migration),
            Box::new(m20250708_160000_harmonize_integer_types::Migration),
        ]
    }
}
