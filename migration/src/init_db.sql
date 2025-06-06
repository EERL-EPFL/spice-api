-- ** Database generated with pgModeler (PostgreSQL Database Modeler).
-- ** pgModeler version: 1.2.0
-- ** PostgreSQL version: 17.0
-- ** Project Site: pgmodeler.io
-- ** Model Author: ---

SET search_path TO pg_catalog,public;
-- ddl-end --

-- object: fuzzystrmatch | type: EXTENSION --
-- DROP EXTENSION IF EXISTS fuzzystrmatch CASCADE;
CREATE EXTENSION fuzzystrmatch
WITH SCHEMA public
VERSION '1.2';
-- ddl-end --
COMMENT ON EXTENSION fuzzystrmatch IS E'determine similarities and distance between strings';
-- ddl-end --

-- object: "uuid-ossp" | type: EXTENSION --
-- DROP EXTENSION IF EXISTS "uuid-ossp" CASCADE;
CREATE EXTENSION "uuid-ossp"
WITH SCHEMA public
VERSION '1.1';
-- ddl-end --
COMMENT ON EXTENSION "uuid-ossp" IS E'generate universally unique identifiers (UUIDs)';
-- ddl-end --

-- object: public.campaign | type: TABLE --
-- DROP TABLE IF EXISTS public.campaign CASCADE;
CREATE TABLE public.campaign (
	id uuid NOT NULL DEFAULT uuid_generate_v4(),
	name character varying NOT NULL,
	comment text,
	start_date timestamptz,
	end_date timestamptz,
	last_updated timestamptz NOT NULL DEFAULT now(),
	created_at timestamptz NOT NULL DEFAULT now(),
	CONSTRAINT campaign_pkey PRIMARY KEY (id),
	CONSTRAINT campaign_name_key UNIQUE (name)
);
-- ddl-end --
ALTER TABLE public.campaign OWNER TO postgres;
-- ddl-end --

-- object: public.sample_type | type: TYPE --
-- DROP TYPE IF EXISTS public.sample_type CASCADE;
CREATE TYPE public.sample_type AS
ENUM ('bulk','filter','blank');
-- ddl-end --
ALTER TYPE public.sample_type OWNER TO postgres;
-- ddl-end --

-- object: public.experiments | type: TABLE --
-- DROP TABLE IF EXISTS public.experiments CASCADE;
CREATE TABLE public.experiments (
	id uuid NOT NULL DEFAULT uuid_generate_v4(),
	name text NOT NULL,
	username text,
	performed_at timestamptz,
	temperature_ramp numeric,
	temperature_start numeric,
	temperature_end numeric,
	is_calibration boolean NOT NULL DEFAULT false,
	remarks text,
	tray_configuration_id uuid,
	created_at timestamptz NOT NULL DEFAULT now(),
	last_updated timestamptz NOT NULL DEFAULT now(),
	CONSTRAINT experiments_pkey PRIMARY KEY (id),
	CONSTRAINT experiments_name_key UNIQUE (name)
);
-- ddl-end --
ALTER TABLE public.experiments OWNER TO postgres;
-- ddl-end --

-- object: public.trays | type: TABLE --
-- DROP TABLE IF EXISTS public.trays CASCADE;
CREATE TABLE public.trays (
	id uuid NOT NULL DEFAULT uuid_generate_v4(),
	name text,
	qty_x_axis integer DEFAULT 8,
	qty_y_axis integer DEFAULT 12,
	well_relative_diameter numeric,
	last_updated timestamptz NOT NULL DEFAULT now(),
	created_at timestamptz NOT NULL DEFAULT now(),
	CONSTRAINT trays_tray_number_check CHECK ((tray_number = ANY (ARRAY[1, 2]))),
	CONSTRAINT trays_pkey PRIMARY KEY (id)
);
-- ddl-end --
ALTER TABLE public.trays OWNER TO postgres;
-- ddl-end --

