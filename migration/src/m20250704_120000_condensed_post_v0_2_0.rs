use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Remove unused date columns from locations table
        manager
            .alter_table(
                Table::alter()
                    .table(Locations::Table)
                    .drop_column(Locations::StartDate)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Locations::Table)
                    .drop_column(Locations::EndDate)
                    .to_owned(),
            )
            .await?;

        // Create temperature_probe_configurations table
        manager
            .create_table(
                Table::create()
                    .table(TemperatureProbeConfigurations::Table)
                    .if_not_exists()
                    .col(pk_uuid(TemperatureProbeConfigurations::Id))
                    .col(uuid(TemperatureProbeConfigurations::ExperimentId))
                    .col(integer(TemperatureProbeConfigurations::ProbeNumber))
                    .col(integer(TemperatureProbeConfigurations::ColumnIndex))
                    .col(decimal(TemperatureProbeConfigurations::PositionX))
                    .col(decimal(TemperatureProbeConfigurations::PositionY))
                    .col(
                        ColumnDef::new(TemperatureProbeConfigurations::CorrectionSlope)
                            .decimal_len(16, 10)
                            .not_null()
                            .default(1.0),
                    )
                    .col(
                        ColumnDef::new(TemperatureProbeConfigurations::CorrectionIntercept)
                            .decimal_len(16, 10)
                            .not_null()
                            .default(0.0),
                    )
                    .col(
                        ColumnDef::new(TemperatureProbeConfigurations::InterpolationMethod)
                            .string()
                            .default("rbf"),
                    )
                    .col(
                        ColumnDef::new(TemperatureProbeConfigurations::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(TemperatureProbeConfigurations::LastUpdated)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_temp_probe_config_experiment")
                            .from(
                                TemperatureProbeConfigurations::Table,
                                TemperatureProbeConfigurations::ExperimentId,
                            )
                            .to(Experiments::Table, Experiments::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create phase_change_events table
        manager
            .create_table(
                Table::create()
                    .table(PhaseChangeEvents::Table)
                    .if_not_exists()
                    .col(pk_uuid(PhaseChangeEvents::Id))
                    .col(uuid(PhaseChangeEvents::ExperimentId))
                    .col(uuid(PhaseChangeEvents::TrayId))
                    .col(integer(PhaseChangeEvents::TraySequence))
                    .col(integer(PhaseChangeEvents::WellRow))
                    .col(integer(PhaseChangeEvents::WellColumn))
                    .col(string(PhaseChangeEvents::WellCoordinate))
                    .col(integer(PhaseChangeEvents::PhaseState))
                    .col(integer_null(PhaseChangeEvents::PreviousState))
                    .col(timestamp_with_time_zone(PhaseChangeEvents::Timestamp))
                    .col(uuid_null(PhaseChangeEvents::RegionId))
                    .col(uuid_null(PhaseChangeEvents::AssetId))
                    .col(
                        ColumnDef::new(PhaseChangeEvents::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(PhaseChangeEvents::LastUpdated)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_phase_change_experiment")
                            .from(PhaseChangeEvents::Table, PhaseChangeEvents::ExperimentId)
                            .to(Experiments::Table, Experiments::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_phase_change_tray")
                            .from(PhaseChangeEvents::Table, PhaseChangeEvents::TrayId)
                            .to(Trays::Table, Trays::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_phase_change_region")
                            .from(PhaseChangeEvents::Table, PhaseChangeEvents::RegionId)
                            .to(Regions::Table, Regions::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_phase_change_asset")
                            .from(PhaseChangeEvents::Table, PhaseChangeEvents::AssetId)
                            .to(S3Assets::Table, S3Assets::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        // Create phase_change_temperatures table
        manager
            .create_table(
                Table::create()
                    .table(PhaseChangeTemperatures::Table)
                    .if_not_exists()
                    .col(pk_uuid(PhaseChangeTemperatures::Id))
                    .col(uuid(PhaseChangeTemperatures::PhaseChangeEventId))
                    .col(integer(PhaseChangeTemperatures::ProbeNumber))
                    .col(integer(PhaseChangeTemperatures::ProbeColumnIndex))
                    .col(decimal(PhaseChangeTemperatures::TemperatureCelsius))
                    .col(decimal_null(PhaseChangeTemperatures::ProbePositionX))
                    .col(decimal_null(PhaseChangeTemperatures::ProbePositionY))
                    .col(decimal_null(PhaseChangeTemperatures::CorrectionSlope))
                    .col(decimal_null(PhaseChangeTemperatures::CorrectionIntercept))
                    .col(decimal_null(PhaseChangeTemperatures::CorrectedTemperature))
                    .col(
                        ColumnDef::new(PhaseChangeTemperatures::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_temp_phase_change_event")
                            .from(
                                PhaseChangeTemperatures::Table,
                                PhaseChangeTemperatures::PhaseChangeEventId,
                            )
                            .to(PhaseChangeEvents::Table, PhaseChangeEvents::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create time_points table
        manager
            .create_table(
                Table::create()
                    .table(TimePoints::Table)
                    .if_not_exists()
                    .col(pk_uuid(TimePoints::Id))
                    .col(uuid(TimePoints::ExperimentId))
                    .col(timestamp_with_time_zone(TimePoints::Timestamp))
                    .col(string_null(TimePoints::ImageFilename))
                    .col(uuid_null(TimePoints::AssetId))
                    .col(
                        ColumnDef::new(TimePoints::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_time_points_experiment")
                            .from(TimePoints::Table, TimePoints::ExperimentId)
                            .to(Experiments::Table, Experiments::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_time_points_asset")
                            .from(TimePoints::Table, TimePoints::AssetId)
                            .to(S3Assets::Table, S3Assets::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        // Create time_point_temperatures table (with correct integer type from start)
        manager
            .create_table(
                Table::create()
                    .table(TimePointTemperatures::Table)
                    .if_not_exists()
                    .col(pk_uuid(TimePointTemperatures::Id))
                    .col(uuid(TimePointTemperatures::TimePointId))
                    .col(integer(TimePointTemperatures::ProbeSequence)) // Using integer, not small_integer
                    .col(decimal(TimePointTemperatures::Temperature))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_time_point_temps_time_point")
                            .from(
                                TimePointTemperatures::Table,
                                TimePointTemperatures::TimePointId,
                            )
                            .to(TimePoints::Table, TimePoints::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create time_point_well_states table
        manager
            .create_table(
                Table::create()
                    .table(TimePointWellStates::Table)
                    .if_not_exists()
                    .col(pk_uuid(TimePointWellStates::Id))
                    .col(uuid(TimePointWellStates::TimePointId))
                    .col(integer(TimePointWellStates::Row))
                    .col(integer(TimePointWellStates::Col))
                    .col(integer(TimePointWellStates::Value))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_time_point_wells_time_point")
                            .from(TimePointWellStates::Table, TimePointWellStates::TimePointId)
                            .to(TimePoints::Table, TimePoints::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create temperature_readings table - optimized approach
        manager
            .create_table(
                Table::create()
                    .table(TemperatureReadings::Table)
                    .if_not_exists()
                    .col(pk_uuid(TemperatureReadings::Id))
                    .col(uuid(TemperatureReadings::ExperimentId))
                    .col(timestamp_with_time_zone(TemperatureReadings::Timestamp))
                    .col(text_null(TemperatureReadings::ImageFilename))
                    .col(decimal_null(TemperatureReadings::Probe1))
                    .col(decimal_null(TemperatureReadings::Probe2))
                    .col(decimal_null(TemperatureReadings::Probe3))
                    .col(decimal_null(TemperatureReadings::Probe4))
                    .col(decimal_null(TemperatureReadings::Probe5))
                    .col(decimal_null(TemperatureReadings::Probe6))
                    .col(decimal_null(TemperatureReadings::Probe7))
                    .col(decimal_null(TemperatureReadings::Probe8))
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
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create well_phase_transitions table - stores ONLY state changes
        manager
            .create_table(
                Table::create()
                    .table(WellPhaseTransitions::Table)
                    .if_not_exists()
                    .col(pk_uuid(WellPhaseTransitions::Id))
                    .col(uuid(WellPhaseTransitions::WellId))
                    .col(uuid(WellPhaseTransitions::ExperimentId))
                    .col(uuid(WellPhaseTransitions::TemperatureReadingId))
                    .col(timestamp_with_time_zone(WellPhaseTransitions::Timestamp))
                    .col(integer(WellPhaseTransitions::PreviousState))
                    .col(integer(WellPhaseTransitions::NewState))
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
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_well_phase_transitions_experiment")
                            .from(
                                WellPhaseTransitions::Table,
                                WellPhaseTransitions::ExperimentId,
                            )
                            .to(Experiments::Table, Experiments::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_well_phase_transitions_temperature_reading")
                            .from(
                                WellPhaseTransitions::Table,
                                WellPhaseTransitions::TemperatureReadingId,
                            )
                            .to(TemperatureReadings::Table, TemperatureReadings::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create all performance indexes
        // Phase change indexes
        manager
            .create_index(
                Index::create()
                    .name("idx_phase_change_events_experiment_timestamp")
                    .table(PhaseChangeEvents::Table)
                    .col(PhaseChangeEvents::ExperimentId)
                    .col(PhaseChangeEvents::Timestamp)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_phase_change_events_tray_well")
                    .table(PhaseChangeEvents::Table)
                    .col(PhaseChangeEvents::TrayId)
                    .col(PhaseChangeEvents::WellRow)
                    .col(PhaseChangeEvents::WellColumn)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_temp_probe_config_experiment")
                    .table(TemperatureProbeConfigurations::Table)
                    .col(TemperatureProbeConfigurations::ExperimentId)
                    .col(TemperatureProbeConfigurations::ProbeNumber)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_phase_change_temperatures_event_id")
                    .table(PhaseChangeTemperatures::Table)
                    .col(PhaseChangeTemperatures::PhaseChangeEventId)
                    .to_owned(),
            )
            .await?;

        // Time points indexes
        manager
            .create_index(
                Index::create()
                    .name("idx_time_points_experiment_timestamp")
                    .table(TimePoints::Table)
                    .col(TimePoints::ExperimentId)
                    .col(TimePoints::Timestamp)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_time_point_temps_time_point")
                    .table(TimePointTemperatures::Table)
                    .col(TimePointTemperatures::TimePointId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_time_point_wells_time_point")
                    .table(TimePointWellStates::Table)
                    .col(TimePointWellStates::TimePointId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_time_point_temperatures_composite")
                    .table(TimePointTemperatures::Table)
                    .col(TimePointTemperatures::TimePointId)
                    .col(TimePointTemperatures::ProbeSequence)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_time_point_well_states_composite")
                    .table(TimePointWellStates::Table)
                    .col(TimePointWellStates::TimePointId)
                    .col(TimePointWellStates::Row)
                    .col(TimePointWellStates::Col)
                    .to_owned(),
            )
            .await?;

        // Optimized phase transitions indexes
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
                    .name("idx_well_phase_transitions_well_timestamp")
                    .table(WellPhaseTransitions::Table)
                    .col(WellPhaseTransitions::WellId)
                    .col(WellPhaseTransitions::Timestamp)
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

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop tables in reverse dependency order
        manager
            .drop_table(Table::drop().table(WellPhaseTransitions::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(TemperatureReadings::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(TimePointWellStates::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(TimePointTemperatures::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(TimePoints::Table).to_owned())
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(PhaseChangeTemperatures::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(PhaseChangeEvents::Table).to_owned())
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(TemperatureProbeConfigurations::Table)
                    .to_owned(),
            )
            .await?;

        // Re-add location columns
        manager
            .alter_table(
                Table::alter()
                    .table(Locations::Table)
                    .add_column(ColumnDef::new(Locations::StartDate).timestamp_with_time_zone())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Locations::Table)
                    .add_column(ColumnDef::new(Locations::EndDate).timestamp_with_time_zone())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

// All table enums
#[derive(DeriveIden)]
enum Locations {
    Table,
    StartDate,
    EndDate,
}

#[derive(DeriveIden)]
enum Experiments {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Trays {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Regions {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum S3Assets {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Wells {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum TemperatureProbeConfigurations {
    Table,
    Id,
    ExperimentId,
    ProbeNumber,
    ColumnIndex,
    PositionX,
    PositionY,
    CorrectionSlope,
    CorrectionIntercept,
    InterpolationMethod,
    CreatedAt,
    LastUpdated,
}

#[derive(DeriveIden)]
enum PhaseChangeEvents {
    Table,
    Id,
    ExperimentId,
    TrayId,
    TraySequence,
    WellRow,
    WellColumn,
    WellCoordinate,
    PhaseState,
    PreviousState,
    Timestamp,
    RegionId,
    AssetId,
    CreatedAt,
    LastUpdated,
}

#[derive(DeriveIden)]
enum PhaseChangeTemperatures {
    Table,
    Id,
    PhaseChangeEventId,
    ProbeNumber,
    ProbeColumnIndex,
    TemperatureCelsius,
    ProbePositionX,
    ProbePositionY,
    CorrectionSlope,
    CorrectionIntercept,
    CorrectedTemperature,
    CreatedAt,
}

#[derive(DeriveIden)]
enum TimePoints {
    Table,
    Id,
    ExperimentId,
    Timestamp,
    ImageFilename,
    AssetId,
    CreatedAt,
}

#[derive(DeriveIden)]
enum TimePointTemperatures {
    Table,
    Id,
    TimePointId,
    ProbeSequence,
    Temperature,
}

#[derive(DeriveIden)]
enum TimePointWellStates {
    Table,
    Id,
    TimePointId,
    Row,
    Col,
    Value,
}

#[derive(DeriveIden)]
enum TemperatureReadings {
    Table,
    Id,
    ExperimentId,
    Timestamp,
    ImageFilename,
    #[sea_orm(iden = "probe_1")]
    Probe1,
    #[sea_orm(iden = "probe_2")]
    Probe2,
    #[sea_orm(iden = "probe_3")]
    Probe3,
    #[sea_orm(iden = "probe_4")]
    Probe4,
    #[sea_orm(iden = "probe_5")]
    Probe5,
    #[sea_orm(iden = "probe_6")]
    Probe6,
    #[sea_orm(iden = "probe_7")]
    Probe7,
    #[sea_orm(iden = "probe_8")]
    Probe8,
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
