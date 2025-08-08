use sea_orm_migration::prelude::*;
use sea_orm_migration::prelude::extension::postgres::Type;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Enable UUID extension for PostgreSQL
        if manager.get_database_backend() == sea_orm::DatabaseBackend::Postgres {
            manager
                .get_connection()
                .execute_unprepared("CREATE EXTENSION IF NOT EXISTS \"uuid-ossp\";")
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
                            SampleType::PureWater,
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
            .col(ColumnDef::new(Projects::Name).string().not_null().unique_key())
            .col(ColumnDef::new(Projects::Note).text())
            .col(ColumnDef::new(Projects::Colour).string())
            .col(ColumnDef::new(Projects::CreatedAt).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
            .col(ColumnDef::new(Projects::LastUpdated).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
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
            .col(ColumnDef::new(Locations::Name).string().not_null().unique_key())
            .col(ColumnDef::new(Locations::Comment).text())
            .col(ColumnDef::new(Locations::ProjectId).uuid())
            .col(ColumnDef::new(Locations::LastUpdated).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
            .col(ColumnDef::new(Locations::CreatedAt).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
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
                locations_table.col(ColumnDef::new(Locations::Id).uuid().not_null().primary_key());
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
            .col(ColumnDef::new(Samples::CreatedAt).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
            .col(ColumnDef::new(Samples::LastUpdated).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
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

        // Create treatments table
        let mut treatments_table = Table::create()
            .table(Treatments::Table)
            .if_not_exists()
            .col(ColumnDef::new(Treatments::Notes).text())
            .col(ColumnDef::new(Treatments::SampleId).uuid())
            .col(ColumnDef::new(Treatments::LastUpdated).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
            .col(ColumnDef::new(Treatments::CreatedAt).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
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
                treatments_table.col(ColumnDef::new(Treatments::Id).uuid().not_null().primary_key());
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
            .col(ColumnDef::new(TrayConfigurations::ExperimentDefault).boolean().not_null())
            .col(ColumnDef::new(TrayConfigurations::CreatedAt).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
            .col(ColumnDef::new(TrayConfigurations::LastUpdated).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
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
                tray_configurations_table.col(ColumnDef::new(TrayConfigurations::Id).uuid().not_null().primary_key());
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
            .col(ColumnDef::new(Experiments::Name).text().not_null().unique_key())
            .col(ColumnDef::new(Experiments::Username).text().null())
            .col(ColumnDef::new(Experiments::PerformedAt).timestamp_with_time_zone().null())
            .col(ColumnDef::new(Experiments::TemperatureRamp).decimal().null())
            .col(ColumnDef::new(Experiments::TemperatureStart).decimal().null())
            .col(ColumnDef::new(Experiments::TemperatureEnd).decimal().null())
            .col(ColumnDef::new(Experiments::IsCalibration).boolean().not_null().default(false))
            .col(ColumnDef::new(Experiments::Remarks).text().null())
            .col(ColumnDef::new(Experiments::TrayConfigurationId).uuid().null())
            .col(ColumnDef::new(Experiments::CreatedAt).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
            .col(ColumnDef::new(Experiments::LastUpdated).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
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
                experiments_table.col(ColumnDef::new(Experiments::Id).uuid().not_null().primary_key());
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
            .col(ColumnDef::new(Trays::CreatedAt).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
            .col(ColumnDef::new(Trays::LastUpdated).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
            .col(ColumnDef::new(Trays::Name).text())
            .col(ColumnDef::new(Trays::QtyXAxis).integer())
            .col(ColumnDef::new(Trays::QtyYAxis).integer())
            .col(ColumnDef::new(Trays::WellRelativeDiameter).decimal())
            .foreign_key(
                ForeignKey::create()
                    .name("fk_tray_assignment_to_configuration")
                    .from(Trays::Table, Trays::TrayConfigurationId)
                    .to(TrayConfigurations::Table, TrayConfigurations::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::NoAction),
            )
            // Add composite unique constraint for SQLite compatibility
            .index(Index::create().name("trays_config_sequence_unique").col(Trays::TrayConfigurationId).col(Trays::OrderSequence).unique())
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
            .col(ColumnDef::new(Wells::ColumnNumber).integer().not_null())
            .col(ColumnDef::new(Wells::RowNumber).integer().not_null())
            .col(ColumnDef::new(Wells::CreatedAt).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
            .col(ColumnDef::new(Wells::LastUpdated).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
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
            .col(ColumnDef::new(Regions::CreatedAt).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
            .col(ColumnDef::new(Regions::LastUpdated).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
            .col(ColumnDef::new(Regions::IsBackgroundKey).boolean().not_null().default(false))
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
            .col(ColumnDef::new(S3Assets::S3Key).text().not_null().unique_key())
            .col(ColumnDef::new(S3Assets::SizeBytes).big_integer())
            .col(ColumnDef::new(S3Assets::UploadedBy).text())
            .col(ColumnDef::new(S3Assets::UploadedAt).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
            .col(ColumnDef::new(S3Assets::IsDeleted).boolean().not_null().default(false))
            .col(ColumnDef::new(S3Assets::CreatedAt).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
            .col(ColumnDef::new(S3Assets::LastUpdated).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
            .col(ColumnDef::new(S3Assets::Type).text().not_null())
            .col(ColumnDef::new(S3Assets::Role).text())
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
            .col(ColumnDef::new(TemperatureReadings::ExperimentId).uuid().not_null())
            .col(ColumnDef::new(TemperatureReadings::Timestamp).timestamp_with_time_zone().not_null())
            .col(ColumnDef::new(TemperatureReadings::ImageFilename).text())
            .col(ColumnDef::new(TemperatureReadings::Probe_1).decimal())
            .col(ColumnDef::new(TemperatureReadings::Probe_2).decimal())
            .col(ColumnDef::new(TemperatureReadings::Probe_3).decimal())
            .col(ColumnDef::new(TemperatureReadings::Probe_4).decimal())
            .col(ColumnDef::new(TemperatureReadings::Probe_5).decimal())
            .col(ColumnDef::new(TemperatureReadings::Probe_6).decimal())
            .col(ColumnDef::new(TemperatureReadings::Probe_7).decimal())
            .col(ColumnDef::new(TemperatureReadings::Probe_8).decimal())
            .col(ColumnDef::new(TemperatureReadings::CreatedAt).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
            .foreign_key(
                ForeignKey::create()
                    .name("fk_temperature_readings_experiment")
                    .from(TemperatureReadings::Table, TemperatureReadings::ExperimentId)
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
                temperature_readings_table.col(ColumnDef::new(TemperatureReadings::Id).uuid().not_null().primary_key());
            }
            _ => {
                return Err(DbErr::Custom("Unsupported database backend".to_string()));
            }
        }

        manager.create_table(temperature_readings_table).await?;

        // Create well_phase_transitions table
        let mut well_phase_transitions_table = Table::create()
            .table(WellPhaseTransitions::Table)
            .if_not_exists()
            .col(ColumnDef::new(WellPhaseTransitions::WellId).uuid().not_null())
            .col(ColumnDef::new(WellPhaseTransitions::ExperimentId).uuid().not_null())
            .col(ColumnDef::new(WellPhaseTransitions::TemperatureReadingId).uuid().not_null())
            .col(ColumnDef::new(WellPhaseTransitions::Timestamp).timestamp_with_time_zone().not_null())
            .col(ColumnDef::new(WellPhaseTransitions::PreviousState).integer().not_null())
            .col(ColumnDef::new(WellPhaseTransitions::NewState).integer().not_null())
            .col(ColumnDef::new(WellPhaseTransitions::CreatedAt).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
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
                    .from(WellPhaseTransitions::Table, WellPhaseTransitions::ExperimentId)
                    .to(Experiments::Table, Experiments::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::NoAction),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_well_phase_transitions_temperature_reading")
                    .from(WellPhaseTransitions::Table, WellPhaseTransitions::TemperatureReadingId)
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
                well_phase_transitions_table.col(ColumnDef::new(WellPhaseTransitions::Id).uuid().not_null().primary_key());
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

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop tables in reverse dependency order
        manager.drop_table(Table::drop().table(WellPhaseTransitions::Table).if_exists().to_owned()).await?;
        manager.drop_table(Table::drop().table(TemperatureReadings::Table).if_exists().to_owned()).await?;
        manager.drop_table(Table::drop().table(S3Assets::Table).if_exists().to_owned()).await?;
        manager.drop_table(Table::drop().table(Regions::Table).if_exists().to_owned()).await?;
        manager.drop_table(Table::drop().table(Wells::Table).if_exists().to_owned()).await?;
        manager.drop_table(Table::drop().table(Trays::Table).if_exists().to_owned()).await?;
        manager.drop_table(Table::drop().table(Experiments::Table).if_exists().to_owned()).await?;
        manager.drop_table(Table::drop().table(TrayConfigurations::Table).if_exists().to_owned()).await?;
        manager.drop_table(Table::drop().table(Treatments::Table).if_exists().to_owned()).await?;
        manager.drop_table(Table::drop().table(Samples::Table).if_exists().to_owned()).await?;
        manager.drop_table(Table::drop().table(Locations::Table).if_exists().to_owned()).await?;
        manager.drop_table(Table::drop().table(Projects::Table).if_exists().to_owned()).await?;

        // Drop enums for PostgreSQL
        if manager.get_database_backend() == sea_orm::DatabaseBackend::Postgres {
            manager.drop_type(Type::drop().name(TreatmentName::Table).if_exists().to_owned()).await?;
            manager.drop_type(Type::drop().name(SampleType::Table).if_exists().to_owned()).await?;
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
    QtyXAxis,
    QtyYAxis,
    WellRelativeDiameter,
}

#[derive(DeriveIden)]
enum Wells {
    Table,
    Id,
    TrayId,
    ColumnNumber,
    RowNumber,
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
}

#[derive(DeriveIden)]
enum TemperatureReadings {
    Table,
    Id,
    ExperimentId,
    Timestamp,
    ImageFilename,
    Probe_1,
    Probe_2,
    Probe_3,
    Probe_4,
    Probe_5,
    Probe_6,
    Probe_7,
    Probe_8,
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
enum SampleType {
    Table,
    Bulk,
    Filter,
    ProceduralBlank,
    PureWater,
}

#[derive(DeriveIden)]
enum TreatmentName {
    Table,
    None,
    Heat,
    H2o2,
}