-- object: public.wells | type: TABLE --
-- DROP TABLE IF EXISTS public.wells CASCADE;
CREATE TABLE public.wells (
	id uuid NOT NULL DEFAULT uuid_generate_v4(),
	tray_id uuid NOT NULL,
	row_label character(1) NOT NULL,
	column_number integer NOT NULL,
	created_at timestamptz NOT NULL DEFAULT now(),
	last_updated timestamptz NOT NULL DEFAULT now(),
	CONSTRAINT wells_pkey PRIMARY KEY (id),
	CONSTRAINT wells_tray_id_row_label_column_number_key UNIQUE (tray_id,row_label,column_number)
);
-- ddl-end --
ALTER TABLE public.wells OWNER TO postgres;
-- ddl-end --

-- object: public.temperature_probes | type: TABLE --
-- DROP TABLE IF EXISTS public.temperature_probes CASCADE;
CREATE TABLE public.temperature_probes (
	id uuid NOT NULL DEFAULT uuid_generate_v4(),
	experiment_id uuid NOT NULL,
	probe_name text,
	column_index integer,
	created_at timestamptz NOT NULL DEFAULT now(),
	last_updated timestamptz NOT NULL DEFAULT now(),
	correction_factor numeric,
	CONSTRAINT temperature_probes_pkey PRIMARY KEY (id)
);
-- ddl-end --
ALTER TABLE public.temperature_probes OWNER TO postgres;
-- ddl-end --

-- object: public.well_temperatures | type: TABLE --
-- DROP TABLE IF EXISTS public.well_temperatures CASCADE;
CREATE TABLE public.well_temperatures (
	id uuid NOT NULL DEFAULT uuid_generate_v4(),
	well_id uuid NOT NULL,
	"timestamp" timestamptz,
	temperature_celsius numeric,
	created_at timestamptz NOT NULL DEFAULT now(),
	last_updated timestamptz NOT NULL DEFAULT now(),
	CONSTRAINT well_temperatures_pkey PRIMARY KEY (id)
);
-- ddl-end --
ALTER TABLE public.well_temperatures OWNER TO postgres;
-- ddl-end --

-- object: public.treatments | type: TABLE --
-- DROP TABLE IF EXISTS public.treatments CASCADE;
CREATE TABLE public.treatments (
	id uuid NOT NULL DEFAULT uuid_generate_v4(),
	name text,
	notes text,
	sample_id uuid,
	last_updated timestamptz NOT NULL DEFAULT now(),
	created_at timestamptz NOT NULL DEFAULT now(),
	enzyme_volume_microlitres float,
	CONSTRAINT treatments_pkey PRIMARY KEY (id)
);
-- ddl-end --
COMMENT ON COLUMN public.treatments.enzyme_volume_microlitres IS E'Only applicable to the peroxide treatment';
-- ddl-end --
ALTER TABLE public.treatments OWNER TO postgres;
-- ddl-end --

-- object: public.regions | type: TABLE --
-- DROP TABLE IF EXISTS public.regions CASCADE;
CREATE TABLE public.regions (
	id uuid NOT NULL DEFAULT uuid_generate_v4(),
	experiment_id uuid NOT NULL,
	treatment_id uuid,
	name text,
	display_colour_hex text,
	tray_id smallint,
	upper_left_corner_x smallint,
	upper_left_corner_y smallint,
	lower_right_corner_x smallint,
	lower_right_corner_y smallint,
	dilution_factor smallint,
	created_at timestamptz NOT NULL DEFAULT now(),
	last_updated timestamptz NOT NULL DEFAULT now(),
	CONSTRAINT regions_pkey PRIMARY KEY (id)
);
-- ddl-end --
ALTER TABLE public.regions OWNER TO postgres;
-- ddl-end --

-- object: public.freezing_results | type: TABLE --
-- DROP TABLE IF EXISTS public.freezing_results CASCADE;
CREATE TABLE public.freezing_results (
	id uuid NOT NULL DEFAULT uuid_generate_v4(),
	well_id uuid NOT NULL,
	freezing_temperature_celsius numeric,
	is_frozen boolean,
	region_id uuid,
	created_at timestamptz NOT NULL DEFAULT now(),
	last_updated timestamptz NOT NULL DEFAULT now(),
	CONSTRAINT freezing_results_pkey PRIMARY KEY (id)
);
-- ddl-end --
ALTER TABLE public.freezing_results OWNER TO postgres;
-- ddl-end --

