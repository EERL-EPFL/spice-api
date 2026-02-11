use sea_orm_migration::prelude::extension::postgres::Type;
use sea_orm_migration::prelude::*;

#[derive(DeriveIden)]
enum SampleType {
    Table,
    FilterBlank,
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
                            .add_value(SampleType::FilterBlank)
                            .to_owned(),
                    )
                    .await?;
            }
            sea_orm::DatabaseBackend::Sqlite => {
                // SQLite stores enums as TEXT — any new string value is valid without migration
            }
            _ => {
                return Err(DbErr::Custom(
                    "Unsupported database backend for this migration".to_owned(),
                ))
            }
        }

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // PostgreSQL doesn't support removing enum values; no-op for all backends
        Ok(())
    }
}
