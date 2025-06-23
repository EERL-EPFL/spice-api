use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Rename the existing coordinate columns to use the new naming convention
        // that matches the API request format: col_min, col_max, row_min, row_max

        manager
            .alter_table(
                Table::alter()
                    .table(Regions::Table)
                    .rename_column(Regions::UpperLeftCornerX, Regions::ColMin)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Regions::Table)
                    .rename_column(Regions::LowerRightCornerX, Regions::ColMax)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Regions::Table)
                    .rename_column(Regions::UpperLeftCornerY, Regions::RowMin)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Regions::Table)
                    .rename_column(Regions::LowerRightCornerY, Regions::RowMax)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Revert the column names back to the original format

        manager
            .alter_table(
                Table::alter()
                    .table(Regions::Table)
                    .rename_column(Regions::ColMin, Regions::UpperLeftCornerX)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Regions::Table)
                    .rename_column(Regions::ColMax, Regions::LowerRightCornerX)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Regions::Table)
                    .rename_column(Regions::RowMin, Regions::UpperLeftCornerY)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Regions::Table)
                    .rename_column(Regions::RowMax, Regions::LowerRightCornerY)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Regions {
    Table,
    UpperLeftCornerX,
    UpperLeftCornerY,
    LowerRightCornerX,
    LowerRightCornerY,
    ColMin,
    ColMax,
    RowMin,
    RowMax,
}