-- object: public.inp_concentrations | type: TABLE --
-- DROP TABLE IF EXISTS public.inp_concentrations CASCADE;
CREATE TABLE public.inp_concentrations (
	id uuid NOT NULL DEFAULT uuid_generate_v4(),
	region_id uuid NOT NULL,
	temperature_celsius numeric,
	nm_value numeric,
	error numeric,
	created_at timestamptz NOT NULL DEFAULT now(),
	last_updated timestamptz NOT NULL DEFAULT now(),
	CONSTRAINT inp_concentrations_pkey PRIMARY KEY (id)
);
-- ddl-end --
ALTER TABLE public.inp_concentrations OWNER TO postgres;
-- ddl-end --

-- object: public.s3_assets | type: TABLE --
-- DROP TABLE IF EXISTS public.s3_assets CASCADE;
CREATE TABLE public.s3_assets (
	id uuid NOT NULL DEFAULT uuid_generate_v4(),
	experiment_id uuid,
	original_filename text NOT NULL,
	s3_key text NOT NULL,
	size_bytes bigint,
	uploaded_by text,
	uploaded_at timestamptz NOT NULL DEFAULT now(),
	is_deleted boolean NOT NULL DEFAULT false,
	created_at timestamptz NOT NULL DEFAULT now(),
	last_updated timestamptz NOT NULL DEFAULT now(),
	type text NOT NULL,
	role text,
	CONSTRAINT s3_assets_pkey PRIMARY KEY (id),
	CONSTRAINT s3_assets_s3_key_key UNIQUE (s3_key)
);
-- ddl-end --
ALTER TABLE public.s3_assets OWNER TO postgres;
-- ddl-end --

-- object: idx_campaign_id | type: INDEX --
-- DROP INDEX IF EXISTS public.idx_campaign_id CASCADE;
CREATE INDEX idx_campaign_id ON public.campaign
USING btree
(
	id
)
WITH (FILLFACTOR = 90);
-- ddl-end --

-- object: idx_campaign_name | type: INDEX --
-- DROP INDEX IF EXISTS public.idx_campaign_name CASCADE;
CREATE INDEX idx_campaign_name ON public.campaign
USING btree
(
	name
)
WITH (FILLFACTOR = 90);
-- ddl-end --

-- object: idx_experiment_id | type: INDEX --
-- DROP INDEX IF EXISTS public.idx_experiment_id CASCADE;
CREATE INDEX idx_experiment_id ON public.experiments
USING btree
(
	id
)
WITH (FILLFACTOR = 90);
-- ddl-end --

-- object: idx_s3_asset_id | type: INDEX --
-- DROP INDEX IF EXISTS public.idx_s3_asset_id CASCADE;
CREATE INDEX idx_s3_asset_id ON public.s3_assets
USING btree
(
	id
)
WITH (FILLFACTOR = 90);
-- ddl-end --

-- object: public.samples | type: TABLE --
-- DROP TABLE IF EXISTS public.samples CASCADE;
CREATE TABLE public.samples (
	id uuid NOT NULL DEFAULT uuid_generate_v4(),
	name text NOT NULL,
	type public.sample_type NOT NULL,
	start_time timestamptz,
	stop_time timestamptz,
	flow_litres_per_minute float,
	total_volume float,
	material_description text,
	extraction_procedure text,
	filter_substrate text,
	suspension_volume_liters numeric,
	air_volume_liters numeric,
	water_volume_liters numeric,
	initial_concentration_gram_l numeric,
	well_volume_liters numeric,
	background_region_key text,
	remarks text,
	longitude numeric(9,6),
	latitude numeric(9,6),
	campaign_id uuid,
	created_at timestamptz NOT NULL DEFAULT now(),
	last_updated timestamptz NOT NULL DEFAULT now(),
	CONSTRAINT samples_pkey PRIMARY KEY (id)
);
-- ddl-end --
ALTER TABLE public.samples OWNER TO postgres;
-- ddl-end --

