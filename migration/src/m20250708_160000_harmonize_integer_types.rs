use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() == sea_orm::DatabaseBackend::Postgres {
            // PostgreSQL: Use ALTER COLUMN to modify existing columns
            self.alter_postgres_regions_table(manager).await?;
            self.alter_postgres_tray_config_assignments_table(manager)
                .await?;
        } else if manager.get_database_backend() == sea_orm::DatabaseBackend::Sqlite {
            // SQLite: Use table recreation approach
            self.recreate_sqlite_regions_table(manager).await?;
            self.recreate_sqlite_tray_config_assignments_table(manager)
                .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() == sea_orm::DatabaseBackend::Postgres {
            // PostgreSQL: Use ALTER COLUMN to revert columns back to small_integer
            self.revert_postgres_regions_table(manager).await?;
            self.revert_postgres_tray_config_assignments_table(manager)
                .await?;
        } else if manager.get_database_backend() == sea_orm::DatabaseBackend::Sqlite {
            // SQLite: Use table recreation approach to revert back to small_integer
            self.revert_sqlite_regions_table(manager).await?;
            self.revert_sqlite_tray_config_assignments_table(manager)
                .await?;
        }

        Ok(())
    }
}

impl Migration {
    async fn alter_postgres_regions_table(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        // Alter regions table to use 32-bit integers instead of 16-bit
        manager
            .alter_table(
                Table::alter()
                    .table(Regions::Table)
                    .modify_column(ColumnDef::new(Regions::TrayId).integer())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Regions::Table)
                    .modify_column(ColumnDef::new(Regions::ColMin).integer())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Regions::Table)
                    .modify_column(ColumnDef::new(Regions::RowMin).integer())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Regions::Table)
                    .modify_column(ColumnDef::new(Regions::ColMax).integer())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Regions::Table)
                    .modify_column(ColumnDef::new(Regions::RowMax).integer())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Regions::Table)
                    .modify_column(ColumnDef::new(Regions::DilutionFactor).integer())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn alter_postgres_tray_config_assignments_table(
        &self,
        manager: &SchemaManager<'_>,
    ) -> Result<(), DbErr> {
        // Alter tray_configuration_assignments table to use 32-bit integers
        manager
            .alter_table(
                Table::alter()
                    .table(TrayConfigurationAssignments::Table)
                    .modify_column(
                        ColumnDef::new(TrayConfigurationAssignments::OrderSequence).integer(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(TrayConfigurationAssignments::Table)
                    .modify_column(
                        ColumnDef::new(TrayConfigurationAssignments::RotationDegrees).integer(),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn recreate_sqlite_regions_table(
        &self,
        manager: &SchemaManager<'_>,
    ) -> Result<(), DbErr> {
        // SQLite doesn't support ALTER COLUMN, so we need to recreate the table

        // 1. Create new table with correct integer types
        manager
            .create_table(
                Table::create()
                    .table(Alias::new("regions_new"))
                    .col(ColumnDef::new(Regions::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Regions::ExperimentId).uuid().not_null())
                    .col(ColumnDef::new(Regions::TreatmentId).uuid())
                    .col(ColumnDef::new(Regions::Name).text())
                    .col(ColumnDef::new(Regions::DisplayColourHex).text())
                    .col(ColumnDef::new(Regions::TrayId).integer()) // Changed from small_integer
                    .col(ColumnDef::new(Regions::ColMin).integer()) // Changed from small_integer
                    .col(ColumnDef::new(Regions::RowMin).integer()) // Changed from small_integer
                    .col(ColumnDef::new(Regions::ColMax).integer()) // Changed from small_integer
                    .col(ColumnDef::new(Regions::RowMax).integer()) // Changed from small_integer
                    .col(ColumnDef::new(Regions::DilutionFactor).integer()) // Changed from small_integer
                    .col(
                        ColumnDef::new(Regions::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Regions::LastUpdated)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Regions::IsBackgroundKey)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;

        // 2. Copy data from old table to new table
        manager
            .get_connection()
            .execute_unprepared("INSERT INTO regions_new SELECT * FROM regions")
            .await?;

        // 3. Drop old table
        manager
            .drop_table(Table::drop().table(Regions::Table).to_owned())
            .await?;

        // 4. Rename new table to original name
        manager
            .get_connection()
            .execute_unprepared("ALTER TABLE regions_new RENAME TO regions")
            .await?;

        Ok(())
    }

    async fn recreate_sqlite_tray_config_assignments_table(
        &self,
        manager: &SchemaManager<'_>,
    ) -> Result<(), DbErr> {
        // SQLite doesn't support ALTER COLUMN, so we need to recreate the table

        // 1. Create new table with correct integer types
        manager
            .create_table(
                Table::create()
                    .table(Alias::new("tray_configuration_assignments_new"))
                    .col(
                        ColumnDef::new(TrayConfigurationAssignments::TrayConfigurationId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TrayConfigurationAssignments::TrayId)
                            .uuid()
                            .not_null(),
                    )
                    .col(ColumnDef::new(TrayConfigurationAssignments::OrderSequence).integer()) // Changed from small_integer
                    .col(ColumnDef::new(TrayConfigurationAssignments::RotationDegrees).integer()) // Changed from small_integer
                    .col(
                        ColumnDef::new(TrayConfigurationAssignments::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(TrayConfigurationAssignments::LastUpdated)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .primary_key(
                        Index::create()
                            .col(TrayConfigurationAssignments::TrayConfigurationId)
                            .col(TrayConfigurationAssignments::TrayId)
                            .col(TrayConfigurationAssignments::OrderSequence),
                    )
                    .to_owned(),
            )
            .await?;

        // 2. Copy data from old table to new table
        manager
            .get_connection()
            .execute_unprepared(
                "INSERT INTO tray_configuration_assignments_new SELECT * FROM tray_configuration_assignments",
            )
            .await?;

        // 3. Drop old table
        manager
            .drop_table(
                Table::drop()
                    .table(TrayConfigurationAssignments::Table)
                    .to_owned(),
            )
            .await?;

        // 4. Rename new table to original name
        manager
            .get_connection()
            .execute_unprepared(
                "ALTER TABLE tray_configuration_assignments_new RENAME TO tray_configuration_assignments",
            )
            .await?;

        Ok(())
    }

    async fn revert_postgres_regions_table(
        &self,
        manager: &SchemaManager<'_>,
    ) -> Result<(), DbErr> {
        // Revert regions table to use 16-bit integers
        manager
            .alter_table(
                Table::alter()
                    .table(Regions::Table)
                    .modify_column(ColumnDef::new(Regions::TrayId).small_integer())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Regions::Table)
                    .modify_column(ColumnDef::new(Regions::ColMin).small_integer())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Regions::Table)
                    .modify_column(ColumnDef::new(Regions::RowMin).small_integer())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Regions::Table)
                    .modify_column(ColumnDef::new(Regions::ColMax).small_integer())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Regions::Table)
                    .modify_column(ColumnDef::new(Regions::RowMax).small_integer())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Regions::Table)
                    .modify_column(ColumnDef::new(Regions::DilutionFactor).small_integer())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn revert_postgres_tray_config_assignments_table(
        &self,
        manager: &SchemaManager<'_>,
    ) -> Result<(), DbErr> {
        // Revert tray_configuration_assignments table to use 16-bit integers
        manager
            .alter_table(
                Table::alter()
                    .table(TrayConfigurationAssignments::Table)
                    .modify_column(
                        ColumnDef::new(TrayConfigurationAssignments::OrderSequence).small_integer(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(TrayConfigurationAssignments::Table)
                    .modify_column(
                        ColumnDef::new(TrayConfigurationAssignments::RotationDegrees)
                            .small_integer(),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn revert_sqlite_regions_table(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        // SQLite doesn't support ALTER COLUMN, so we need to recreate the table with small_integer

        // 1. Create new table with small_integer types (original schema)
        manager
            .create_table(
                Table::create()
                    .table(Alias::new("regions_new"))
                    .col(ColumnDef::new(Regions::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Regions::ExperimentId).uuid().not_null())
                    .col(ColumnDef::new(Regions::TreatmentId).uuid())
                    .col(ColumnDef::new(Regions::Name).text())
                    .col(ColumnDef::new(Regions::DisplayColourHex).text())
                    .col(ColumnDef::new(Regions::TrayId).small_integer()) // Reverted back to small_integer
                    .col(ColumnDef::new(Regions::ColMin).small_integer()) // Reverted back to small_integer
                    .col(ColumnDef::new(Regions::RowMin).small_integer()) // Reverted back to small_integer
                    .col(ColumnDef::new(Regions::ColMax).small_integer()) // Reverted back to small_integer
                    .col(ColumnDef::new(Regions::RowMax).small_integer()) // Reverted back to small_integer
                    .col(ColumnDef::new(Regions::DilutionFactor).small_integer()) // Reverted back to small_integer
                    .col(
                        ColumnDef::new(Regions::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Regions::LastUpdated)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Regions::IsBackgroundKey)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;

        // 2. Copy data from old table to new table
        manager
            .get_connection()
            .execute_unprepared("INSERT INTO regions_new SELECT * FROM regions")
            .await?;

        // 3. Drop old table
        manager
            .drop_table(Table::drop().table(Regions::Table).to_owned())
            .await?;

        // 4. Rename new table to original name
        manager
            .get_connection()
            .execute_unprepared("ALTER TABLE regions_new RENAME TO regions")
            .await?;

        Ok(())
    }

    async fn revert_sqlite_tray_config_assignments_table(
        &self,
        manager: &SchemaManager<'_>,
    ) -> Result<(), DbErr> {
        // SQLite doesn't support ALTER COLUMN, so we need to recreate the table with small_integer

        // 1. Create new table with small_integer types (original schema)
        manager
            .create_table(
                Table::create()
                    .table(Alias::new("tray_configuration_assignments_new"))
                    .col(
                        ColumnDef::new(TrayConfigurationAssignments::TrayConfigurationId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TrayConfigurationAssignments::TrayId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TrayConfigurationAssignments::OrderSequence).small_integer(),
                    ) // Reverted back to small_integer
                    .col(
                        ColumnDef::new(TrayConfigurationAssignments::RotationDegrees)
                            .small_integer(),
                    ) // Reverted back to small_integer
                    .col(
                        ColumnDef::new(TrayConfigurationAssignments::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(TrayConfigurationAssignments::LastUpdated)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .primary_key(
                        Index::create()
                            .col(TrayConfigurationAssignments::TrayConfigurationId)
                            .col(TrayConfigurationAssignments::TrayId)
                            .col(TrayConfigurationAssignments::OrderSequence),
                    )
                    .to_owned(),
            )
            .await?;

        // 2. Copy data from old table to new table
        manager
            .get_connection()
            .execute_unprepared(
                "INSERT INTO tray_configuration_assignments_new SELECT * FROM tray_configuration_assignments",
            )
            .await?;

        // 3. Drop old table
        manager
            .drop_table(
                Table::drop()
                    .table(TrayConfigurationAssignments::Table)
                    .to_owned(),
            )
            .await?;

        // 4. Rename new table to original name
        manager
            .get_connection()
            .execute_unprepared(
                "ALTER TABLE tray_configuration_assignments_new RENAME TO tray_configuration_assignments",
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Regions {
    Table,
    Id,
    ExperimentId,
    TreatmentId,
    Name,
    DisplayColourHex,
    TrayId,
    ColMin,
    RowMin,
    ColMax,
    RowMax,
    DilutionFactor,
    CreatedAt,
    LastUpdated,
    IsBackgroundKey,
}

#[derive(DeriveIden)]
enum TrayConfigurationAssignments {
    Table,
    TrayConfigurationId,
    TrayId,
    OrderSequence,
    RotationDegrees,
    CreatedAt,
    LastUpdated,
}
