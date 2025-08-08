use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Test basic table creation - just experiments table
        match manager.get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => {
                manager
                    .create_table(
                        Table::create()
                            .table(Experiments::Table)
                            .if_not_exists()
                            .col(
                                ColumnDef::new(Experiments::Id)
                                    .uuid()
                                    .not_null()
                                    .primary_key()
                                    .default(Expr::cust("uuid_generate_v4()")),
                            )
                            .col(ColumnDef::new(Experiments::Name).text().not_null())
                            .col(ColumnDef::new(Experiments::IsCalibration).boolean().not_null().default(false))
                            .col(ColumnDef::new(Experiments::CreatedAt).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
                            .col(ColumnDef::new(Experiments::LastUpdated).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
                            .to_owned(),
                    )
                    .await?;
            }
            sea_orm::DatabaseBackend::Sqlite => {
                manager
                    .create_table(
                        Table::create()
                            .table(Experiments::Table)
                            .if_not_exists()
                            .col(ColumnDef::new(Experiments::Id).uuid().not_null().primary_key())
                            .col(ColumnDef::new(Experiments::Name).text().not_null())
                            .col(ColumnDef::new(Experiments::IsCalibration).boolean().not_null().default(false))
                            .col(ColumnDef::new(Experiments::CreatedAt).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
                            .col(ColumnDef::new(Experiments::LastUpdated).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
                            .to_owned(),
                    )
                    .await?;
            }
            _ => {
                return Err(DbErr::Custom("Unsupported database backend".to_string()));
            }
        }

        // Create unique constraint
        manager
            .create_index(
                Index::create()
                    .name("experiments_name_key")
                    .table(Experiments::Table)
                    .col(Experiments::Name)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Experiments::Table).if_exists().to_owned()).await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Experiments {
    Table,
    Id,
    Name,
    IsCalibration,
    CreatedAt,
    LastUpdated,
}