-- object: public.tray_configurations | type: TABLE --
-- DROP TABLE IF EXISTS public.tray_configurations CASCADE;
CREATE TABLE public.tray_configurations (
	id uuid NOT NULL,
	name text,
	experiment_default boolean,
	CONSTRAINT tray_configurations_pkey PRIMARY KEY (id),
	CONSTRAINT name_uniqueness UNIQUE (name)
);
-- ddl-end --
ALTER TABLE public.tray_configurations OWNER TO postgres;
-- ddl-end --

-- object: public.tray_configuration_assignments | type: TABLE --
-- DROP TABLE IF EXISTS public.tray_configuration_assignments CASCADE;
CREATE TABLE public.tray_configuration_assignments (
	tray_id uuid NOT NULL,
	tray_configuration_id uuid NOT NULL,
	order_sequence smallint NOT NULL,
	rotation_degrees smallint,
	CONSTRAINT tray_configuration_assignments_pk PRIMARY KEY (tray_id,tray_configuration_id,order_sequence)
);
-- ddl-end --
ALTER TABLE public.tray_configuration_assignments OWNER TO postgres;
-- ddl-end --

-- object: fk_experiment_tray_configuration | type: CONSTRAINT --
-- ALTER TABLE public.experiments DROP CONSTRAINT IF EXISTS fk_experiment_tray_configuration CASCADE;
ALTER TABLE public.experiments ADD CONSTRAINT fk_experiment_tray_configuration FOREIGN KEY (tray_configuration_id)
REFERENCES public.tray_configurations (id) MATCH SIMPLE
ON DELETE NO ACTION ON UPDATE NO ACTION;
-- ddl-end --

-- object: wells_tray_id_fkey | type: CONSTRAINT --
-- ALTER TABLE public.wells DROP CONSTRAINT IF EXISTS wells_tray_id_fkey CASCADE;
ALTER TABLE public.wells ADD CONSTRAINT wells_tray_id_fkey FOREIGN KEY (tray_id)
REFERENCES public.trays (id) MATCH SIMPLE
ON DELETE NO ACTION ON UPDATE NO ACTION;
-- ddl-end --

-- object: temperature_probes_experiment_id_fkey | type: CONSTRAINT --
-- ALTER TABLE public.temperature_probes DROP CONSTRAINT IF EXISTS temperature_probes_experiment_id_fkey CASCADE;
ALTER TABLE public.temperature_probes ADD CONSTRAINT temperature_probes_experiment_id_fkey FOREIGN KEY (experiment_id)
REFERENCES public.experiments (id) MATCH SIMPLE
ON DELETE NO ACTION ON UPDATE NO ACTION;
-- ddl-end --

-- object: well_temperatures_well_id_fkey | type: CONSTRAINT --
-- ALTER TABLE public.well_temperatures DROP CONSTRAINT IF EXISTS well_temperatures_well_id_fkey CASCADE;
ALTER TABLE public.well_temperatures ADD CONSTRAINT well_temperatures_well_id_fkey FOREIGN KEY (well_id)
REFERENCES public.wells (id) MATCH SIMPLE
ON DELETE NO ACTION ON UPDATE NO ACTION;
-- ddl-end --

-- object: sample_treatments | type: CONSTRAINT --
-- ALTER TABLE public.treatments DROP CONSTRAINT IF EXISTS sample_treatments CASCADE;
ALTER TABLE public.treatments ADD CONSTRAINT sample_treatments FOREIGN KEY (sample_id)
REFERENCES public.samples (id) MATCH SIMPLE
ON DELETE NO ACTION ON UPDATE NO ACTION;
-- ddl-end --

-- object: regions_experiment_id_fkey | type: CONSTRAINT --
-- ALTER TABLE public.regions DROP CONSTRAINT IF EXISTS regions_experiment_id_fkey CASCADE;
ALTER TABLE public.regions ADD CONSTRAINT regions_experiment_id_fkey FOREIGN KEY (experiment_id)
REFERENCES public.experiments (id) MATCH SIMPLE
ON DELETE NO ACTION ON UPDATE NO ACTION;
-- ddl-end --

