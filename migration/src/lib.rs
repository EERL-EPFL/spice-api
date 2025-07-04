pub use sea_orm_migration::prelude::*;

mod m20250624_000000_create_spice_schema;
mod m20250704_112509_remove_location_dates;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20250624_000000_create_spice_schema::Migration),
            Box::new(m20250704_112509_remove_location_dates::Migration),
        ]
    }
}
