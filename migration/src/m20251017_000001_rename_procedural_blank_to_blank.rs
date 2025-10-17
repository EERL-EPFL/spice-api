use sea_orm_migration::prelude::extension::postgres::Type;
use sea_orm_migration::prelude::*;
use sea_query::{Expr, Query};

#[derive(DeriveIden)]
enum Samples {
    Table,
    Type,
}

#[derive(DeriveIden)]
enum SampleType {
    Table,
    Blank,
    ProceduralBlank,
}

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        match manager.get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => {
                manager
                    .alter_type(
                        Type::alter()
                            .name(SampleType::Table)
                            .rename_value(SampleType::ProceduralBlank, SampleType::Blank)
                            .to_owned(),
                    )
                    .await?;
            }
            sea_orm::DatabaseBackend::Sqlite => {
                manager
                    .exec_stmt(
                        Query::update()
                            .table(Samples::Table)
                            .value(Samples::Type, Expr::value("blank"))
                            .and_where(Expr::col(Samples::Type).eq("procedural_blank"))
                            .to_owned(),
                    )
                    .await?;
            }
            _ => {
                return Err(DbErr::Custom(
                    "Unsupported database backend for this migration".to_owned(),
                ))
            }
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        match manager.get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => {
                manager
                    .alter_type(
                        Type::alter()
                            .name(SampleType::Table)
                            .rename_value(SampleType::Blank, SampleType::ProceduralBlank)
                            .to_owned(),
                    )
                    .await?;
            }
            sea_orm::DatabaseBackend::Sqlite => {
                manager
                    .exec_stmt(
                        Query::update()
                            .table(Samples::Table)
                            .value(Samples::Type, Expr::value("procedural_blank"))
                            .and_where(Expr::col(Samples::Type).eq("blank"))
                            .to_owned(),
                    )
                    .await?;
            }
            _ => {
                return Err(DbErr::Custom(
                    "Unsupported database backend for this migration".to_owned(),
                ))
            }
        }

        Ok(())
    }
}
