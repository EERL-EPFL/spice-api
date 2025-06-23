pub use sea_orm_migration::prelude::*;

mod m20250326_152449_create_campaign_table;
mod m20250618_114538_modify_units_move_background_key;
mod m20250618_120731_convert_volume_fields_to_decimal;
mod m20250618_140000_add_projects_rename_campaigns_to_locations;
mod m20250618_150000_rename_liters_to_litres;
mod m20250623_091826_regions_use_numeric_coordinates;
mod m20250623_130000_wells_numeric_rows;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20250326_152449_create_campaign_table::Migration),
            Box::new(m20250618_114538_modify_units_move_background_key::Migration),
            Box::new(m20250618_120731_convert_volume_fields_to_decimal::Migration),
            Box::new(m20250618_140000_add_projects_rename_campaigns_to_locations::Migration),
            Box::new(m20250618_150000_rename_liters_to_litres::Migration),
            Box::new(m20250623_091826_regions_use_numeric_coordinates::Migration),
            Box::new(m20250623_130000_wells_numeric_rows::Migration),
        ]
    }
}
