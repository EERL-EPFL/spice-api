use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Rename columns from liters to litres in samples table
        manager
            .alter_table(
                Table::alter()
                    .table(Samples::Table)
                    .rename_column(
                        Alias::new("suspension_volume_liters"),
                        Samples::SuspensionVolumeLitres,
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Samples::Table)
                    .rename_column(Alias::new("air_volume_liters"), Samples::AirVolumeLitres)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Samples::Table)
                    .rename_column(
                        Alias::new("water_volume_liters"),
                        Samples::WaterVolumeLitres,
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Samples::Table)
                    .rename_column(Alias::new("well_volume_liters"), Samples::WellVolumeLitres)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Revert columns back to liters from litres in samples table
        manager
            .alter_table(
                Table::alter()
                    .table(Samples::Table)
                    .rename_column(
                        Samples::SuspensionVolumeLitres,
                        Alias::new("suspension_volume_liters"),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Samples::Table)
                    .rename_column(Samples::AirVolumeLitres, Alias::new("air_volume_liters"))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Samples::Table)
                    .rename_column(
                        Samples::WaterVolumeLitres,
                        Alias::new("water_volume_liters"),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Samples::Table)
                    .rename_column(Samples::WellVolumeLitres, Alias::new("well_volume_liters"))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Samples {
    Table,
    SuspensionVolumeLitres,
    AirVolumeLitres,
    WaterVolumeLitres,
    WellVolumeLitres,
}
