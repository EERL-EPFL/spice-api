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
        match manager.get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => {
                manager
                    .create_table(
                        Table::create()
                            .table(Projects::Table)
                            .if_not_exists()
                            .col(
                                ColumnDef::new(Projects::Id)
                                    .uuid()
                                    .not_null()
                                    .primary_key()
                                    .default(Expr::cust("uuid_generate_v4()")),
                            )
                            .col(ColumnDef::new(Projects::Name).string().not_null())
                            .col(ColumnDef::new(Projects::Note).text())
                            .col(ColumnDef::new(Projects::Colour).string())
                            .col(ColumnDef::new(Projects::CreatedAt).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
                            .col(ColumnDef::new(Projects::LastUpdated).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
                            .to_owned(),
                    )
                    .await?;
            }
            sea_orm::DatabaseBackend::Sqlite => {
                manager
                    .create_table(
                        Table::create()
                            .table(Projects::Table)
                            .if_not_exists()
                            .col(ColumnDef::new(Projects::Id).uuid().not_null().primary_key())
                            .col(ColumnDef::new(Projects::Name).string().not_null())
                            .col(ColumnDef::new(Projects::Note).text())
                            .col(ColumnDef::new(Projects::Colour).string())
                            .col(ColumnDef::new(Projects::CreatedAt).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
                            .col(ColumnDef::new(Projects::LastUpdated).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
                            .to_owned(),
                    )
                    .await?;
            }
            _ => {
                return Err(DbErr::Custom("Unsupported database backend".to_string()));
            }
        }

        // Create locations table
        match manager.get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => {
                manager
                    .create_table(
                        Table::create()
                            .table(Locations::Table)
                            .if_not_exists()
                            .col(
                                ColumnDef::new(Locations::Id)
                                    .uuid()
                                    .not_null()
                                    .primary_key()
                                    .default(Expr::cust("uuid_generate_v4()")),
                            )
                            .col(ColumnDef::new(Locations::Name).string().not_null())
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
                            .to_owned(),
                    )
                    .await?;
            }
            sea_orm::DatabaseBackend::Sqlite => {
                manager
                    .create_table(
                        Table::create()
                            .table(Locations::Table)
                            .if_not_exists()
                            .col(ColumnDef::new(Locations::Id).uuid().not_null().primary_key())
                            .col(ColumnDef::new(Locations::Name).string().not_null())
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
                            .to_owned(),
                    )
                    .await?;
            }
            _ => {
                return Err(DbErr::Custom("Unsupported database backend".to_string()));
            }
        }

        // Create samples table
        match manager.get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => {
                manager
                    .create_table(
                        Table::create()
                            .table(Samples::Table)
                            .if_not_exists()
                            .col(
                                ColumnDef::new(Samples::Id)
                                    .uuid()
                                    .not_null()
                                    .primary_key()
                                    .default(Expr::cust("uuid_generate_v4()")),
                            )
                            .col(ColumnDef::new(Samples::Name).text().not_null())
                            .col(
                                ColumnDef::new(Samples::Type)
                                    .custom(SampleType::Table)
                                    .not_null(),
                            )
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
                            .to_owned(),
                    )
                    .await?;
            }
            sea_orm::DatabaseBackend::Sqlite => {
                manager
                    .create_table(
                        Table::create()
                            .table(Samples::Table)
                            .if_not_exists()
                            .col(ColumnDef::new(Samples::Id).uuid().not_null().primary_key())
                            .col(ColumnDef::new(Samples::Name).text().not_null())
                            .col(ColumnDef::new(Samples::Type).text().not_null())
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
                            .to_owned(),
                    )
                    .await?;
            }
            _ => {
                return Err(DbErr::Custom("Unsupported database backend".to_string()));
            }
        }

        // Create treatments table
        match manager.get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => {
                manager
                    .create_table(
                        Table::create()
                            .table(Treatments::Table)
                            .if_not_exists()
                            .col(
                                ColumnDef::new(Treatments::Id)
                                    .uuid()
                                    .not_null()
                                    .primary_key()
                                    .default(Expr::cust("uuid_generate_v4()")),
                            )
                            .col(
                                ColumnDef::new(Treatments::Name)
                                    .custom(TreatmentName::Table)
                                    .not_null(),
                            )
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
                            .to_owned(),
                    )
                    .await?;
            }
            sea_orm::DatabaseBackend::Sqlite => {
                manager
                    .create_table(
                        Table::create()
                            .table(Treatments::Table)
                            .if_not_exists()
                            .col(ColumnDef::new(Treatments::Id).uuid().not_null().primary_key())
                            .col(ColumnDef::new(Treatments::Name).text().not_null())
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
                            .to_owned(),
                    )
                    .await?;
            }
            _ => {
                return Err(DbErr::Custom("Unsupported database backend".to_string()));
            }
        }

        // Create tray_configurations table
        match manager.get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => {
                manager
                    .create_table(
                        Table::create()
                            .table(TrayConfigurations::Table)
                            .if_not_exists()
                            .col(
                                ColumnDef::new(TrayConfigurations::Id)
                                    .uuid()
                                    .not_null()
                                    .primary_key()
                                    .default(Expr::cust("uuid_generate_v4()")),
                            )
                            .col(ColumnDef::new(TrayConfigurations::Name).text())
                            .col(ColumnDef::new(TrayConfigurations::ExperimentDefault).boolean().not_null())
                            .col(ColumnDef::new(TrayConfigurations::CreatedAt).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
                            .col(ColumnDef::new(TrayConfigurations::LastUpdated).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
                            .to_owned(),
                    )
                    .await?;
            }
            sea_orm::DatabaseBackend::Sqlite => {
                manager
                    .create_table(
                        Table::create()
                            .table(TrayConfigurations::Table)
                            .if_not_exists()
                            .col(ColumnDef::new(TrayConfigurations::Id).uuid().not_null().primary_key())
                            .col(ColumnDef::new(TrayConfigurations::Name).text())
                            .col(ColumnDef::new(TrayConfigurations::ExperimentDefault).boolean().not_null())
                            .col(ColumnDef::new(TrayConfigurations::CreatedAt).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
                            .col(ColumnDef::new(TrayConfigurations::LastUpdated).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
                            .to_owned(),
                    )
                    .await?;
            }
            _ => {
                return Err(DbErr::Custom("Unsupported database backend".to_string()));
            }
        }

        // Create experiments table
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
                            .col(ColumnDef::new(Experiments::Username).text())
                            .col(ColumnDef::new(Experiments::PerformedAt).timestamp_with_time_zone())
                            .col(ColumnDef::new(Experiments::TemperatureRamp).decimal())
                            .col(ColumnDef::new(Experiments::TemperatureStart).decimal())
                            .col(ColumnDef::new(Experiments::TemperatureEnd).decimal())
                            .col(ColumnDef::new(Experiments::IsCalibration).boolean().not_null().default(false))
                            .col(ColumnDef::new(Experiments::Remarks).text())
                            .col(ColumnDef::new(Experiments::TrayConfigurationId).uuid())
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
                            .col(ColumnDef::new(Experiments::Username).text())
                            .col(ColumnDef::new(Experiments::PerformedAt).timestamp_with_time_zone())
                            .col(ColumnDef::new(Experiments::TemperatureRamp).decimal())
                            .col(ColumnDef::new(Experiments::TemperatureStart).decimal())
                            .col(ColumnDef::new(Experiments::TemperatureEnd).decimal())
                            .col(ColumnDef::new(Experiments::IsCalibration).boolean().not_null().default(false))
                            .col(ColumnDef::new(Experiments::Remarks).text())
                            .col(ColumnDef::new(Experiments::TrayConfigurationId).uuid())
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
                            .to_owned(),
                    )
                    .await?;
            }
            _ => {
                return Err(DbErr::Custom("Unsupported database backend".to_string()));
            }
        }

        // Create unique indexes
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

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop tables in reverse dependency order
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