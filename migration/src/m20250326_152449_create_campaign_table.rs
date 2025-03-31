use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        let sql = r#"
        CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

        -- CAMPAIGNS
        CREATE TABLE IF NOT EXISTS campaign (
            id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
            name VARCHAR NOT NULL UNIQUE,
            latitude NUMERIC(9, 6),
            longitude NUMERIC(9, 6),
            comment TEXT,
            start_date TIMESTAMPTZ,
            end_date TIMESTAMPTZ,
            last_updated TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        -- EXPERIMENTS
        CREATE TABLE IF NOT EXISTS experiments (
            id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
            experiment_code TEXT NOT NULL UNIQUE,
            campaign_id UUID REFERENCES campaign(id),
            user_identifier TEXT,
            experiment_date DATE,
            experiment_time TIME,
            created_at TIMESTAMPTZ DEFAULT NOW(),
            image_capture_started_at TIMESTAMPTZ,
            image_capture_ended_at TIMESTAMPTZ,
            temperature_ramp NUMERIC,
            temperature_start NUMERIC,
            temperature_end NUMERIC,
            cooling_rate NUMERIC,
            temperature_calibration_slope NUMERIC,
            temperature_calibration_intercept NUMERIC
        );

        -- SAMPLES
        CREATE TABLE IF NOT EXISTS samples (
            id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
            experiment_id UUID NOT NULL UNIQUE REFERENCES experiments(id),
            name TEXT NOT NULL,
            type TEXT NOT NULL,
            treatment TEXT,
            material_description TEXT,
            extraction_procedure TEXT,
            filter_substrate TEXT,
            suspension_volume_liters NUMERIC,
            air_volume_liters NUMERIC,
            water_volume_liters NUMERIC,
            initial_concentration_gram_l NUMERIC,
            well_volume_liters NUMERIC,
            background_region_key TEXT,
            remarks TEXT
        );

        -- TRAYS
        CREATE TABLE IF NOT EXISTS trays (
            id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
            experiment_id UUID NOT NULL REFERENCES experiments(id),
            tray_number INTEGER CHECK (tray_number IN (1, 2)),
            n_rows INTEGER DEFAULT 8,
            n_columns INTEGER DEFAULT 12,
            well_relative_diameter NUMERIC,
            upper_left_corner_x INTEGER,
            upper_left_corner_y INTEGER,
            lower_right_corner_x INTEGER,
            lower_right_corner_y INTEGER
        );

        -- WELLS
        CREATE TABLE IF NOT EXISTS wells (
            id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
            tray_id UUID NOT NULL REFERENCES trays(id),
            row_label CHAR(1) NOT NULL,
            column_number INTEGER NOT NULL,
            UNIQUE (tray_id, row_label, column_number)
        );

        -- IMAGES
        CREATE TABLE IF NOT EXISTS images (
            id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
            experiment_id UUID NOT NULL REFERENCES experiments(id),
            filename TEXT NOT NULL,
            timestamp TIMESTAMPTZ,
            order_index INTEGER
        );

        -- CONFIGS
        CREATE TABLE IF NOT EXISTS configs (
            id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
            experiment_id UUID NOT NULL REFERENCES experiments(id),
            config_type TEXT, -- 'trays', 'temperature', 'regions'
            original_filename TEXT,
            content TEXT,
            created_at TIMESTAMPTZ DEFAULT NOW()
        );

        -- TEMPERATURE PROBES
        CREATE TABLE IF NOT EXISTS temperature_probes (
            id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
            experiment_id UUID NOT NULL REFERENCES experiments(id),
            probe_name TEXT,
            column_index INTEGER,
            correction_factor NUMERIC
        );

        -- WELL TEMPERATURES
        CREATE TABLE IF NOT EXISTS well_temperatures (
            id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
            well_id UUID NOT NULL REFERENCES wells(id),
            timestamp TIMESTAMPTZ,
            temperature_celsius NUMERIC
        );

        -- TREATMENTS
        CREATE TABLE IF NOT EXISTS treatments (
            id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
            experiment_id UUID NOT NULL REFERENCES experiments(id),
            treatment_code TEXT,
            dilution_factor NUMERIC,
            notes TEXT
        );

        -- REGIONS
        CREATE TABLE IF NOT EXISTS regions (
            id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
            experiment_id UUID NOT NULL REFERENCES experiments(id),
            region_name TEXT,
            treatment_id UUID REFERENCES treatments(id),
            wells UUID[]
        );

        -- FREEZING RESULTS
        CREATE TABLE IF NOT EXISTS freezing_results (
            id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
            well_id UUID NOT NULL REFERENCES wells(id),
            freezing_temperature_celsius NUMERIC,
            is_frozen BOOLEAN,
            region_id UUID REFERENCES regions(id)
        );

        -- INP CONCENTRATIONS
        CREATE TABLE IF NOT EXISTS inp_concentrations (
            id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
            region_id UUID NOT NULL REFERENCES regions(id),
            temperature_celsius NUMERIC,
            nm_value NUMERIC,
            error NUMERIC
        );

        -- S3 ASSETS
        CREATE TABLE IF NOT EXISTS s3_assets (
            id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
            experiment_id UUID REFERENCES experiments(id),
            original_filename TEXT NOT NULL,
            s3_key TEXT NOT NULL UNIQUE,
            size_bytes BIGINT,
            uploaded_by TEXT,
            uploaded_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            is_deleted BOOLEAN NOT NULL DEFAULT FALSE,
            type TEXT NOT NULL,   -- 'image', 'netcdf', 'xlsx'
            role TEXT             -- 'raw_image', 'merged_xlsx', 'plot', etc.
        );

        -- Indexes
        CREATE INDEX IF NOT EXISTS idx_campaign_id ON campaign (id);
        CREATE INDEX IF NOT EXISTS idx_campaign_name ON campaign (name);
        CREATE INDEX IF NOT EXISTS idx_experiment_id ON experiments (id);
        CREATE INDEX IF NOT EXISTS idx_s3_asset_id ON s3_assets (id);
        "#;

        db.execute_unprepared(sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        let sql = r#"
            DROP TABLE IF EXISTS s3_assets;
            DROP TABLE IF EXISTS inp_concentrations;
            DROP TABLE IF EXISTS freezing_results;
            DROP TABLE IF EXISTS regions;
            DROP TABLE IF EXISTS treatments;
            DROP TABLE IF EXISTS well_temperatures;
            DROP TABLE IF EXISTS temperature_probes;
            DROP TABLE IF EXISTS configs;
            DROP TABLE IF EXISTS images;
            DROP TABLE IF EXISTS wells;
            DROP TABLE IF EXISTS trays;
            DROP TABLE IF EXISTS samples;
            DROP TABLE IF EXISTS experiments;
            DROP TABLE IF EXISTS campaign;
        "#;

        db.execute_unprepared(sql).await?;
        Ok(())
    }
}
