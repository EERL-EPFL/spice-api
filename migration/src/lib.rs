pub use sea_orm_migration::prelude::*;

mod m20250808_000001_consolidated_schema;
mod m20250826_000001_add_pg_trgm_extension;
mod m20251017_000001_rename_procedural_blank_to_blank;
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20250808_000001_consolidated_schema::Migration),
            Box::new(m20250826_000001_add_pg_trgm_extension::Migration),
            Box::new(m20251017_000001_rename_procedural_blank_to_blank::Migration),
        ]
    }
}
