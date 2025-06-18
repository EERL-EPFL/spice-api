use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 1. Modify treatments table: rename enzyme_volume_microlitres to enzyme_volume_litres
        manager
            .alter_table(
                Table::alter()
                    .table(Treatments::Table)
                    .rename_column(
                        Treatments::EnzymeVolumeMicrolitres,
                        Treatments::EnzymeVolumeLitres,
                    )
                    .to_owned(),
            )
            .await?;

        // 2. Remove background_region_key from samples table
        manager
            .alter_table(
                Table::alter()
                    .table(Samples::Table)
                    .drop_column(Samples::BackgroundRegionKey)
                    .to_owned(),
            )
            .await?;

        // 3. Add is_background_key column to regions table
        manager
            .alter_table(
                Table::alter()
                    .table(Regions::Table)
                    .add_column(
                        ColumnDef::new(Regions::IsBackgroundKey)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 1. Revert treatments table: rename enzyme_volume_litres back to enzyme_volume_microlitres
        manager
            .alter_table(
                Table::alter()
                    .table(Treatments::Table)
                    .rename_column(
                        Treatments::EnzymeVolumeLitres,
                        Treatments::EnzymeVolumeMicrolitres,
                    )
                    .to_owned(),
            )
            .await?;

        // 2. Add back background_region_key to samples table
        manager
            .alter_table(
                Table::alter()
                    .table(Samples::Table)
                    .add_column(ColumnDef::new(Samples::BackgroundRegionKey).text().null())
                    .to_owned(),
            )
            .await?;

        // 3. Remove is_background_key column from regions table
        manager
            .alter_table(
                Table::alter()
                    .table(Regions::Table)
                    .drop_column(Regions::IsBackgroundKey)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Treatments {
    Table,
    EnzymeVolumeMicrolitres,
    EnzymeVolumeLitres,
}

#[derive(DeriveIden)]
enum Samples {
    Table,
    BackgroundRegionKey,
}

#[derive(DeriveIden)]
enum Regions {
    Table,
    IsBackgroundKey,
}
