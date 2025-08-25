use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add check constraint to prevent procedural blanks from having a location_id
        // Procedural blanks should be location-independent quality control samples
        match manager.get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => {
                manager
                    .get_connection()
                    .execute_unprepared(
                        "ALTER TABLE samples ADD CONSTRAINT chk_procedural_blank_no_location 
                         CHECK (type != 'procedural_blank' OR location_id IS NULL)"
                    )
                    .await?;
            }
            sea_orm::DatabaseBackend::Sqlite => {
                // SQLite doesn't support ADD CONSTRAINT for CHECK constraints on existing tables
                // We would need to recreate the table, but since this is a business rule,
                // we'll enforce it in the application layer instead
                println!("Warning: SQLite doesn't support adding CHECK constraints to existing tables.");
                println!("Procedural blank constraint will be enforced in application layer only.");
            }
            _ => {
                return Err(DbErr::Custom("Unsupported database backend".to_string()));
            }
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Remove the check constraint
        match manager.get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => {
                manager
                    .get_connection()
                    .execute_unprepared("ALTER TABLE samples DROP CONSTRAINT IF EXISTS chk_procedural_blank_no_location")
                    .await?;
            }
            sea_orm::DatabaseBackend::Sqlite => {
                // No constraint to remove in SQLite
                println!("No constraint to remove in SQLite");
            }
            _ => {
                return Err(DbErr::Custom("Unsupported database backend".to_string()));
            }
        }

        Ok(())
    }
}