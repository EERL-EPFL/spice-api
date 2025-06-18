use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Convert treatments.enzyme_volume_litres from DOUBLE PRECISION to NUMERIC
        manager
            .alter_table(
                Table::alter()
                    .table(Treatments::Table)
                    .modify_column(
                        ColumnDef::new(Treatments::EnzymeVolumeLitres)
                            .decimal_len(20, 10)
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Convert samples.flow_litres_per_minute from DOUBLE PRECISION to NUMERIC
        manager
            .alter_table(
                Table::alter()
                    .table(Samples::Table)
                    .modify_column(
                        ColumnDef::new(Samples::FlowLitresPerMinute)
                            .decimal_len(20, 10)
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Convert samples.total_volume from DOUBLE PRECISION to NUMERIC
        manager
            .alter_table(
                Table::alter()
                    .table(Samples::Table)
                    .modify_column(
                        ColumnDef::new(Samples::TotalVolume)
                            .decimal_len(20, 10)
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Revert treatments.enzyme_volume_litres back to DOUBLE PRECISION
        manager
            .alter_table(
                Table::alter()
                    .table(Treatments::Table)
                    .modify_column(
                        ColumnDef::new(Treatments::EnzymeVolumeLitres)
                            .double()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Revert samples.flow_litres_per_minute back to DOUBLE PRECISION
        manager
            .alter_table(
                Table::alter()
                    .table(Samples::Table)
                    .modify_column(ColumnDef::new(Samples::FlowLitresPerMinute).double().null())
                    .to_owned(),
            )
            .await?;

        // Revert samples.total_volume back to DOUBLE PRECISION
        manager
            .alter_table(
                Table::alter()
                    .table(Samples::Table)
                    .modify_column(ColumnDef::new(Samples::TotalVolume).double().null())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Treatments {
    Table,
    EnzymeVolumeLitres,
}

#[derive(DeriveIden)]
enum Samples {
    Table,
    FlowLitresPerMinute,
    TotalVolume,
}
