use sea_orm_migration::prelude::extension::postgres::Type;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    #[allow(clippy::too_many_lines)] // Large migration requires extensive table definitions
    #[allow(clippy::match_wildcard_for_single_variants)] // Wildcard matches for unsupported databases are semantically correct
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Enable UUID, PostGIS, and trigram extensions for PostgreSQL
        if manager.get_database_backend() == sea_orm::DatabaseBackend::Postgres {
            manager
                .get_connection()
                .execute_unprepared("CREATE EXTENSION IF NOT EXISTS \"uuid-ossp\";")
                .await?;
            manager
                .get_connection()
                .execute_unprepared("CREATE EXTENSION IF NOT EXISTS \"postgis\";")
                .await?;
            // Enable trigram extension for similarity-based search
            manager
                .get_connection()
                .execute_unprepared("CREATE EXTENSION IF NOT EXISTS pg_trgm")
                .await?;
        }

        // Create custom types for PostgreSQL (will be ignored by SQLite)
        if manager.get_database_backend() == sea_orm::DatabaseBackend::Postgres {
            // Create sample_type enum
            manager
                .create_type(
                    Type::create()
                        .as_enum(SampleType::Table)
                        .values([
                            SampleType::Bulk,
                            SampleType::Filter,
                            SampleType::ProceduralBlank,
                        ])
                        .to_owned(),
                )
                .await?;

            // Create treatment_name enum
            manager
                .create_type(
                    Type::create()
                        .as_enum(TreatmentName::Table)
                        .values([
                            TreatmentName::None,
                            TreatmentName::Heat,
                            TreatmentName::H2o2,
                        ])
                        .to_owned(),
                )
                .await?;
        }

        // Create projects table
        let mut projects_table = Table::create()
            .table(Projects::Table)
            .if_not_exists()
            .col(
                ColumnDef::new(Projects::Name)
                    .string()
                    .not_null()
                    .unique_key(),
            )
            .col(ColumnDef::new(Projects::Note).text())
            .col(ColumnDef::new(Projects::Colour).string())
            .col(
                ColumnDef::new(Projects::CreatedAt)
                    .timestamp_with_time_zone()
                    .not_null()
                    .default(Expr::current_timestamp()),
            )
            .col(
                ColumnDef::new(Projects::LastUpdated)
                    .timestamp_with_time_zone()
                    .not_null()
                    .default(Expr::current_timestamp()),
            )
            .to_owned();

        // Add ID column with appropriate type and default based on database backend
        match manager.get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => {
                projects_table.col(
                    ColumnDef::new(Projects::Id)
                        .uuid()
                        .not_null()
                        .primary_key()
                        .default(Expr::cust("uuid_generate_v4()")),
                );
            }
            sea_orm::DatabaseBackend::Sqlite => {
                projects_table.col(ColumnDef::new(Projects::Id).uuid().not_null().primary_key());
            }
            _ => {
                return Err(DbErr::Custom("Unsupported database backend".to_string()));
            }
        }

        manager.create_table(projects_table).await?;

        // Create locations table
        let mut locations_table = Table::create()
            .table(Locations::Table)
            .if_not_exists()
            .col(
                ColumnDef::new(Locations::Name)
                    .string()
                    .not_null()
                    .unique_key(),
            )
            .col(ColumnDef::new(Locations::Comment).text())
            .col(ColumnDef::new(Locations::ProjectId).uuid())
            .col(
                ColumnDef::new(Locations::LastUpdated)
                    .timestamp_with_time_zone()
                    .not_null()
                    .default(Expr::current_timestamp()),
            )
            .col(
                ColumnDef::new(Locations::CreatedAt)
                    .timestamp_with_time_zone()
                    .not_null()
                    .default(Expr::current_timestamp()),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_locations_project_id")
                    .from(Locations::Table, Locations::ProjectId)
                    .to(Projects::Table, Projects::Id)
                    .on_delete(ForeignKeyAction::SetNull)
                    .on_update(ForeignKeyAction::NoAction),
            )
            .to_owned();

        // Add ID column with appropriate type and default based on database backend
        match manager.get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => {
                locations_table.col(
                    ColumnDef::new(Locations::Id)
                        .uuid()
                        .not_null()
                        .primary_key()
                        .default(Expr::cust("uuid_generate_v4()")),
                );
            }
            sea_orm::DatabaseBackend::Sqlite => {
                locations_table.col(
                    ColumnDef::new(Locations::Id)
                        .uuid()
                        .not_null()
                        .primary_key(),
                );
            }
            _ => {
                return Err(DbErr::Custom("Unsupported database backend".to_string()));
            }
        }

        manager.create_table(locations_table).await?;

        // Create samples table
        let mut samples_table = Table::create()
            .table(Samples::Table)
            .if_not_exists()
            .col(ColumnDef::new(Samples::Name).text().not_null())
            .col(ColumnDef::new(Samples::StartTime).timestamp_with_time_zone())
            .col(ColumnDef::new(Samples::StopTime).timestamp_with_time_zone())
            .col(ColumnDef::new(Samples::FlowLitresPerMinute).decimal_len(16, 10))
            .col(ColumnDef::new(Samples::TotalVolume).decimal_len(16, 10))
            .col(ColumnDef::new(Samples::MaterialDescription).text())
            .col(ColumnDef::new(Samples::ExtractionProcedure).text())
            .col(ColumnDef::new(Samples::FilterSubstrate).text())
            .col(ColumnDef::new(Samples::SuspensionVolumeLitres).decimal())
            .col(ColumnDef::new(Samples::AirVolumeLitres).decimal())
            .col(ColumnDef::new(Samples::WaterVolumeLitres).decimal())
            .col(ColumnDef::new(Samples::InitialConcentrationGramL).decimal())
            .col(ColumnDef::new(Samples::WellVolumeLitres).decimal())
            .col(ColumnDef::new(Samples::Remarks).text())
            .col(ColumnDef::new(Samples::Longitude).decimal_len(9, 6))
            .col(ColumnDef::new(Samples::Latitude).decimal_len(9, 6))
            .col(ColumnDef::new(Samples::LocationId).uuid())
            .col(
                ColumnDef::new(Samples::CreatedAt)
                    .timestamp_with_time_zone()
                    .not_null()
                    .default(Expr::current_timestamp()),
            )
            .col(
                ColumnDef::new(Samples::LastUpdated)
                    .timestamp_with_time_zone()
                    .not_null()
                    .default(Expr::current_timestamp()),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("samples_location_id_fkey")
                    .from(Samples::Table, Samples::LocationId)
                    .to(Locations::Table, Locations::Id)
                    .on_delete(ForeignKeyAction::NoAction)
                    .on_update(ForeignKeyAction::NoAction),
            )
            .to_owned();

        // Add ID column with appropriate type and default based on database backend
        match manager.get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => {
                samples_table.col(
                    ColumnDef::new(Samples::Id)
                        .uuid()
                        .not_null()
                        .primary_key()
                        .default(Expr::cust("uuid_generate_v4()")),
                );
            }
            sea_orm::DatabaseBackend::Sqlite => {
                samples_table.col(ColumnDef::new(Samples::Id).uuid().not_null().primary_key());
            }
            _ => {
                return Err(DbErr::Custom("Unsupported database backend".to_string()));
            }
        }

        // Add type column with appropriate constraint based on database backend
        match manager.get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => {
                samples_table.col(
                    ColumnDef::new(Samples::Type)
                        .custom(SampleType::Table)
                        .not_null(),
                );
            }
            sea_orm::DatabaseBackend::Sqlite => {
                samples_table.col(ColumnDef::new(Samples::Type).text().not_null());
            }
            _ => {
                return Err(DbErr::Custom("Unsupported database backend".to_string()));
            }
        }

        manager.create_table(samples_table).await?;

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
            }
            _ => {
                return Err(DbErr::Custom("Unsupported database backend".to_string()));
            }
        }

        // Add PostGIS geometry column for PostgreSQL only
        if manager.get_database_backend() == sea_orm::DatabaseBackend::Postgres {
            manager
                .get_connection()
                .execute_unprepared(
                    "ALTER TABLE samples ADD COLUMN geom geometry(Point, 4326) GENERATED ALWAYS AS (
                        CASE
                            WHEN latitude IS NOT NULL AND longitude IS NOT NULL
                            THEN ST_SetSRID(ST_MakePoint(longitude, latitude), 4326)
                            ELSE NULL
                        END
                    ) STORED;"
                )
                .await?;
        }

        // Create treatments table
        let mut treatments_table = Table::create()
            .table(Treatments::Table)
            .if_not_exists()
            .col(ColumnDef::new(Treatments::Notes).text())
            .col(ColumnDef::new(Treatments::SampleId).uuid())
            .col(
                ColumnDef::new(Treatments::LastUpdated)
                    .timestamp_with_time_zone()
                    .not_null()
                    .default(Expr::current_timestamp()),
            )
            .col(
                ColumnDef::new(Treatments::CreatedAt)
                    .timestamp_with_time_zone()
                    .not_null()
                    .default(Expr::current_timestamp()),
            )
            .col(ColumnDef::new(Treatments::EnzymeVolumeLitres).decimal_len(16, 10))
            .foreign_key(
                ForeignKey::create()
                    .name("sample_treatments")
                    .from(Treatments::Table, Treatments::SampleId)
                    .to(Samples::Table, Samples::Id)
                    .on_delete(ForeignKeyAction::NoAction)
                    .on_update(ForeignKeyAction::NoAction),
            )
            .to_owned();

        // Add ID column with appropriate type and default based on database backend
        match manager.get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => {
                treatments_table.col(
                    ColumnDef::new(Treatments::Id)
                        .uuid()
                        .not_null()
                        .primary_key()
                        .default(Expr::cust("uuid_generate_v4()")),
                );
            }
            sea_orm::DatabaseBackend::Sqlite => {
                treatments_table.col(
                    ColumnDef::new(Treatments::Id)
                        .uuid()
                        .not_null()
                        .primary_key(),
                );
            }
            _ => {
                return Err(DbErr::Custom("Unsupported database backend".to_string()));
            }
        }

        // Add name column with appropriate constraint based on database backend
        match manager.get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => {
                treatments_table.col(
                    ColumnDef::new(Treatments::Name)
                        .custom(TreatmentName::Table)
                        .not_null(),
                );
            }
            sea_orm::DatabaseBackend::Sqlite => {
                treatments_table.col(ColumnDef::new(Treatments::Name).text().not_null());
            }
            _ => {
                return Err(DbErr::Custom("Unsupported database backend".to_string()));
            }
        }

        manager.create_table(treatments_table).await?;

        // Create tray_configurations table
        let mut tray_configurations_table = Table::create()
            .table(TrayConfigurations::Table)
            .if_not_exists()
            .col(ColumnDef::new(TrayConfigurations::Name).text().unique_key())
            .col(
                ColumnDef::new(TrayConfigurations::ExperimentDefault)
                    .boolean()
                    .not_null(),
            )
            .col(
                ColumnDef::new(TrayConfigurations::CreatedAt)
                    .timestamp_with_time_zone()
                    .not_null()
                    .default(Expr::current_timestamp()),
            )
            .col(
                ColumnDef::new(TrayConfigurations::LastUpdated)
                    .timestamp_with_time_zone()
                    .not_null()
                    .default(Expr::current_timestamp()),
            )
            .to_owned();

        // Add ID column with appropriate type and default based on database backend
        match manager.get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => {
                tray_configurations_table.col(
                    ColumnDef::new(TrayConfigurations::Id)
                        .uuid()
                        .not_null()
                        .primary_key()
                        .default(Expr::cust("uuid_generate_v4()")),
                );
            }
            sea_orm::DatabaseBackend::Sqlite => {
                tray_configurations_table.col(
                    ColumnDef::new(TrayConfigurations::Id)
                        .uuid()
                        .not_null()
                        .primary_key(),
                );
            }
            _ => {
                return Err(DbErr::Custom("Unsupported database backend".to_string()));
            }
        }

        manager.create_table(tray_configurations_table).await?;

        // Create experiments table
        let mut experiments_table = Table::create()
            .table(Experiments::Table)
            .if_not_exists()
            .col(
                ColumnDef::new(Experiments::Name)
                    .text()
                    .not_null()
                    .unique_key(),
            )
            .col(ColumnDef::new(Experiments::Username).text().null())
            .col(
                ColumnDef::new(Experiments::PerformedAt)
                    .timestamp_with_time_zone()
                    .null(),
            )
            .col(
                ColumnDef::new(Experiments::TemperatureRamp)
                    .decimal()
                    .null(),
            )
            .col(
                ColumnDef::new(Experiments::TemperatureStart)
                    .decimal()
                    .null(),
            )
            .col(ColumnDef::new(Experiments::TemperatureEnd).decimal().null())
            .col(
                ColumnDef::new(Experiments::IsCalibration)
                    .boolean()
                    .not_null()
                    .default(false),
            )
            .col(ColumnDef::new(Experiments::Remarks).text().null())
            .col(
                ColumnDef::new(Experiments::TrayConfigurationId)
                    .uuid()
                    .null(),
            )
            .col(
                ColumnDef::new(Experiments::CreatedAt)
                    .timestamp_with_time_zone()
                    .not_null()
                    .default(Expr::current_timestamp()),
            )
            .col(
                ColumnDef::new(Experiments::LastUpdated)
                    .timestamp_with_time_zone()
                    .not_null()
                    .default(Expr::current_timestamp()),
            )
            .col(
                ColumnDef::new(Experiments::HasResults)
                    .boolean()
                    .not_null()
                    .default(false),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_experiment_tray_configuration")
                    .from(Experiments::Table, Experiments::TrayConfigurationId)
                    .to(TrayConfigurations::Table, TrayConfigurations::Id)
                    .on_delete(ForeignKeyAction::NoAction)
                    .on_update(ForeignKeyAction::NoAction),
            )
            .to_owned();

        // Add ID column with appropriate type and default based on database backend
        match manager.get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => {
                experiments_table.col(
                    ColumnDef::new(Experiments::Id)
                        .uuid()
                        .not_null()
                        .primary_key()
                        .default(Expr::cust("uuid_generate_v4()")),
                );
            }
            sea_orm::DatabaseBackend::Sqlite => {
                experiments_table.col(
                    ColumnDef::new(Experiments::Id)
                        .uuid()
                        .not_null()
                        .primary_key(),
                );
            }
            _ => {
                return Err(DbErr::Custom("Unsupported database backend".to_string()));
            }
        }

        manager.create_table(experiments_table).await?;

        // Create trays table
        let mut trays_table = Table::create()
            .table(Trays::Table)
            .if_not_exists()
            .col(ColumnDef::new(Trays::TrayConfigurationId).uuid().not_null())
            .col(ColumnDef::new(Trays::OrderSequence).integer().not_null())
            .col(ColumnDef::new(Trays::RotationDegrees).integer().not_null())
            .col(
                ColumnDef::new(Trays::CreatedAt)
                    .timestamp_with_time_zone()
                    .not_null()
                    .default(Expr::current_timestamp()),
            )
            .col(
                ColumnDef::new(Trays::LastUpdated)
                    .timestamp_with_time_zone()
                    .not_null()
                    .default(Expr::current_timestamp()),
            )
            .col(ColumnDef::new(Trays::Name).text())
            .col(ColumnDef::new(Trays::QtyCols).integer())
            .col(ColumnDef::new(Trays::QtyRows).integer())
            .col(ColumnDef::new(Trays::WellRelativeDiameter).decimal())
            // Image coordinate columns for probe positioning
            .col(ColumnDef::new(Trays::UpperLeftCornerX).integer())
            .col(ColumnDef::new(Trays::UpperLeftCornerY).integer())
            .col(ColumnDef::new(Trays::LowerRightCornerX).integer())
            .col(ColumnDef::new(Trays::LowerRightCornerY).integer())
            .foreign_key(
                ForeignKey::create()
                    .name("fk_tray_assignment_to_configuration")
                    .from(Trays::Table, Trays::TrayConfigurationId)
                    .to(TrayConfigurations::Table, TrayConfigurations::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::NoAction),
            )
            // Add composite unique constraint for SQLite compatibility
            .index(
                Index::create()
                    .name("trays_config_sequence_unique")
                    .col(Trays::TrayConfigurationId)
                    .col(Trays::OrderSequence)
                    .unique(),
            )
            .to_owned();

        // Note: trays table does not have UUID primary key, it uses composite key or no explicit primary key in the SQL
        // However, SeaORM models typically need a primary key, so we might need to add one
        // For now, let's add an ID column to match SeaORM expectations
        match manager.get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => {
                trays_table.col(
                    ColumnDef::new(Trays::Id)
                        .uuid()
                        .not_null()
                        .primary_key()
                        .default(Expr::cust("uuid_generate_v4()")),
                );
            }
            sea_orm::DatabaseBackend::Sqlite => {
                trays_table.col(ColumnDef::new(Trays::Id).uuid().not_null().primary_key());
            }
            _ => {
                return Err(DbErr::Custom("Unsupported database backend".to_string()));
            }
        }

        manager.create_table(trays_table).await?;

        // Create wells table
        let mut wells_table = Table::create()
            .table(Wells::Table)
            .if_not_exists()
            .col(ColumnDef::new(Wells::TrayId).uuid().not_null())
            .col(ColumnDef::new(Wells::RowLetter).string_len(2).not_null())
            .col(ColumnDef::new(Wells::ColumnNumber).integer().not_null())
            .col(
                ColumnDef::new(Wells::CreatedAt)
                    .timestamp_with_time_zone()
                    .not_null()
                    .default(Expr::current_timestamp()),
            )
            .col(
                ColumnDef::new(Wells::LastUpdated)
                    .timestamp_with_time_zone()
                    .not_null()
                    .default(Expr::current_timestamp()),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_wells_tray_id")
                    .from(Wells::Table, Wells::TrayId)
                    .to(Trays::Table, Trays::Id)
                    .on_delete(ForeignKeyAction::NoAction)
                    .on_update(ForeignKeyAction::NoAction),
            )
            .to_owned();

        // Add ID column with appropriate type and default based on database backend
        match manager.get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => {
                wells_table.col(
                    ColumnDef::new(Wells::Id)
                        .uuid()
                        .not_null()
                        .primary_key()
                        .default(Expr::cust("uuid_generate_v4()")),
                );
            }
            sea_orm::DatabaseBackend::Sqlite => {
                wells_table.col(ColumnDef::new(Wells::Id).uuid().not_null().primary_key());
            }
            _ => {
                return Err(DbErr::Custom("Unsupported database backend".to_string()));
            }
        }

        manager.create_table(wells_table).await?;

        // Create regions table
        let mut regions_table = Table::create()
            .table(Regions::Table)
            .if_not_exists()
            .col(ColumnDef::new(Regions::ExperimentId).uuid().not_null())
            .col(ColumnDef::new(Regions::TreatmentId).uuid())
            .col(ColumnDef::new(Regions::Name).text())
            .col(ColumnDef::new(Regions::DisplayColourHex).text())
            .col(ColumnDef::new(Regions::TrayId).integer())
            .col(ColumnDef::new(Regions::ColMin).integer())
            .col(ColumnDef::new(Regions::RowMin).integer())
            .col(ColumnDef::new(Regions::ColMax).integer())
            .col(ColumnDef::new(Regions::RowMax).integer())
            .col(ColumnDef::new(Regions::DilutionFactor).integer())
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
            .foreign_key(
                ForeignKey::create()
                    .name("regions_experiment_id_fkey")
                    .from(Regions::Table, Regions::ExperimentId)
                    .to(Experiments::Table, Experiments::Id)
                    .on_delete(ForeignKeyAction::NoAction)
                    .on_update(ForeignKeyAction::NoAction),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("regions_treatment_id_fkey")
                    .from(Regions::Table, Regions::TreatmentId)
                    .to(Treatments::Table, Treatments::Id)
                    .on_delete(ForeignKeyAction::NoAction)
                    .on_update(ForeignKeyAction::NoAction),
            )
            .to_owned();

        // Add ID column with appropriate type and default based on database backend
        match manager.get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => {
                regions_table.col(
                    ColumnDef::new(Regions::Id)
                        .uuid()
                        .not_null()
                        .primary_key()
                        .default(Expr::cust("uuid_generate_v4()")),
                );
            }
            sea_orm::DatabaseBackend::Sqlite => {
                regions_table.col(ColumnDef::new(Regions::Id).uuid().not_null().primary_key());
            }
            _ => {
                return Err(DbErr::Custom("Unsupported database backend".to_string()));
            }
        }

        manager.create_table(regions_table).await?;

        // Create s3_assets table
        let mut s3_assets_table = Table::create()
            .table(S3Assets::Table)
            .if_not_exists()
            .col(ColumnDef::new(S3Assets::ExperimentId).uuid())
            .col(ColumnDef::new(S3Assets::OriginalFilename).text().not_null())
            .col(
                ColumnDef::new(S3Assets::S3Key)
                    .text()
                    .not_null()
                    .unique_key(),
            )
            .col(ColumnDef::new(S3Assets::SizeBytes).big_integer())
            .col(ColumnDef::new(S3Assets::UploadedBy).text())
            .col(
                ColumnDef::new(S3Assets::UploadedAt)
                    .timestamp_with_time_zone()
                    .not_null()
                    .default(Expr::current_timestamp()),
            )
            .col(
                ColumnDef::new(S3Assets::IsDeleted)
                    .boolean()
                    .not_null()
                    .default(false),
            )
            .col(
                ColumnDef::new(S3Assets::CreatedAt)
                    .timestamp_with_time_zone()
                    .not_null()
                    .default(Expr::current_timestamp()),
            )
            .col(
                ColumnDef::new(S3Assets::LastUpdated)
                    .timestamp_with_time_zone()
                    .not_null()
                    .default(Expr::current_timestamp()),
            )
            .col(ColumnDef::new(S3Assets::Type).text().not_null())
            .col(ColumnDef::new(S3Assets::Role).text())
            .col(ColumnDef::new(S3Assets::ProcessingStatus).text())
            .col(ColumnDef::new(S3Assets::ProcessingMessage).text())
            .foreign_key(
                ForeignKey::create()
                    .name("s3_assets_experiment_id_fkey")
                    .from(S3Assets::Table, S3Assets::ExperimentId)
                    .to(Experiments::Table, Experiments::Id)
                    .on_delete(ForeignKeyAction::NoAction)
                    .on_update(ForeignKeyAction::NoAction),
            )
            .to_owned();

        // Add ID column with appropriate type and default based on database backend
        match manager.get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => {
                s3_assets_table.col(
                    ColumnDef::new(S3Assets::Id)
                        .uuid()
                        .not_null()
                        .primary_key()
                        .default(Expr::cust("uuid_generate_v4()")),
                );
            }
            sea_orm::DatabaseBackend::Sqlite => {
                s3_assets_table.col(ColumnDef::new(S3Assets::Id).uuid().not_null().primary_key());
            }
            _ => {
                return Err(DbErr::Custom("Unsupported database backend".to_string()));
            }
        }

        manager.create_table(s3_assets_table).await?;

        // Create temperature_readings table
        let mut temperature_readings_table = Table::create()
            .table(TemperatureReadings::Table)
            .if_not_exists()
            .col(
                ColumnDef::new(TemperatureReadings::ExperimentId)
                    .uuid()
                    .not_null(),
            )
            .col(
                ColumnDef::new(TemperatureReadings::Timestamp)
                    .timestamp_with_time_zone()
                    .not_null(),
            )
            .col(ColumnDef::new(TemperatureReadings::ImageFilename).text())
            .col(
                ColumnDef::new(TemperatureReadings::CreatedAt)
                    .timestamp_with_time_zone()
                    .not_null()
                    .default(Expr::current_timestamp()),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_temperature_readings_experiment")
                    .from(
                        TemperatureReadings::Table,
                        TemperatureReadings::ExperimentId,
                    )
                    .to(Experiments::Table, Experiments::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::NoAction),
            )
            .to_owned();

        // Add ID column with appropriate type and default based on database backend
        match manager.get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => {
                temperature_readings_table.col(
                    ColumnDef::new(TemperatureReadings::Id)
                        .uuid()
                        .not_null()
                        .primary_key()
                        .default(Expr::cust("uuid_generate_v4()")),
                );
            }
            sea_orm::DatabaseBackend::Sqlite => {
                temperature_readings_table.col(
                    ColumnDef::new(TemperatureReadings::Id)
                        .uuid()
                        .not_null()
                        .primary_key(),
                );
            }
            _ => {
                return Err(DbErr::Custom("Unsupported database backend".to_string()));
            }
        }

        manager.create_table(temperature_readings_table).await?;

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

        // Create well_phase_transitions table
        let mut well_phase_transitions_table = Table::create()
            .table(WellPhaseTransitions::Table)
            .if_not_exists()
            .col(
                ColumnDef::new(WellPhaseTransitions::WellId)
                    .uuid()
                    .not_null(),
            )
            .col(
                ColumnDef::new(WellPhaseTransitions::ExperimentId)
                    .uuid()
                    .not_null(),
            )
            .col(
                ColumnDef::new(WellPhaseTransitions::TemperatureReadingId)
                    .uuid()
                    .not_null(),
            )
            .col(
                ColumnDef::new(WellPhaseTransitions::Timestamp)
                    .timestamp_with_time_zone()
                    .not_null(),
            )
            .col(
                ColumnDef::new(WellPhaseTransitions::PreviousState)
                    .integer()
                    .not_null(),
            )
            .col(
                ColumnDef::new(WellPhaseTransitions::NewState)
                    .integer()
                    .not_null(),
            )
            .col(
                ColumnDef::new(WellPhaseTransitions::CreatedAt)
                    .timestamp_with_time_zone()
                    .not_null()
                    .default(Expr::current_timestamp()),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_well_phase_transitions_well")
                    .from(WellPhaseTransitions::Table, WellPhaseTransitions::WellId)
                    .to(Wells::Table, Wells::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::NoAction),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_well_phase_transitions_experiment")
                    .from(
                        WellPhaseTransitions::Table,
                        WellPhaseTransitions::ExperimentId,
                    )
                    .to(Experiments::Table, Experiments::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::NoAction),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_well_phase_transitions_temperature_reading")
                    .from(
                        WellPhaseTransitions::Table,
                        WellPhaseTransitions::TemperatureReadingId,
                    )
                    .to(TemperatureReadings::Table, TemperatureReadings::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::NoAction),
            )
            .to_owned();

        // Add ID column with appropriate type and default based on database backend
        match manager.get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => {
                well_phase_transitions_table.col(
                    ColumnDef::new(WellPhaseTransitions::Id)
                        .uuid()
                        .not_null()
                        .primary_key()
                        .default(Expr::cust("uuid_generate_v4()")),
                );
            }
            sea_orm::DatabaseBackend::Sqlite => {
                well_phase_transitions_table.col(
                    ColumnDef::new(WellPhaseTransitions::Id)
                        .uuid()
                        .not_null()
                        .primary_key(),
                );
            }
            _ => {
                return Err(DbErr::Custom("Unsupported database backend".to_string()));
            }
        }

        manager.create_table(well_phase_transitions_table).await?;

        // Note: All unique constraints now added during table creation for SQLite compatibility

        // All unique constraints moved to table creation for SQLite compatibility

        // Create non-unique indexes for performance
        manager
            .create_index(
                Index::create()
                    .name("idx_locations_project_id")
                    .table(Locations::Table)
                    .col(Locations::ProjectId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_samples_location_id")
                    .table(Samples::Table)
                    .col(Samples::LocationId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_experiments_tray_configuration_id")
                    .table(Experiments::Table)
                    .col(Experiments::TrayConfigurationId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_regions_experiment_id")
                    .table(Regions::Table)
                    .col(Regions::ExperimentId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_regions_treatment_id")
                    .table(Regions::Table)
                    .col(Regions::TreatmentId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_s3_assets_experiment_id")
                    .table(S3Assets::Table)
                    .col(S3Assets::ExperimentId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_s3_assets_original_filename")
                    .table(S3Assets::Table)
                    .col(S3Assets::OriginalFilename)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_trays_tray_configuration_id")
                    .table(Trays::Table)
                    .col(Trays::TrayConfigurationId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_wells_tray_id")
                    .table(Wells::Table)
                    .col(Wells::TrayId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_temperature_readings_experiment_timestamp")
                    .table(TemperatureReadings::Table)
                    .col(TemperatureReadings::ExperimentId)
                    .col(TemperatureReadings::Timestamp)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_well_phase_transitions_experiment")
                    .table(WellPhaseTransitions::Table)
                    .col(WellPhaseTransitions::ExperimentId)
                    .col(WellPhaseTransitions::Timestamp)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_well_phase_transitions_well_timestamp")
                    .table(WellPhaseTransitions::Table)
                    .col(WellPhaseTransitions::WellId)
                    .col(WellPhaseTransitions::Timestamp)
                    .to_owned(),
            )
            .await?;

        // Create performance indexes for experiments table based on crudcrate analysis

        // High Priority: Fulltext search index for PostgreSQL (exact match to crudcrate analysis)
        if manager.get_database_backend() == sea_orm::DatabaseBackend::Postgres {
            manager
                .get_connection()
                .execute_unprepared(
                    "CREATE INDEX idx_experiments_fulltext ON experiments USING GIN (to_tsvector('english', name || ' ' || username || ' ' || remarks))"
                )
                .await?;
        }

        // Medium Priority: Individual column indexes for filterable/sortable fields

        // Username index (filterable & sortable)
        manager
            .create_index(
                Index::create()
                    .name("idx_experiments_username")
                    .table(Experiments::Table)
                    .col(Experiments::Username)
                    .to_owned(),
            )
            .await?;

        // Performed at index (filterable & sortable)
        manager
            .create_index(
                Index::create()
                    .name("idx_experiments_performed_at")
                    .table(Experiments::Table)
                    .col(Experiments::PerformedAt)
                    .to_owned(),
            )
            .await?;

        // Temperature ramp index (filterable & sortable)
        manager
            .create_index(
                Index::create()
                    .name("idx_experiments_temperature_ramp")
                    .table(Experiments::Table)
                    .col(Experiments::TemperatureRamp)
                    .to_owned(),
            )
            .await?;

        // Temperature start index (filterable & sortable)
        manager
            .create_index(
                Index::create()
                    .name("idx_experiments_temperature_start")
                    .table(Experiments::Table)
                    .col(Experiments::TemperatureStart)
                    .to_owned(),
            )
            .await?;

        // Temperature end index (filterable & sortable)
        manager
            .create_index(
                Index::create()
                    .name("idx_experiments_temperature_end")
                    .table(Experiments::Table)
                    .col(Experiments::TemperatureEnd)
                    .to_owned(),
            )
            .await?;

        // Is calibration index (filterable)
        manager
            .create_index(
                Index::create()
                    .name("idx_experiments_is_calibration")
                    .table(Experiments::Table)
                    .col(Experiments::IsCalibration)
                    .to_owned(),
            )
            .await?;

        // Remarks index (filterable & sortable)
        manager
            .create_index(
                Index::create()
                    .name("idx_experiments_remarks")
                    .table(Experiments::Table)
                    .col(Experiments::Remarks)
                    .to_owned(),
            )
            .await?;

        // Created at index (sortable)
        manager
            .create_index(
                Index::create()
                    .name("idx_experiments_created_at")
                    .table(Experiments::Table)
                    .col(Experiments::CreatedAt)
                    .to_owned(),
            )
            .await?;

        // Last updated index (sortable)
        manager
            .create_index(
                Index::create()
                    .name("idx_experiments_last_updated")
                    .table(Experiments::Table)
                    .col(Experiments::LastUpdated)
                    .to_owned(),
            )
            .await?;

        // ============ COMPREHENSIVE PERFORMANCE INDEXES ============
        
        // ============ LOCATIONS TABLE INDEXES ============
        manager
            .create_index(
                Index::create()
                    .name("idx_locations_comment")
                    .table(Locations::Table)
                    .col(Locations::Comment)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_locations_created_at")
                    .table(Locations::Table)
                    .col(Locations::CreatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_locations_last_updated")
                    .table(Locations::Table)
                    .col(Locations::LastUpdated)
                    .to_owned(),
            )
            .await?;

        // Locations fulltext index
        if manager.get_database_backend() == sea_orm::DatabaseBackend::Postgres {
            manager
                .get_connection()
                .execute_unprepared(
                    "CREATE INDEX idx_locations_fulltext ON locations USING GIN (to_tsvector('english', name || ' ' || comment))"
                )
                .await?;
        }

        // ============ PROJECTS TABLE INDEXES ============
        manager
            .create_index(
                Index::create()
                    .name("idx_projects_note")
                    .table(Projects::Table)
                    .col(Projects::Note)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_projects_colour")
                    .table(Projects::Table)
                    .col(Projects::Colour)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_projects_created_at")
                    .table(Projects::Table)
                    .col(Projects::CreatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_projects_last_updated")
                    .table(Projects::Table)
                    .col(Projects::LastUpdated)
                    .to_owned(),
            )
            .await?;

        // Projects fulltext index
        if manager.get_database_backend() == sea_orm::DatabaseBackend::Postgres {
            manager
                .get_connection()
                .execute_unprepared(
                    "CREATE INDEX idx_projects_fulltext ON projects USING GIN (to_tsvector('english', name || ' ' || note || ' ' || colour))"
                )
                .await?;
        }

        // ============ SAMPLES TABLE INDEXES ============
        manager
            .create_index(
                Index::create()
                    .name("idx_samples_name")
                    .table(Samples::Table)
                    .col(Samples::Name)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_samples_type")
                    .table(Samples::Table)
                    .col(Samples::Type)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_samples_flow_litres_per_minute")
                    .table(Samples::Table)
                    .col(Samples::FlowLitresPerMinute)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_samples_total_volume")
                    .table(Samples::Table)
                    .col(Samples::TotalVolume)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_samples_material_description")
                    .table(Samples::Table)
                    .col(Samples::MaterialDescription)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_samples_extraction_procedure")
                    .table(Samples::Table)
                    .col(Samples::ExtractionProcedure)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_samples_filter_substrate")
                    .table(Samples::Table)
                    .col(Samples::FilterSubstrate)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_samples_suspension_volume_litres")
                    .table(Samples::Table)
                    .col(Samples::SuspensionVolumeLitres)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_samples_air_volume_litres")
                    .table(Samples::Table)
                    .col(Samples::AirVolumeLitres)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_samples_water_volume_litres")
                    .table(Samples::Table)
                    .col(Samples::WaterVolumeLitres)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_samples_initial_concentration_gram_l")
                    .table(Samples::Table)
                    .col(Samples::InitialConcentrationGramL)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_samples_well_volume_litres")
                    .table(Samples::Table)
                    .col(Samples::WellVolumeLitres)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_samples_remarks")
                    .table(Samples::Table)
                    .col(Samples::Remarks)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_samples_start_time")
                    .table(Samples::Table)
                    .col(Samples::StartTime)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_samples_stop_time")
                    .table(Samples::Table)
                    .col(Samples::StopTime)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_samples_created_at")
                    .table(Samples::Table)
                    .col(Samples::CreatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_samples_last_updated")
                    .table(Samples::Table)
                    .col(Samples::LastUpdated)
                    .to_owned(),
            )
            .await?;

        // Samples fulltext index
        if manager.get_database_backend() == sea_orm::DatabaseBackend::Postgres {
            manager
                .get_connection()
                .execute_unprepared(
                    "CREATE INDEX idx_samples_fulltext ON samples USING GIN (to_tsvector('english', name || ' ' || material_description || ' ' || extraction_procedure || ' ' || filter_substrate || ' ' || remarks))"
                )
                .await?;
        }

        // ============ S3_ASSETS TABLE INDEXES ============
        manager
            .create_index(
                Index::create()
                    .name("idx_s3_assets_uploaded_by")
                    .table(S3Assets::Table)
                    .col(S3Assets::UploadedBy)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_s3_assets_is_deleted")
                    .table(S3Assets::Table)
                    .col(S3Assets::IsDeleted)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_s3_assets_type")
                    .table(S3Assets::Table)
                    .col(S3Assets::Type)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_s3_assets_role")
                    .table(S3Assets::Table)
                    .col(S3Assets::Role)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_s3_assets_processing_status")
                    .table(S3Assets::Table)
                    .col(S3Assets::ProcessingStatus)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_s3_assets_processing_message")
                    .table(S3Assets::Table)
                    .col(S3Assets::ProcessingMessage)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_s3_assets_size_bytes")
                    .table(S3Assets::Table)
                    .col(S3Assets::SizeBytes)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_s3_assets_uploaded_at")
                    .table(S3Assets::Table)
                    .col(S3Assets::UploadedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_s3_assets_created_at")
                    .table(S3Assets::Table)
                    .col(S3Assets::CreatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_s3_assets_last_updated")
                    .table(S3Assets::Table)
                    .col(S3Assets::LastUpdated)
                    .to_owned(),
            )
            .await?;

        // S3Assets fulltext index
        if manager.get_database_backend() == sea_orm::DatabaseBackend::Postgres {
            manager
                .get_connection()
                .execute_unprepared(
                    "CREATE INDEX idx_s3_assets_fulltext ON s3_assets USING GIN (to_tsvector('english', original_filename || ' ' || s3_key || ' ' || uploaded_by || ' ' || type || ' ' || role))"
                )
                .await?;
        }

        // ============ TRAY_CONFIGURATIONS TABLE INDEXES ============
        manager
            .create_index(
                Index::create()
                    .name("idx_tray_configurations_experiment_default")
                    .table(TrayConfigurations::Table)
                    .col(TrayConfigurations::ExperimentDefault)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_tray_configurations_created_at")
                    .table(TrayConfigurations::Table)
                    .col(TrayConfigurations::CreatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_tray_configurations_last_updated")
                    .table(TrayConfigurations::Table)
                    .col(TrayConfigurations::LastUpdated)
                    .to_owned(),
            )
            .await?;

        // Tray configurations fulltext index
        if manager.get_database_backend() == sea_orm::DatabaseBackend::Postgres {
            manager
                .get_connection()
                .execute_unprepared(
                    "CREATE INDEX idx_tray_configurations_fulltext ON tray_configurations USING GIN (to_tsvector('english', name))"
                )
                .await?;
        }

        // ============ TREATMENTS TABLE INDEXES ============
        manager
            .create_index(
                Index::create()
                    .name("idx_treatments_name")
                    .table(Treatments::Table)
                    .col(Treatments::Name)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_treatments_notes")
                    .table(Treatments::Table)
                    .col(Treatments::Notes)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_treatments_sample_id")
                    .table(Treatments::Table)
                    .col(Treatments::SampleId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_treatments_enzyme_volume_litres")
                    .table(Treatments::Table)
                    .col(Treatments::EnzymeVolumeLitres)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_treatments_created_at")
                    .table(Treatments::Table)
                    .col(Treatments::CreatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_treatments_last_updated")
                    .table(Treatments::Table)
                    .col(Treatments::LastUpdated)
                    .to_owned(),
            )
            .await?;

        // Treatments fulltext index
        if manager.get_database_backend() == sea_orm::DatabaseBackend::Postgres {
            manager
                .get_connection()
                .execute_unprepared(
                    "CREATE INDEX idx_treatments_fulltext ON treatments USING GIN (to_tsvector('english', notes))"
                )
                .await?;
        }

        // ============ PROBE INDEXES ============
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

        // Add spatial indexes for PostGIS geometry column (PostgreSQL only)
        if manager.get_database_backend() == sea_orm::DatabaseBackend::Postgres {
            // Spatial index on samples.geom for efficient spatial queries
            manager
                .get_connection()
                .execute_unprepared("CREATE INDEX idx_samples_geom ON samples USING GIST (geom);")
                .await?;
            
            // Optional: Coordinate indexes for filtering by lat/lon directly
            manager
                .create_index(
                    Index::create()
                        .name("idx_samples_latitude")
                        .table(Samples::Table)
                        .col(Samples::Latitude)
                        .to_owned(),
                )
                .await?;
            
            manager
                .create_index(
                    Index::create()
                        .name("idx_samples_longitude")
                        .table(Samples::Table)
                        .col(Samples::Longitude)
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }

    #[allow(clippy::too_many_lines)] // Large rollback requires extensive table drops
    #[allow(clippy::match_wildcard_for_single_variants)] // Wildcard matches for unsupported databases are semantically correct
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // ============ DROP COMPREHENSIVE PERFORMANCE INDEXES ============
        
        // Drop PostgreSQL fulltext indexes if they exist
        if manager.get_database_backend() == sea_orm::DatabaseBackend::Postgres {
            manager
                .get_connection()
                .execute_unprepared("DROP INDEX IF EXISTS idx_locations_fulltext")
                .await
                .ok();
            manager
                .get_connection()
                .execute_unprepared("DROP INDEX IF EXISTS idx_projects_fulltext")
                .await
                .ok();
            manager
                .get_connection()
                .execute_unprepared("DROP INDEX IF EXISTS idx_samples_fulltext")
                .await
                .ok();
            manager
                .get_connection()
                .execute_unprepared("DROP INDEX IF EXISTS idx_s3_assets_fulltext")
                .await
                .ok();
            manager
                .get_connection()
                .execute_unprepared("DROP INDEX IF EXISTS idx_tray_configurations_fulltext")
                .await
                .ok();
            manager
                .get_connection()
                .execute_unprepared("DROP INDEX IF EXISTS idx_treatments_fulltext")
                .await
                .ok();
        }

        // Drop probe indexes
        manager
            .drop_index(
                Index::drop()
                    .name("idx_probe_temp_readings_probe_id")
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();
        manager
            .drop_index(
                Index::drop()
                    .name("idx_probes_tray_id")
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();

        // Drop treatments indexes
        manager
            .drop_index(
                Index::drop()
                    .name("idx_treatments_last_updated")
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();
        manager
            .drop_index(
                Index::drop()
                    .name("idx_treatments_created_at")
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();
        manager
            .drop_index(
                Index::drop()
                    .name("idx_treatments_enzyme_volume_litres")
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();
        manager
            .drop_index(
                Index::drop()
                    .name("idx_treatments_sample_id")
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();
        manager
            .drop_index(
                Index::drop()
                    .name("idx_treatments_notes")
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();
        manager
            .drop_index(
                Index::drop()
                    .name("idx_treatments_name")
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();

        // Drop tray configurations indexes
        manager
            .drop_index(
                Index::drop()
                    .name("idx_tray_configurations_last_updated")
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();
        manager
            .drop_index(
                Index::drop()
                    .name("idx_tray_configurations_created_at")
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();
        manager
            .drop_index(
                Index::drop()
                    .name("idx_tray_configurations_experiment_default")
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();

        // Drop S3Assets indexes
        manager
            .drop_index(
                Index::drop()
                    .name("idx_s3_assets_last_updated")
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();
        manager
            .drop_index(
                Index::drop()
                    .name("idx_s3_assets_created_at")
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();
        manager
            .drop_index(
                Index::drop()
                    .name("idx_s3_assets_uploaded_at")
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();
        manager
            .drop_index(
                Index::drop()
                    .name("idx_s3_assets_size_bytes")
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();
        manager
            .drop_index(
                Index::drop()
                    .name("idx_s3_assets_processing_message")
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();
        manager
            .drop_index(
                Index::drop()
                    .name("idx_s3_assets_processing_status")
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();
        manager
            .drop_index(
                Index::drop()
                    .name("idx_s3_assets_role")
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();
        manager
            .drop_index(
                Index::drop()
                    .name("idx_s3_assets_type")
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();
        manager
            .drop_index(
                Index::drop()
                    .name("idx_s3_assets_is_deleted")
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();
        manager
            .drop_index(
                Index::drop()
                    .name("idx_s3_assets_uploaded_by")
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();

        // Drop Samples indexes
        manager
            .drop_index(
                Index::drop()
                    .name("idx_samples_last_updated")
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();
        manager
            .drop_index(
                Index::drop()
                    .name("idx_samples_created_at")
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();
        manager
            .drop_index(
                Index::drop()
                    .name("idx_samples_stop_time")
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();
        manager
            .drop_index(
                Index::drop()
                    .name("idx_samples_start_time")
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();
        manager
            .drop_index(
                Index::drop()
                    .name("idx_samples_remarks")
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();
        manager
            .drop_index(
                Index::drop()
                    .name("idx_samples_well_volume_litres")
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();
        manager
            .drop_index(
                Index::drop()
                    .name("idx_samples_initial_concentration_gram_l")
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();
        manager
            .drop_index(
                Index::drop()
                    .name("idx_samples_water_volume_litres")
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();
        manager
            .drop_index(
                Index::drop()
                    .name("idx_samples_air_volume_litres")
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();
        manager
            .drop_index(
                Index::drop()
                    .name("idx_samples_suspension_volume_litres")
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();
        manager
            .drop_index(
                Index::drop()
                    .name("idx_samples_filter_substrate")
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();
        manager
            .drop_index(
                Index::drop()
                    .name("idx_samples_extraction_procedure")
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();
        manager
            .drop_index(
                Index::drop()
                    .name("idx_samples_material_description")
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();
        manager
            .drop_index(
                Index::drop()
                    .name("idx_samples_total_volume")
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();
        manager
            .drop_index(
                Index::drop()
                    .name("idx_samples_flow_litres_per_minute")
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();
        manager
            .drop_index(
                Index::drop()
                    .name("idx_samples_type")
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();
        manager
            .drop_index(
                Index::drop()
                    .name("idx_samples_name")
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();

        // Drop Projects indexes
        manager
            .drop_index(
                Index::drop()
                    .name("idx_projects_last_updated")
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();
        manager
            .drop_index(
                Index::drop()
                    .name("idx_projects_created_at")
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();
        manager
            .drop_index(
                Index::drop()
                    .name("idx_projects_colour")
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();
        manager
            .drop_index(
                Index::drop()
                    .name("idx_projects_note")
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();

        // Drop Locations indexes
        manager
            .drop_index(
                Index::drop()
                    .name("idx_locations_last_updated")
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();
        manager
            .drop_index(
                Index::drop()
                    .name("idx_locations_created_at")
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();
        manager
            .drop_index(
                Index::drop()
                    .name("idx_locations_comment")
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();

        // Drop procedural blank constraint
        match manager.get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => {
                manager
                    .get_connection()
                    .execute_unprepared("ALTER TABLE samples DROP CONSTRAINT IF EXISTS chk_procedural_blank_no_location")
                    .await
                    .ok();
            }
            sea_orm::DatabaseBackend::Sqlite => {
                // No constraint to remove in SQLite
            }
            _ => {}
        }

        // Drop spatial indexes for PostgreSQL
        if manager.get_database_backend() == sea_orm::DatabaseBackend::Postgres {
            manager
                .get_connection()
                .execute_unprepared("DROP INDEX IF EXISTS idx_samples_geom;")
                .await
                .ok();
            
            manager
                .drop_index(
                    Index::drop()
                        .name("idx_samples_latitude")
                        .table(Samples::Table)
                        .if_exists()
                        .to_owned(),
                )
                .await
                .ok();
            
            manager
                .drop_index(
                    Index::drop()
                        .name("idx_samples_longitude")
                        .table(Samples::Table)
                        .if_exists()
                        .to_owned(),
                )
                .await
                .ok();
        }

        // Drop tables in reverse dependency order
        manager
            .drop_table(
                Table::drop()
                    .table(WellPhaseTransitions::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(TemperatureReadings::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(S3Assets::Table).if_exists().to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Regions::Table).if_exists().to_owned())
            .await?;
        // Drop probe tables (probe_temperature_readings first due to foreign keys)
        manager
            .drop_table(
                Table::drop()
                    .table(ProbeTemperatureReadings::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(Probes::Table).if_exists().to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Wells::Table).if_exists().to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Trays::Table).if_exists().to_owned())
            .await?;
        // Drop experiments performance indexes first
        manager
            .drop_index(
                Index::drop()
                    .name("idx_experiments_last_updated")
                    .table(Experiments::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();
        manager
            .drop_index(
                Index::drop()
                    .name("idx_experiments_created_at")
                    .table(Experiments::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();
        manager
            .drop_index(
                Index::drop()
                    .name("idx_experiments_remarks")
                    .table(Experiments::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();
        manager
            .drop_index(
                Index::drop()
                    .name("idx_experiments_is_calibration")
                    .table(Experiments::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();
        manager
            .drop_index(
                Index::drop()
                    .name("idx_experiments_temperature_end")
                    .table(Experiments::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();
        manager
            .drop_index(
                Index::drop()
                    .name("idx_experiments_temperature_start")
                    .table(Experiments::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();
        manager
            .drop_index(
                Index::drop()
                    .name("idx_experiments_temperature_ramp")
                    .table(Experiments::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();
        manager
            .drop_index(
                Index::drop()
                    .name("idx_experiments_performed_at")
                    .table(Experiments::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();
        manager
            .drop_index(
                Index::drop()
                    .name("idx_experiments_username")
                    .table(Experiments::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await
            .ok();

        // Drop PostgreSQL fulltext index if it exists
        if manager.get_database_backend() == sea_orm::DatabaseBackend::Postgres {
            manager
                .get_connection()
                .execute_unprepared("DROP INDEX IF EXISTS idx_experiments_fulltext")
                .await
                .ok();
        }

        manager
            .drop_table(
                Table::drop()
                    .table(Experiments::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(TrayConfigurations::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(Treatments::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(Samples::Table).if_exists().to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Locations::Table).if_exists().to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Projects::Table).if_exists().to_owned())
            .await?;

        // Drop enums for PostgreSQL
        if manager.get_database_backend() == sea_orm::DatabaseBackend::Postgres {
            manager
                .drop_type(
                    Type::drop()
                        .name(TreatmentName::Table)
                        .if_exists()
                        .to_owned(),
                )
                .await?;
            manager
                .drop_type(Type::drop().name(SampleType::Table).if_exists().to_owned())
                .await?;
        }

        Ok(())
    }
}

// All table and enum identifiers
#[derive(DeriveIden)]
enum Projects {
    Table,
    Id,
    Name,
    Note,
    Colour,
    CreatedAt,
    LastUpdated,
}

#[derive(DeriveIden)]
enum Locations {
    Table,
    Id,
    Name,
    Comment,
    ProjectId,
    CreatedAt,
    LastUpdated,
}

#[derive(DeriveIden)]
enum Samples {
    Table,
    Id,
    Name,
    Type,
    StartTime,
    StopTime,
    FlowLitresPerMinute,
    TotalVolume,
    MaterialDescription,
    ExtractionProcedure,
    FilterSubstrate,
    SuspensionVolumeLitres,
    AirVolumeLitres,
    WaterVolumeLitres,
    InitialConcentrationGramL,
    WellVolumeLitres,
    Remarks,
    Longitude,
    Latitude,
    LocationId,
    CreatedAt,
    LastUpdated,
}

#[derive(DeriveIden)]
enum Treatments {
    Table,
    Id,
    Name,
    Notes,
    SampleId,
    LastUpdated,
    CreatedAt,
    EnzymeVolumeLitres,
}

#[derive(DeriveIden)]
enum TrayConfigurations {
    Table,
    Id,
    Name,
    ExperimentDefault,
    CreatedAt,
    LastUpdated,
}

#[derive(DeriveIden)]
enum Experiments {
    Table,
    Id,
    Name,
    Username,
    PerformedAt,
    TemperatureRamp,
    TemperatureStart,
    TemperatureEnd,
    IsCalibration,
    Remarks,
    TrayConfigurationId,
    CreatedAt,
    LastUpdated,
    HasResults,
}

#[derive(DeriveIden)]
enum Trays {
    Table,
    Id,
    TrayConfigurationId,
    OrderSequence,
    RotationDegrees,
    CreatedAt,
    LastUpdated,
    Name,
    QtyCols,
    QtyRows,
    WellRelativeDiameter,
    UpperLeftCornerX,
    UpperLeftCornerY,
    LowerRightCornerX,
    LowerRightCornerY,
}

#[derive(DeriveIden)]
enum Wells {
    Table,
    Id,
    TrayId,
    RowLetter,
    ColumnNumber,
    CreatedAt,
    LastUpdated,
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
enum S3Assets {
    Table,
    Id,
    ExperimentId,
    OriginalFilename,
    S3Key,
    SizeBytes,
    UploadedBy,
    UploadedAt,
    IsDeleted,
    CreatedAt,
    LastUpdated,
    Type,
    Role,
    ProcessingStatus,
    ProcessingMessage,
}

#[derive(DeriveIden)]
enum TemperatureReadings {
    Table,
    Id,
    ExperimentId,
    Timestamp,
    ImageFilename,
    CreatedAt,
}

#[derive(DeriveIden)]
enum WellPhaseTransitions {
    Table,
    Id,
    WellId,
    ExperimentId,
    TemperatureReadingId,
    Timestamp,
    PreviousState,
    NewState,
    CreatedAt,
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
enum SampleType {
    Table,
    Bulk,
    Filter,
    ProceduralBlank,
}

#[derive(DeriveIden)]
enum TreatmentName {
    Table,
    None,
    Heat,
    H2o2,
}
