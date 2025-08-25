pub use sea_orm_migration::prelude::*;

mod m20250808_000001_consolidated_schema;
mod m20250120_000002_crudcrate_performance_indexes;
mod m20250820_000001_add_probe_configurations;
mod m20250122_000002_experiment_results_view;
mod m20250822_154010_add_procedural_blank_constraint;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20250808_000001_consolidated_schema::Migration),
            Box::new(m20250120_000002_crudcrate_performance_indexes::Migration),
            Box::new(m20250820_000001_add_probe_configurations::Migration),
            Box::new(m20250122_000002_experiment_results_view::Migration),
            Box::new(m20250822_154010_add_procedural_blank_constraint::Migration),
        ]
    }
}
