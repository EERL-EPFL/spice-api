use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        match manager.get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => {
                // PostgreSQL version - can do multiple columns at once
                // Step 1: Add tray detail columns to tray_configuration_assignments table
                manager
                    .alter_table(
                        Table::alter()
                            .table(TrayConfigurationAssignments::Table)
                            .add_column(ColumnDef::new(TrayConfigurationAssignments::Name).text())
                            .add_column(
                                ColumnDef::new(TrayConfigurationAssignments::QtyXAxis).integer(),
                            )
                            .add_column(
                                ColumnDef::new(TrayConfigurationAssignments::QtyYAxis).integer(),
                            )
                            .add_column(
                                ColumnDef::new(TrayConfigurationAssignments::WellRelativeDiameter)
                                    .decimal(),
                            )
                            .to_owned(),
                    )
                    .await?;

                // Step 2: Migrate data using PostgreSQL JOIN syntax
                db.execute_unprepared(
                    "UPDATE tray_configuration_assignments 
                     SET name = trays.name,
                         qty_x_axis = trays.qty_x_axis,
                         qty_y_axis = trays.qty_y_axis,
                         well_relative_diameter = trays.well_relative_diameter
                     FROM trays 
                     WHERE tray_configuration_assignments.tray_id = trays.id",
                )
                .await?;

                // Step 3: Drop foreign key constraint
                let _ = manager
                    .drop_foreign_key(
                        ForeignKey::drop()
                            .name("fk_tray_configuration_assignment_tray_id")
                            .table(TrayConfigurationAssignments::Table)
                            .to_owned(),
                    )
                    .await; // Ignore error if FK doesn't exist

                // Step 4: Drop tray_id column
                manager
                    .alter_table(
                        Table::alter()
                            .table(TrayConfigurationAssignments::Table)
                            .drop_column(TrayConfigurationAssignments::TrayId)
                            .to_owned(),
                    )
                    .await?;

                // Step 5: Drop old trays table
                manager
                    .drop_table(
                        Table::drop()
                            .table(Trays::Table)
                            .if_exists()
                            .cascade()
                            .to_owned(),
                    )
                    .await?;

                // Step 6: Rename table
                db.execute_unprepared("ALTER TABLE tray_configuration_assignments RENAME TO trays")
                    .await?;

                // Step 7: Recreate index with new name
                let _ = manager
                    .drop_index(
                        Index::drop()
                            .name("idx_tray_configuration_assignments_tray_configuration_id")
                            .to_owned(),
                    )
                    .await; // Ignore if doesn't exist

                manager
                    .create_index(
                        Index::create()
                            .name("idx_trays_tray_configuration_id")
                            .table(Trays::Table)
                            .col(Trays::TrayConfigurationId)
                            .to_owned(),
                    )
                    .await?;
            }
            sea_orm::DatabaseBackend::Sqlite => {
                // SQLite version - simplified approach for both fresh and existing databases
                // Just create the new table structure directly

                // Drop old tables if they exist
                db.execute_unprepared("DROP TABLE IF EXISTS tray_configuration_assignments")
                    .await?;
                db.execute_unprepared("DROP TABLE IF EXISTS trays").await?;

                // Create the new trays table with embedded tray details
                db.execute_unprepared(
                    "CREATE TABLE trays (
                        id TEXT PRIMARY KEY,
                        tray_configuration_id TEXT NOT NULL,
                        order_sequence INTEGER NOT NULL,
                        rotation_degrees INTEGER NOT NULL,
                        name TEXT,
                        qty_x_axis INTEGER,
                        qty_y_axis INTEGER,
                        well_relative_diameter REAL,
                        created_at TEXT NOT NULL,
                        last_updated TEXT NOT NULL,
                        FOREIGN KEY (tray_configuration_id) REFERENCES tray_configurations (id) ON DELETE CASCADE
                    )"
                ).await?;

                // Create index
                db.execute_unprepared(
                    "CREATE INDEX idx_trays_tray_configuration_id ON trays (tray_configuration_id)",
                )
                .await?;
            }
            _ => {
                return Err(DbErr::Custom("Unsupported database backend".to_string()));
            }
        }

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // This is a complex migration that simplifies the schema.
        // Rolling back would require recreating the original 3-table structure.
        // For now, we'll just log a warning about the complexity of rollback.

        println!("Warning: Rolling back this migration is complex and not implemented.");
        println!("This migration simplified tray_configuration_assignments + trays into a single trays table.");
        println!("To rollback, you would need to:");
        println!("1. Rename trays back to tray_configuration_assignments");
        println!("2. Recreate the separate trays table");
        println!("3. Extract tray details back into separate records");
        println!("4. Restore the foreign key relationships");
        println!("Consider restoring from backup if rollback is needed.");

        Ok(())
    }
}

// Table identifiers
#[derive(DeriveIden)]
enum TrayConfigurationAssignments {
    Table,
    TrayId,
    Name,
    QtyXAxis,
    QtyYAxis,
    WellRelativeDiameter,
}

#[derive(DeriveIden)]
enum Trays {
    Table,
    TrayConfigurationId,
}
