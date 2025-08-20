use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Enable PostgreSQL extensions for enhanced search capabilities
        if manager.get_database_backend() == sea_orm::DatabaseBackend::Postgres {
            // Enable trigram extension for similarity-based search
            manager
                .get_connection()
                .execute_unprepared("CREATE EXTENSION IF NOT EXISTS pg_trgm")
                .await?;
        }

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

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
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

        // Treatments indexes
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

        // Tray configurations indexes
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

        // S3Assets indexes
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

        // Samples indexes
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

        // Projects indexes
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

        // Locations indexes
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

        Ok(())
    }
}

// Table and enum identifiers using Sea-Query DeriveIden
#[derive(DeriveIden)]
enum Locations {
    Table,
    Comment,
    CreatedAt,
    LastUpdated,
}

#[derive(DeriveIden)]
enum Projects {
    Table,
    Note,
    Colour,
    CreatedAt,
    LastUpdated,
}

#[derive(DeriveIden)]
enum Samples {
    Table,
    Name,
    Type,
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
    StartTime,
    StopTime,
    CreatedAt,
    LastUpdated,
}

#[derive(DeriveIden)]
enum S3Assets {
    Table,
    UploadedBy,
    IsDeleted,
    Type,
    Role,
    ProcessingStatus,
    ProcessingMessage,
    SizeBytes,
    UploadedAt,
    CreatedAt,
    LastUpdated,
}

#[derive(DeriveIden)]
enum TrayConfigurations {
    Table,
    ExperimentDefault,
    CreatedAt,
    LastUpdated,
}

#[derive(DeriveIden)]
enum Treatments {
    Table,
    Name,
    Notes,
    SampleId,
    EnzymeVolumeLitres,
    CreatedAt,
    LastUpdated,
}
