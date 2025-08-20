pub use sea_orm_migration::prelude::*;

mod m20250808_000001_consolidated_schema;
mod m20250120_000002_crudcrate_performance_indexes;
mod m20250820_000001_add_probe_configurations;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20250808_000001_consolidated_schema::Migration),
            Box::new(m20250120_000002_crudcrate_performance_indexes::Migration),
            Box::new(m20250820_000001_add_probe_configurations::Migration),
        ]
    }
}