-- object: regions_treatment_id_fkey | type: CONSTRAINT --
-- ALTER TABLE public.regions DROP CONSTRAINT IF EXISTS regions_treatment_id_fkey CASCADE;
ALTER TABLE public.regions ADD CONSTRAINT regions_treatment_id_fkey FOREIGN KEY (treatment_id)
REFERENCES public.treatments (id) MATCH SIMPLE
ON DELETE NO ACTION ON UPDATE NO ACTION;
-- ddl-end --

-- object: freezing_results_well_id_fkey | type: CONSTRAINT --
-- ALTER TABLE public.freezing_results DROP CONSTRAINT IF EXISTS freezing_results_well_id_fkey CASCADE;
ALTER TABLE public.freezing_results ADD CONSTRAINT freezing_results_well_id_fkey FOREIGN KEY (well_id)
REFERENCES public.wells (id) MATCH SIMPLE
ON DELETE NO ACTION ON UPDATE NO ACTION;
-- ddl-end --

-- object: freezing_results_region_id_fkey | type: CONSTRAINT --
-- ALTER TABLE public.freezing_results DROP CONSTRAINT IF EXISTS freezing_results_region_id_fkey CASCADE;
ALTER TABLE public.freezing_results ADD CONSTRAINT freezing_results_region_id_fkey FOREIGN KEY (region_id)
REFERENCES public.regions (id) MATCH SIMPLE
ON DELETE NO ACTION ON UPDATE NO ACTION;
-- ddl-end --

-- object: inp_concentrations_region_id_fkey | type: CONSTRAINT --
-- ALTER TABLE public.inp_concentrations DROP CONSTRAINT IF EXISTS inp_concentrations_region_id_fkey CASCADE;
ALTER TABLE public.inp_concentrations ADD CONSTRAINT inp_concentrations_region_id_fkey FOREIGN KEY (region_id)
REFERENCES public.regions (id) MATCH SIMPLE
ON DELETE NO ACTION ON UPDATE NO ACTION;
-- ddl-end --

-- object: s3_assets_experiment_id_fkey | type: CONSTRAINT --
-- ALTER TABLE public.s3_assets DROP CONSTRAINT IF EXISTS s3_assets_experiment_id_fkey CASCADE;
ALTER TABLE public.s3_assets ADD CONSTRAINT s3_assets_experiment_id_fkey FOREIGN KEY (experiment_id)
REFERENCES public.experiments (id) MATCH SIMPLE
ON DELETE NO ACTION ON UPDATE NO ACTION;
-- ddl-end --

-- object: samples_campaign_id_fkey | type: CONSTRAINT --
-- ALTER TABLE public.samples DROP CONSTRAINT IF EXISTS samples_campaign_id_fkey CASCADE;
ALTER TABLE public.samples ADD CONSTRAINT samples_campaign_id_fkey FOREIGN KEY (campaign_id)
REFERENCES public.campaign (id) MATCH SIMPLE
ON DELETE NO ACTION ON UPDATE NO ACTION;
-- ddl-end --

-- object: fk_tray_assignments_to_tray | type: CONSTRAINT --
-- ALTER TABLE public.tray_configuration_assignments DROP CONSTRAINT IF EXISTS fk_tray_assignments_to_tray CASCADE;
ALTER TABLE public.tray_configuration_assignments ADD CONSTRAINT fk_tray_assignments_to_tray FOREIGN KEY (tray_id)
REFERENCES public.trays (id) MATCH SIMPLE
ON DELETE NO ACTION ON UPDATE NO ACTION;
-- ddl-end --

-- object: fk_tray_assignment_to_configuration | type: CONSTRAINT --
-- ALTER TABLE public.tray_configuration_assignments DROP CONSTRAINT IF EXISTS fk_tray_assignment_to_configuration CASCADE;
ALTER TABLE public.tray_configuration_assignments ADD CONSTRAINT fk_tray_assignment_to_configuration FOREIGN KEY (tray_configuration_id)
REFERENCES public.tray_configurations (id) MATCH SIMPLE
ON DELETE NO ACTION ON UPDATE NO ACTION;
-- ddl-end --


