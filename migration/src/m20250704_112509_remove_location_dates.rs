use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Locations::Table)
                    .drop_column(Locations::StartDate)
                    .drop_column(Locations::EndDate)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Locations::Table)
                    .add_column(ColumnDef::new(Locations::StartDate).timestamp_with_time_zone())
                    .add_column(ColumnDef::new(Locations::EndDate).timestamp_with_time_zone())
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Locations {
    Table,
    StartDate,
    EndDate,
}