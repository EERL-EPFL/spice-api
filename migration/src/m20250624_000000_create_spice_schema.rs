use sea_orm_migration::prelude::extension::postgres::Type;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
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
            .col(ColumnDef::new(Projects::Name).string().not_null())
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

        // Create locations table (formerly campaigns)
        let mut locations_table = Table::create()
            .table(Locations::Table)
            .if_not_exists()
            .col(ColumnDef::new(Locations::Name).string().not_null())
            .col(ColumnDef::new(Locations::Comment).text())
            .col(ColumnDef::new(Locations::StartDate).timestamp_with_time_zone())
            .col(ColumnDef::new(Locations::EndDate).timestamp_with_time_zone())
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
            .col(ColumnDef::new(Locations::ProjectId).uuid())
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

        // Add foreign key constraint for SQLite
        if manager.get_database_backend() == sea_orm::DatabaseBackend::Sqlite {
            locations_table.foreign_key(
                &mut ForeignKey::create()
                    .name("fk_locations_project_id")
                    .from(Locations::Table, Locations::ProjectId)
                    .to(Projects::Table, Projects::Id)
                    .on_delete(ForeignKeyAction::SetNull)
                    .on_update(ForeignKeyAction::NoAction),
            );
        }

        manager.create_table(locations_table).await?;

        // Create tray_configurations table
        let mut tray_configurations_table = Table::create()
            .table(TrayConfigurations::Table)
            .if_not_exists()
            .col(ColumnDef::new(TrayConfigurations::Name).text())
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
            .col(ColumnDef::new(Experiments::Name).text().not_null())
            .col(ColumnDef::new(Experiments::Username).text())
            .col(ColumnDef::new(Experiments::PerformedAt).timestamp_with_time_zone())
            .col(ColumnDef::new(Experiments::TemperatureRamp).decimal())
            .col(ColumnDef::new(Experiments::TemperatureStart).decimal())
            .col(ColumnDef::new(Experiments::TemperatureEnd).decimal())
            .col(
                ColumnDef::new(Experiments::IsCalibration)
                    .boolean()
                    .not_null()
                    .default(false),
            )
            .col(ColumnDef::new(Experiments::Remarks).text())
            .col(ColumnDef::new(Experiments::TrayConfigurationId).uuid())
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

        // Add foreign key constraint for SQLite
        if manager.get_database_backend() == sea_orm::DatabaseBackend::Sqlite {
            experiments_table.foreign_key(
                &mut ForeignKey::create()
                    .name("fk_experiment_tray_configuration")
                    .from(Experiments::Table, Experiments::TrayConfigurationId)
                    .to(TrayConfigurations::Table, TrayConfigurations::Id)
                    .on_delete(ForeignKeyAction::NoAction)
                    .on_update(ForeignKeyAction::NoAction),
            );
        }

        manager.create_table(experiments_table).await?;

        // Create trays table
        let mut trays_table = Table::create()
            .table(Trays::Table)
            .if_not_exists()
            .col(ColumnDef::new(Trays::Name).text())
            .col(ColumnDef::new(Trays::QtyXAxis).integer().default(8))
            .col(ColumnDef::new(Trays::QtyYAxis).integer().default(12))
            .col(ColumnDef::new(Trays::WellRelativeDiameter).decimal())
            .col(
                ColumnDef::new(Trays::LastUpdated)
                    .timestamp_with_time_zone()
                    .not_null()
                    .default(Expr::current_timestamp()),
            )
            .col(
                ColumnDef::new(Trays::CreatedAt)
                    .timestamp_with_time_zone()
                    .not_null()
                    .default(Expr::current_timestamp()),
            )
            .to_owned();

        // Add ID column with appropriate type and default based on database backend
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

        // Add foreign key constraint for SQLite
        if manager.get_database_backend() == sea_orm::DatabaseBackend::Sqlite {
            wells_table.foreign_key(
                &mut ForeignKey::create()
                    .name("wells_tray_id_fkey")
                    .from(Wells::Table, Wells::TrayId)
                    .to(Trays::Table, Trays::Id)
                    .on_delete(ForeignKeyAction::NoAction)
                    .on_update(ForeignKeyAction::NoAction),
            );
        }

        manager.create_table(wells_table).await?;

        // Create temperature_probes table
        let mut temperature_probes_table = Table::create()
            .table(TemperatureProbes::Table)
            .if_not_exists()
            .col(
                ColumnDef::new(TemperatureProbes::ExperimentId)
                    .uuid()
                    .not_null(),
            )
            .col(ColumnDef::new(TemperatureProbes::ProbeName).text())
            .col(ColumnDef::new(TemperatureProbes::ColumnIndex).integer())
            .col(
                ColumnDef::new(TemperatureProbes::CreatedAt)
                    .timestamp_with_time_zone()
                    .not_null()
                    .default(Expr::current_timestamp()),
            )
            .col(
                ColumnDef::new(TemperatureProbes::LastUpdated)
                    .timestamp_with_time_zone()
                    .not_null()
                    .default(Expr::current_timestamp()),
            )
            .col(ColumnDef::new(TemperatureProbes::CorrectionFactor).decimal())
            .to_owned();

        // Add ID column with appropriate type and default based on database backend
        match manager.get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => {
                temperature_probes_table.col(
                    ColumnDef::new(TemperatureProbes::Id)
                        .uuid()
                        .not_null()
                        .primary_key()
                        .default(Expr::cust("uuid_generate_v4()")),
                );
            }
            sea_orm::DatabaseBackend::Sqlite => {
                temperature_probes_table.col(
                    ColumnDef::new(TemperatureProbes::Id)
                        .uuid()
                        .not_null()
                        .primary_key(),
                );
            }
            _ => {
                return Err(DbErr::Custom("Unsupported database backend".to_string()));
            }
        }

        // Add foreign key constraint for SQLite
        if manager.get_database_backend() == sea_orm::DatabaseBackend::Sqlite {
            temperature_probes_table.foreign_key(
                &mut ForeignKey::create()
                    .name("temperature_probes_experiment_id_fkey")
                    .from(TemperatureProbes::Table, TemperatureProbes::ExperimentId)
                    .to(Experiments::Table, Experiments::Id)
                    .on_delete(ForeignKeyAction::NoAction)
                    .on_update(ForeignKeyAction::NoAction),
            );
        }

        manager.create_table(temperature_probes_table).await?;

        // Create well_temperatures table
        let mut well_temperatures_table = Table::create()
            .table(WellTemperatures::Table)
            .if_not_exists()
            .col(ColumnDef::new(WellTemperatures::WellId).uuid().not_null())
            .col(ColumnDef::new(WellTemperatures::Timestamp).timestamp_with_time_zone())
            .col(ColumnDef::new(WellTemperatures::TemperatureCelsius).decimal())
            .col(
                ColumnDef::new(WellTemperatures::CreatedAt)
                    .timestamp_with_time_zone()
                    .not_null()
                    .default(Expr::current_timestamp()),
            )
            .col(
                ColumnDef::new(WellTemperatures::LastUpdated)
                    .timestamp_with_time_zone()
                    .not_null()
                    .default(Expr::current_timestamp()),
            )
            .to_owned();

        // Add ID column with appropriate type and default based on database backend
        match manager.get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => {
                well_temperatures_table.col(
                    ColumnDef::new(WellTemperatures::Id)
                        .uuid()
                        .not_null()
                        .primary_key()
                        .default(Expr::cust("uuid_generate_v4()")),
                );
            }
            sea_orm::DatabaseBackend::Sqlite => {
                well_temperatures_table.col(
                    ColumnDef::new(WellTemperatures::Id)
                        .uuid()
                        .not_null()
                        .primary_key(),
                );
            }
            _ => {
                return Err(DbErr::Custom("Unsupported database backend".to_string()));
            }
        }

        // Add foreign key constraint for SQLite
        if manager.get_database_backend() == sea_orm::DatabaseBackend::Sqlite {
            well_temperatures_table.foreign_key(
                &mut ForeignKey::create()
                    .name("well_temperatures_well_id_fkey")
                    .from(WellTemperatures::Table, WellTemperatures::WellId)
                    .to(Wells::Table, Wells::Id)
                    .on_delete(ForeignKeyAction::NoAction)
                    .on_update(ForeignKeyAction::NoAction),
            );
        }

        manager.create_table(well_temperatures_table).await?;

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
                samples_table.col(ColumnDef::new(Samples::Type).text().not_null().check(
                    Expr::col(Samples::Type).is_in([
                        "bulk",
                        "filter",
                        "procedural_blank",
                        "pure_water",
                    ]),
                ));
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
                treatments_table.col(
                    ColumnDef::new(Treatments::Name)
                        .text()
                        .not_null()
                        .check(Expr::col(Treatments::Name).is_in(["none", "heat", "h2o2"])),
                );
            }
            _ => {
                return Err(DbErr::Custom("Unsupported database backend".to_string()));
            }
        }

        // Add foreign key constraint for SQLite
        if manager.get_database_backend() == sea_orm::DatabaseBackend::Sqlite {
            treatments_table.foreign_key(
                &mut ForeignKey::create()
                    .name("sample_treatments")
                    .from(Treatments::Table, Treatments::SampleId)
                    .to(Samples::Table, Samples::Id)
                    .on_delete(ForeignKeyAction::NoAction)
                    .on_update(ForeignKeyAction::NoAction),
            );
        }

        manager.create_table(treatments_table).await?;

        // Create regions table
        let mut regions_table = Table::create()
            .table(Regions::Table)
            .if_not_exists()
            .col(ColumnDef::new(Regions::ExperimentId).uuid().not_null())
            .col(ColumnDef::new(Regions::TreatmentId).uuid())
            .col(ColumnDef::new(Regions::Name).text())
            .col(ColumnDef::new(Regions::DisplayColourHex).text())
            .col(ColumnDef::new(Regions::TrayId).small_integer())
            .col(ColumnDef::new(Regions::ColMin).small_integer())
            .col(ColumnDef::new(Regions::RowMin).small_integer())
            .col(ColumnDef::new(Regions::ColMax).small_integer())
            .col(ColumnDef::new(Regions::RowMax).small_integer())
            .col(ColumnDef::new(Regions::DilutionFactor).small_integer())
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

        // Add foreign key constraints for SQLite
        if manager.get_database_backend() == sea_orm::DatabaseBackend::Sqlite {
            regions_table.foreign_key(
                &mut ForeignKey::create()
                    .name("regions_experiment_id_fkey")
                    .from(Regions::Table, Regions::ExperimentId)
                    .to(Experiments::Table, Experiments::Id)
                    .on_delete(ForeignKeyAction::NoAction)
                    .on_update(ForeignKeyAction::NoAction),
            );
            regions_table.foreign_key(
                &mut ForeignKey::create()
                    .name("regions_treatment_id_fkey")
                    .from(Regions::Table, Regions::TreatmentId)
                    .to(Treatments::Table, Treatments::Id)
                    .on_delete(ForeignKeyAction::NoAction)
                    .on_update(ForeignKeyAction::NoAction),
            );
        }

        manager.create_table(regions_table).await?;

        // Create freezing_results table
        let mut freezing_results_table = Table::create()
            .table(FreezingResults::Table)
            .if_not_exists()
            .col(ColumnDef::new(FreezingResults::WellId).uuid().not_null())
            .col(ColumnDef::new(FreezingResults::FreezingTemperatureCelsius).decimal())
            .col(ColumnDef::new(FreezingResults::IsFrozen).boolean())
            .col(ColumnDef::new(FreezingResults::RegionId).uuid())
            .col(
                ColumnDef::new(FreezingResults::CreatedAt)
                    .timestamp_with_time_zone()
                    .not_null()
                    .default(Expr::current_timestamp()),
            )
            .col(
                ColumnDef::new(FreezingResults::LastUpdated)
                    .timestamp_with_time_zone()
                    .not_null()
                    .default(Expr::current_timestamp()),
            )
            .to_owned();

        // Add ID column with appropriate type and default based on database backend
        match manager.get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => {
                freezing_results_table.col(
                    ColumnDef::new(FreezingResults::Id)
                        .uuid()
                        .not_null()
                        .primary_key()
                        .default(Expr::cust("uuid_generate_v4()")),
                );
            }
            sea_orm::DatabaseBackend::Sqlite => {
                freezing_results_table.col(
                    ColumnDef::new(FreezingResults::Id)
                        .uuid()
                        .not_null()
                        .primary_key(),
                );
            }
            _ => {
                return Err(DbErr::Custom("Unsupported database backend".to_string()));
            }
        }

        // Add foreign key constraints for SQLite
        if manager.get_database_backend() == sea_orm::DatabaseBackend::Sqlite {
            freezing_results_table.foreign_key(
                &mut ForeignKey::create()
                    .name("freezing_results_well_id_fkey")
                    .from(FreezingResults::Table, FreezingResults::WellId)
                    .to(Wells::Table, Wells::Id)
                    .on_delete(ForeignKeyAction::NoAction)
                    .on_update(ForeignKeyAction::NoAction),
            );
            freezing_results_table.foreign_key(
                &mut ForeignKey::create()
                    .name("freezing_results_region_id_fkey")
                    .from(FreezingResults::Table, FreezingResults::RegionId)
                    .to(Regions::Table, Regions::Id)
                    .on_delete(ForeignKeyAction::NoAction)
                    .on_update(ForeignKeyAction::NoAction),
            );
        }

        manager.create_table(freezing_results_table).await?;

        // Create inp_concentrations table
        let mut inp_concentrations_table = Table::create()
            .table(InpConcentrations::Table)
            .if_not_exists()
            .col(
                ColumnDef::new(InpConcentrations::RegionId)
                    .uuid()
                    .not_null(),
            )
            .col(ColumnDef::new(InpConcentrations::TemperatureCelsius).decimal())
            .col(ColumnDef::new(InpConcentrations::NmValue).decimal())
            .col(ColumnDef::new(InpConcentrations::Error).decimal())
            .col(
                ColumnDef::new(InpConcentrations::CreatedAt)
                    .timestamp_with_time_zone()
                    .not_null()
                    .default(Expr::current_timestamp()),
            )
            .col(
                ColumnDef::new(InpConcentrations::LastUpdated)
                    .timestamp_with_time_zone()
                    .not_null()
                    .default(Expr::current_timestamp()),
            )
            .to_owned();

        // Add ID column with appropriate type and default based on database backend
        match manager.get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => {
                inp_concentrations_table.col(
                    ColumnDef::new(InpConcentrations::Id)
                        .uuid()
                        .not_null()
                        .primary_key()
                        .default(Expr::cust("uuid_generate_v4()")),
                );
            }
            sea_orm::DatabaseBackend::Sqlite => {
                inp_concentrations_table.col(
                    ColumnDef::new(InpConcentrations::Id)
                        .uuid()
                        .not_null()
                        .primary_key(),
                );
            }
            _ => {
                return Err(DbErr::Custom("Unsupported database backend".to_string()));
            }
        }

        // Add foreign key constraint for SQLite
        if manager.get_database_backend() == sea_orm::DatabaseBackend::Sqlite {
            inp_concentrations_table.foreign_key(
                &mut ForeignKey::create()
                    .name("inp_concentrations_region_id_fkey")
                    .from(InpConcentrations::Table, InpConcentrations::RegionId)
                    .to(Regions::Table, Regions::Id)
                    .on_delete(ForeignKeyAction::NoAction)
                    .on_update(ForeignKeyAction::NoAction),
            );
        }

        manager.create_table(inp_concentrations_table).await?;

        // Create s3_assets table
        let mut s3_assets_table = Table::create()
            .table(S3Assets::Table)
            .if_not_exists()
            .col(ColumnDef::new(S3Assets::ExperimentId).uuid())
            .col(ColumnDef::new(S3Assets::OriginalFilename).text().not_null())
            .col(ColumnDef::new(S3Assets::S3Key).text().not_null())
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

        // Add foreign key constraint for SQLite
        if manager.get_database_backend() == sea_orm::DatabaseBackend::Sqlite {
            s3_assets_table.foreign_key(
                &mut ForeignKey::create()
                    .name("s3_assets_experiment_id_fkey")
                    .from(S3Assets::Table, S3Assets::ExperimentId)
                    .to(Experiments::Table, Experiments::Id)
                    .on_delete(ForeignKeyAction::NoAction)
                    .on_update(ForeignKeyAction::NoAction),
            );
        }

        manager.create_table(s3_assets_table).await?;

        // Create tray_configuration_assignments table
        let mut tray_config_assignments_table = Table::create()
            .table(TrayConfigurationAssignments::Table)
            .if_not_exists()
            .col(
                ColumnDef::new(TrayConfigurationAssignments::TrayId)
                    .uuid()
                    .not_null(),
            )
            .col(
                ColumnDef::new(TrayConfigurationAssignments::TrayConfigurationId)
                    .uuid()
                    .not_null(),
            )
            .col(
                ColumnDef::new(TrayConfigurationAssignments::OrderSequence)
                    .small_integer()
                    .not_null(),
            )
            .col(
                ColumnDef::new(TrayConfigurationAssignments::RotationDegrees)
                    .small_integer()
                    .not_null(),
            )
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
                    .col(TrayConfigurationAssignments::TrayId)
                    .col(TrayConfigurationAssignments::TrayConfigurationId)
                    .col(TrayConfigurationAssignments::OrderSequence),
            )
            .to_owned();

        // Add foreign key constraints for SQLite
        if manager.get_database_backend() == sea_orm::DatabaseBackend::Sqlite {
            tray_config_assignments_table.foreign_key(
                &mut ForeignKey::create()
                    .name("fk_tray_assignments_to_tray")
                    .from(
                        TrayConfigurationAssignments::Table,
                        TrayConfigurationAssignments::TrayId,
                    )
                    .to(Trays::Table, Trays::Id)
                    .on_delete(ForeignKeyAction::NoAction)
                    .on_update(ForeignKeyAction::NoAction),
            );
            tray_config_assignments_table.foreign_key(
                &mut ForeignKey::create()
                    .name("fk_tray_assignment_to_configuration")
                    .from(
                        TrayConfigurationAssignments::Table,
                        TrayConfigurationAssignments::TrayConfigurationId,
                    )
                    .to(TrayConfigurations::Table, TrayConfigurations::Id)
                    .on_delete(ForeignKeyAction::NoAction)
                    .on_update(ForeignKeyAction::NoAction),
            );
        }

        manager.create_table(tray_config_assignments_table).await?;

        // Create unique constraints and indexes
        self.create_unique_constraints(manager).await?;
        self.create_indexes(manager).await?;

        // Only create foreign keys for PostgreSQL (SQLite foreign keys are already defined inline)
        if manager.get_database_backend() == sea_orm::DatabaseBackend::Postgres {
            self.create_foreign_keys(manager).await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop all foreign keys first
        self.drop_foreign_keys(manager).await?;

        // Drop all tables in reverse order
        manager
            .drop_table(
                Table::drop()
                    .table(TrayConfigurationAssignments::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(S3Assets::Table).if_exists().to_owned())
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(FreezingResults::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(Regions::Table).if_exists().to_owned())
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
            .drop_table(
                Table::drop()
                    .table(WellTemperatures::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(TemperatureProbes::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(Wells::Table).if_exists().to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Trays::Table).if_exists().to_owned())
            .await?;
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
            .drop_table(Table::drop().table(Locations::Table).if_exists().to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Projects::Table).if_exists().to_owned())
            .await?;

        // Drop custom types for PostgreSQL
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

impl Migration {
    async fn create_unique_constraints<'a>(
        &self,
        manager: &SchemaManager<'a>,
    ) -> Result<(), DbErr> {
        // Projects name uniqueness
        manager
            .create_index(
                Index::create()
                    .name("projects_name_key")
                    .table(Projects::Table)
                    .col(Projects::Name)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Locations name uniqueness
        manager
            .create_index(
                Index::create()
                    .name("locations_name_key")
                    .table(Locations::Table)
                    .col(Locations::Name)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Experiments name uniqueness
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

        // Tray configurations name uniqueness
        manager
            .create_index(
                Index::create()
                    .name("name_uniqueness")
                    .table(TrayConfigurations::Table)
                    .col(TrayConfigurations::Name)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // S3 assets s3_key uniqueness
        manager
            .create_index(
                Index::create()
                    .name("s3_assets_s3_key_key")
                    .table(S3Assets::Table)
                    .col(S3Assets::S3Key)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Tray configuration assignments sequence uniqueness
        manager
            .create_index(
                Index::create()
                    .name("no_duplicate_sequences")
                    .table(TrayConfigurationAssignments::Table)
                    .col(TrayConfigurationAssignments::TrayConfigurationId)
                    .col(TrayConfigurationAssignments::OrderSequence)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn create_indexes<'a>(&self, manager: &SchemaManager<'a>) -> Result<(), DbErr> {
        // Create indexes for each table individually to avoid type issues

        // Projects indexes
        manager
            .create_index(
                Index::create()
                    .name("idx_projects_id")
                    .table(Projects::Table)
                    .col(Projects::Id)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_projects_name")
                    .table(Projects::Table)
                    .col(Projects::Name)
                    .to_owned(),
            )
            .await?;

        // Locations indexes
        manager
            .create_index(
                Index::create()
                    .name("idx_locations_id")
                    .table(Locations::Table)
                    .col(Locations::Id)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_locations_name")
                    .table(Locations::Table)
                    .col(Locations::Name)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_locations_project_id")
                    .table(Locations::Table)
                    .col(Locations::ProjectId)
                    .to_owned(),
            )
            .await?;

        // Experiments indexes
        manager
            .create_index(
                Index::create()
                    .name("idx_experiment_id")
                    .table(Experiments::Table)
                    .col(Experiments::Id)
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

        // Wells indexes
        manager
            .create_index(
                Index::create()
                    .name("idx_wells_tray_id")
                    .table(Wells::Table)
                    .col(Wells::TrayId)
                    .to_owned(),
            )
            .await?;

        // Temperature probes indexes
        manager
            .create_index(
                Index::create()
                    .name("idx_temperature_probes_experiment_id")
                    .table(TemperatureProbes::Table)
                    .col(TemperatureProbes::ExperimentId)
                    .to_owned(),
            )
            .await?;

        // Well temperatures indexes
        manager
            .create_index(
                Index::create()
                    .name("idx_well_temperatures_well_id")
                    .table(WellTemperatures::Table)
                    .col(WellTemperatures::WellId)
                    .to_owned(),
            )
            .await?;

        // Samples indexes
        manager
            .create_index(
                Index::create()
                    .name("idx_samples_location_id")
                    .table(Samples::Table)
                    .col(Samples::LocationId)
                    .to_owned(),
            )
            .await?;

        // Regions indexes
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

        // Freezing results indexes
        manager
            .create_index(
                Index::create()
                    .name("idx_freezing_results_well_id")
                    .table(FreezingResults::Table)
                    .col(FreezingResults::WellId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_freezing_results_region_id")
                    .table(FreezingResults::Table)
                    .col(FreezingResults::RegionId)
                    .to_owned(),
            )
            .await?;

        // INP concentrations indexes
        manager
            .create_index(
                Index::create()
                    .name("idx_inp_concentrations_region_id")
                    .table(InpConcentrations::Table)
                    .col(InpConcentrations::RegionId)
                    .to_owned(),
            )
            .await?;

        // S3 assets indexes
        manager
            .create_index(
                Index::create()
                    .name("idx_s3_asset_id")
                    .table(S3Assets::Table)
                    .col(S3Assets::Id)
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

        // Tray configuration assignments indexes
        manager
            .create_index(
                Index::create()
                    .name("idx_tray_configuration_assignments_tray_id")
                    .table(TrayConfigurationAssignments::Table)
                    .col(TrayConfigurationAssignments::TrayId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_tray_configuration_assignments_tray_configuration_id")
                    .table(TrayConfigurationAssignments::Table)
                    .col(TrayConfigurationAssignments::TrayConfigurationId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn create_foreign_keys<'a>(&self, manager: &SchemaManager<'a>) -> Result<(), DbErr> {
        // Locations -> Projects
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_locations_project_id")
                    .from(Locations::Table, Locations::ProjectId)
                    .to(Projects::Table, Projects::Id)
                    .on_delete(ForeignKeyAction::SetNull)
                    .on_update(ForeignKeyAction::NoAction)
                    .to_owned(),
            )
            .await?;

        // Experiments -> TrayConfigurations
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_experiment_tray_configuration")
                    .from(Experiments::Table, Experiments::TrayConfigurationId)
                    .to(TrayConfigurations::Table, TrayConfigurations::Id)
                    .on_delete(ForeignKeyAction::NoAction)
                    .on_update(ForeignKeyAction::NoAction)
                    .to_owned(),
            )
            .await?;

        // Wells -> Trays
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("wells_tray_id_fkey")
                    .from(Wells::Table, Wells::TrayId)
                    .to(Trays::Table, Trays::Id)
                    .on_delete(ForeignKeyAction::NoAction)
                    .on_update(ForeignKeyAction::NoAction)
                    .to_owned(),
            )
            .await?;

        // TemperatureProbes -> Experiments
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("temperature_probes_experiment_id_fkey")
                    .from(TemperatureProbes::Table, TemperatureProbes::ExperimentId)
                    .to(Experiments::Table, Experiments::Id)
                    .on_delete(ForeignKeyAction::NoAction)
                    .on_update(ForeignKeyAction::NoAction)
                    .to_owned(),
            )
            .await?;

        // WellTemperatures -> Wells
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("well_temperatures_well_id_fkey")
                    .from(WellTemperatures::Table, WellTemperatures::WellId)
                    .to(Wells::Table, Wells::Id)
                    .on_delete(ForeignKeyAction::NoAction)
                    .on_update(ForeignKeyAction::NoAction)
                    .to_owned(),
            )
            .await?;

        // Treatments -> Samples
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("sample_treatments")
                    .from(Treatments::Table, Treatments::SampleId)
                    .to(Samples::Table, Samples::Id)
                    .on_delete(ForeignKeyAction::NoAction)
                    .on_update(ForeignKeyAction::NoAction)
                    .to_owned(),
            )
            .await?;

        // Regions -> Experiments
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("regions_experiment_id_fkey")
                    .from(Regions::Table, Regions::ExperimentId)
                    .to(Experiments::Table, Experiments::Id)
                    .on_delete(ForeignKeyAction::NoAction)
                    .on_update(ForeignKeyAction::NoAction)
                    .to_owned(),
            )
            .await?;

        // Regions -> Treatments
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("regions_treatment_id_fkey")
                    .from(Regions::Table, Regions::TreatmentId)
                    .to(Treatments::Table, Treatments::Id)
                    .on_delete(ForeignKeyAction::NoAction)
                    .on_update(ForeignKeyAction::NoAction)
                    .to_owned(),
            )
            .await?;

        // FreezingResults -> Wells
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("freezing_results_well_id_fkey")
                    .from(FreezingResults::Table, FreezingResults::WellId)
                    .to(Wells::Table, Wells::Id)
                    .on_delete(ForeignKeyAction::NoAction)
                    .on_update(ForeignKeyAction::NoAction)
                    .to_owned(),
            )
            .await?;

        // FreezingResults -> Regions
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("freezing_results_region_id_fkey")
                    .from(FreezingResults::Table, FreezingResults::RegionId)
                    .to(Regions::Table, Regions::Id)
                    .on_delete(ForeignKeyAction::NoAction)
                    .on_update(ForeignKeyAction::NoAction)
                    .to_owned(),
            )
            .await?;

        // InpConcentrations -> Regions
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("inp_concentrations_region_id_fkey")
                    .from(InpConcentrations::Table, InpConcentrations::RegionId)
                    .to(Regions::Table, Regions::Id)
                    .on_delete(ForeignKeyAction::NoAction)
                    .on_update(ForeignKeyAction::NoAction)
                    .to_owned(),
            )
            .await?;

        // S3Assets -> Experiments
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("s3_assets_experiment_id_fkey")
                    .from(S3Assets::Table, S3Assets::ExperimentId)
                    .to(Experiments::Table, Experiments::Id)
                    .on_delete(ForeignKeyAction::NoAction)
                    .on_update(ForeignKeyAction::NoAction)
                    .to_owned(),
            )
            .await?;

        // Samples -> Locations
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("samples_location_id_fkey")
                    .from(Samples::Table, Samples::LocationId)
                    .to(Locations::Table, Locations::Id)
                    .on_delete(ForeignKeyAction::NoAction)
                    .on_update(ForeignKeyAction::NoAction)
                    .to_owned(),
            )
            .await?;

        // TrayConfigurationAssignments -> Trays
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_tray_assignments_to_tray")
                    .from(
                        TrayConfigurationAssignments::Table,
                        TrayConfigurationAssignments::TrayId,
                    )
                    .to(Trays::Table, Trays::Id)
                    .on_delete(ForeignKeyAction::NoAction)
                    .on_update(ForeignKeyAction::NoAction)
                    .to_owned(),
            )
            .await?;

        // TrayConfigurationAssignments -> TrayConfigurations
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_tray_assignment_to_configuration")
                    .from(
                        TrayConfigurationAssignments::Table,
                        TrayConfigurationAssignments::TrayConfigurationId,
                    )
                    .to(TrayConfigurations::Table, TrayConfigurations::Id)
                    .on_delete(ForeignKeyAction::NoAction)
                    .on_update(ForeignKeyAction::NoAction)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn drop_foreign_keys<'a>(&self, manager: &SchemaManager<'a>) -> Result<(), DbErr> {
        // Drop foreign keys individually to avoid compilation issues
        let _ = manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_tray_assignment_to_configuration")
                    .table(TrayConfigurationAssignments::Table)
                    .to_owned(),
            )
            .await;
        let _ = manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_tray_assignments_to_tray")
                    .table(TrayConfigurationAssignments::Table)
                    .to_owned(),
            )
            .await;
        let _ = manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("samples_location_id_fkey")
                    .table(Samples::Table)
                    .to_owned(),
            )
            .await;
        let _ = manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("s3_assets_experiment_id_fkey")
                    .table(S3Assets::Table)
                    .to_owned(),
            )
            .await;
        let _ = manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("inp_concentrations_region_id_fkey")
                    .table(InpConcentrations::Table)
                    .to_owned(),
            )
            .await;
        let _ = manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("freezing_results_region_id_fkey")
                    .table(FreezingResults::Table)
                    .to_owned(),
            )
            .await;
        let _ = manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("freezing_results_well_id_fkey")
                    .table(FreezingResults::Table)
                    .to_owned(),
            )
            .await;
        let _ = manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("regions_treatment_id_fkey")
                    .table(Regions::Table)
                    .to_owned(),
            )
            .await;
        let _ = manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("regions_experiment_id_fkey")
                    .table(Regions::Table)
                    .to_owned(),
            )
            .await;
        let _ = manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("sample_treatments")
                    .table(Treatments::Table)
                    .to_owned(),
            )
            .await;
        let _ = manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("well_temperatures_well_id_fkey")
                    .table(WellTemperatures::Table)
                    .to_owned(),
            )
            .await;
        let _ = manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("temperature_probes_experiment_id_fkey")
                    .table(TemperatureProbes::Table)
                    .to_owned(),
            )
            .await;
        let _ = manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("wells_tray_id_fkey")
                    .table(Wells::Table)
                    .to_owned(),
            )
            .await;
        let _ = manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_experiment_tray_configuration")
                    .table(Experiments::Table)
                    .to_owned(),
            )
            .await;
        let _ = manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_locations_project_id")
                    .table(Locations::Table)
                    .to_owned(),
            )
            .await;

        Ok(())
    }
}

// Define all the table and column identifiers
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
    StartDate,
    EndDate,
    LastUpdated,
    CreatedAt,
    ProjectId,
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
    Name,
    QtyXAxis,
    QtyYAxis,
    WellRelativeDiameter,
    LastUpdated,
    CreatedAt,
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
enum TemperatureProbes {
    Table,
    Id,
    ExperimentId,
    ProbeName,
    ColumnIndex,
    CreatedAt,
    LastUpdated,
    CorrectionFactor,
}

#[derive(DeriveIden)]
enum WellTemperatures {
    Table,
    Id,
    WellId,
    Timestamp,
    TemperatureCelsius,
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
enum FreezingResults {
    Table,
    Id,
    WellId,
    FreezingTemperatureCelsius,
    IsFrozen,
    RegionId,
    CreatedAt,
    LastUpdated,
}

#[derive(DeriveIden)]
enum InpConcentrations {
    Table,
    Id,
    RegionId,
    TemperatureCelsius,
    NmValue,
    Error,
    CreatedAt,
    LastUpdated,
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
enum TrayConfigurationAssignments {
    Table,
    TrayId,
    TrayConfigurationId,
    OrderSequence,
    RotationDegrees,
    CreatedAt,
    LastUpdated,
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
