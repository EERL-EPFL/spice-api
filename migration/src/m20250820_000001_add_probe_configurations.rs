use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    #[allow(clippy::too_many_lines)] // Probe configuration migration requires extensive table modifications
    #[allow(clippy::match_wildcard_for_single_variants)] // Wildcard matches for unsupported databases are semantically correct
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add image coordinate columns to existing trays table (SQLite requires separate ALTER statements)
        manager
            .alter_table(
                Table::alter()
                    .table(Trays::Table)
                    .add_column(ColumnDef::new(Trays::UpperLeftCornerX).integer())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Trays::Table)
                    .add_column(ColumnDef::new(Trays::UpperLeftCornerY).integer())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Trays::Table)
                    .add_column(ColumnDef::new(Trays::LowerRightCornerX).integer())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Trays::Table)
                    .add_column(ColumnDef::new(Trays::LowerRightCornerY).integer())
                    .to_owned(),
            )
            .await?;

        // Create probes table directly linked to trays
        let mut probes_table = Table::create()
            .table(Probes::Table)
            .if_not_exists()
            .col(ColumnDef::new(Probes::TrayId).uuid().not_null())
            .col(ColumnDef::new(Probes::Name).text().not_null())
            .col(ColumnDef::new(Probes::DataColumnIndex).integer().not_null())
            .col(ColumnDef::new(Probes::PositionX).decimal().not_null())
            .col(ColumnDef::new(Probes::PositionY).decimal().not_null())
            .col(
                ColumnDef::new(Probes::CreatedAt)
                    .timestamp_with_time_zone()
                    .not_null()
                    .default(Expr::current_timestamp()),
            )
            .col(
                ColumnDef::new(Probes::LastUpdated)
                    .timestamp_with_time_zone()
                    .not_null()
                    .default(Expr::current_timestamp()),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_probes_tray_id")
                    .from(Probes::Table, Probes::TrayId)
                    .to(Trays::Table, Trays::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::NoAction),
            )
            // Ensure unique data column per tray
            .index(
                Index::create()
                    .name("probes_tray_data_column_unique")
                    .col(Probes::TrayId)
                    .col(Probes::DataColumnIndex)
                    .unique(),
            )
            .to_owned();

        // Add ID column with appropriate type and default based on database backend
        match manager.get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => {
                probes_table.col(
                    ColumnDef::new(Probes::Id)
                        .uuid()
                        .not_null()
                        .primary_key()
                        .default(Expr::cust("uuid_generate_v4()")),
                );
            }
            sea_orm::DatabaseBackend::Sqlite => {
                probes_table.col(ColumnDef::new(Probes::Id).uuid().not_null().primary_key());
            }
            _ => {
                return Err(DbErr::Custom("Unsupported database backend".to_string()));
            }
        }

        manager.create_table(probes_table).await?;

        // Create probe_temperature_readings table to link individual probes to temperature readings
        let mut probe_temperature_readings_table = Table::create()
            .table(ProbeTemperatureReadings::Table)
            .if_not_exists()
            .col(
                ColumnDef::new(ProbeTemperatureReadings::ProbeId)
                    .uuid()
                    .not_null(),
            )
            .col(
                ColumnDef::new(ProbeTemperatureReadings::TemperatureReadingId)
                    .uuid()
                    .not_null(),
            )
            .col(
                ColumnDef::new(ProbeTemperatureReadings::Temperature)
                    .decimal()
                    .not_null(),
            )
            .col(
                ColumnDef::new(ProbeTemperatureReadings::CreatedAt)
                    .timestamp_with_time_zone()
                    .not_null()
                    .default(Expr::current_timestamp()),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_probe_temp_readings_probe")
                    .from(ProbeTemperatureReadings::Table, ProbeTemperatureReadings::ProbeId)
                    .to(Probes::Table, Probes::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::NoAction),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_probe_temp_readings_temp_reading")
                    .from(
                        ProbeTemperatureReadings::Table,
                        ProbeTemperatureReadings::TemperatureReadingId,
                    )
                    .to(TemperatureReadings::Table, TemperatureReadings::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::NoAction),
            )
            // Composite primary key
            .index(
                Index::create()
                    .name("probe_temp_readings_pk")
                    .col(ProbeTemperatureReadings::ProbeId)
                    .col(ProbeTemperatureReadings::TemperatureReadingId)
                    .unique(),
            )
            .to_owned();

        // Add ID column for SeaORM compatibility
        match manager.get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => {
                probe_temperature_readings_table.col(
                    ColumnDef::new(ProbeTemperatureReadings::Id)
                        .uuid()
                        .not_null()
                        .primary_key()
                        .default(Expr::cust("uuid_generate_v4()")),
                );
            }
            sea_orm::DatabaseBackend::Sqlite => {
                probe_temperature_readings_table.col(
                    ColumnDef::new(ProbeTemperatureReadings::Id)
                        .uuid()
                        .not_null()
                        .primary_key(),
                );
            }
            _ => {
                return Err(DbErr::Custom("Unsupported database backend".to_string()));
            }
        }

        manager.create_table(probe_temperature_readings_table).await?;

        // Create performance indexes
        manager
            .create_index(
                Index::create()
                    .name("idx_probes_tray_id")
                    .table(Probes::Table)
                    .col(Probes::TrayId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_probe_temp_readings_probe_id")
                    .table(ProbeTemperatureReadings::Table)
                    .col(ProbeTemperatureReadings::ProbeId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop probe_temperature_readings table first (has foreign key to probes)
        manager
            .drop_table(
                Table::drop()
                    .table(ProbeTemperatureReadings::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;

        // Drop probes table
        manager
            .drop_table(Table::drop().table(Probes::Table).if_exists().to_owned())
            .await?;

        // Remove columns from trays table (SQLite requires separate ALTER statements)
        manager
            .alter_table(
                Table::alter()
                    .table(Trays::Table)
                    .drop_column(Trays::UpperLeftCornerX)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Trays::Table)
                    .drop_column(Trays::UpperLeftCornerY)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Trays::Table)
                    .drop_column(Trays::LowerRightCornerX)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Trays::Table)
                    .drop_column(Trays::LowerRightCornerY)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Trays {
    Table,
    Id,
    UpperLeftCornerX,
    UpperLeftCornerY,
    LowerRightCornerX,
    LowerRightCornerY,
}

#[derive(DeriveIden)]
enum Probes {
    Table,
    Id,
    TrayId,
    Name,
    DataColumnIndex,
    PositionX,
    PositionY,
    CreatedAt,
    LastUpdated,
}

#[derive(DeriveIden)]
enum ProbeTemperatureReadings {
    Table,
    Id,
    ProbeId,
    TemperatureReadingId,
    Temperature,
    CreatedAt,
}

#[derive(DeriveIden)]
enum TemperatureReadings {
    Table,
    Id,
}