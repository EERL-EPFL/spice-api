--
-- PostgreSQL database dump
--

-- Dumped from database version 15.13
-- Dumped by pg_dump version 15.13

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

--
-- Name: uuid-ossp; Type: EXTENSION; Schema: -; Owner: -
--

CREATE EXTENSION IF NOT EXISTS "uuid-ossp" WITH SCHEMA public;


--
-- Name: EXTENSION "uuid-ossp"; Type: COMMENT; Schema: -; Owner: 
--

COMMENT ON EXTENSION "uuid-ossp" IS 'generate universally unique identifiers (UUIDs)';


--
-- Name: sample_type; Type: TYPE; Schema: public; Owner: postgres
--

CREATE TYPE public.sample_type AS ENUM (
    'bulk',
    'filter',
    'procedural_blank',
    'pure_water'
);


ALTER TYPE public.sample_type OWNER TO postgres;

--
-- Name: treatment_name; Type: TYPE; Schema: public; Owner: postgres
--

CREATE TYPE public.treatment_name AS ENUM (
    'none',
    'heat',
    'h2o2'
);


ALTER TYPE public.treatment_name OWNER TO postgres;

SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: experiments; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.experiments (
    name text NOT NULL,
    username text,
    performed_at timestamp with time zone,
    temperature_ramp numeric,
    temperature_start numeric,
    temperature_end numeric,
    is_calibration boolean DEFAULT false NOT NULL,
    remarks text,
    tray_configuration_id uuid,
    created_at timestamp with time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    last_updated timestamp with time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL
);


ALTER TABLE public.experiments OWNER TO postgres;

--
-- Name: locations; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.locations (
    name character varying NOT NULL,
    comment text,
    project_id uuid,
    last_updated timestamp with time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    created_at timestamp with time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL
);


ALTER TABLE public.locations OWNER TO postgres;

--
-- Name: projects; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.projects (
    name character varying NOT NULL,
    note text,
    colour character varying,
    created_at timestamp with time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    last_updated timestamp with time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL
);


ALTER TABLE public.projects OWNER TO postgres;

--
-- Name: regions; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.regions (
    experiment_id uuid NOT NULL,
    treatment_id uuid,
    name text,
    display_colour_hex text,
    tray_id integer,
    col_min integer,
    row_min integer,
    col_max integer,
    row_max integer,
    dilution_factor integer,
    created_at timestamp with time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    last_updated timestamp with time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    is_background_key boolean DEFAULT false NOT NULL,
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL
);


ALTER TABLE public.regions OWNER TO postgres;

--
-- Name: s3_assets; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.s3_assets (
    experiment_id uuid,
    original_filename text NOT NULL,
    s3_key text NOT NULL,
    size_bytes bigint,
    uploaded_by text,
    uploaded_at timestamp with time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    is_deleted boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    last_updated timestamp with time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    type text NOT NULL,
    role text,
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL
);


ALTER TABLE public.s3_assets OWNER TO postgres;

--
-- Name: samples; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.samples (
    name text NOT NULL,
    start_time timestamp with time zone,
    stop_time timestamp with time zone,
    flow_litres_per_minute numeric(16,10),
    total_volume numeric(16,10),
    material_description text,
    extraction_procedure text,
    filter_substrate text,
    suspension_volume_litres numeric,
    air_volume_litres numeric,
    water_volume_litres numeric,
    initial_concentration_gram_l numeric,
    well_volume_litres numeric,
    remarks text,
    longitude numeric(9,6),
    latitude numeric(9,6),
    location_id uuid,
    created_at timestamp with time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    last_updated timestamp with time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    type public.sample_type NOT NULL
);


ALTER TABLE public.samples OWNER TO postgres;

--
-- Name: seaql_migrations; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.seaql_migrations (
    version character varying NOT NULL,
    applied_at bigint NOT NULL
);


ALTER TABLE public.seaql_migrations OWNER TO postgres;

--
-- Name: temperature_readings; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.temperature_readings (
    experiment_id uuid NOT NULL,
    "timestamp" timestamp with time zone NOT NULL,
    image_filename text,
    probe1 numeric,
    probe2 numeric,
    probe3 numeric,
    probe4 numeric,
    probe5 numeric,
    probe6 numeric,
    probe7 numeric,
    probe8 numeric,
    created_at timestamp with time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL
);


ALTER TABLE public.temperature_readings OWNER TO postgres;

--
-- Name: tray_configurations; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.tray_configurations (
    name text,
    experiment_default boolean NOT NULL,
    created_at timestamp with time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    last_updated timestamp with time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL
);


ALTER TABLE public.tray_configurations OWNER TO postgres;

--
-- Name: trays; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.trays (
    tray_configuration_id uuid NOT NULL,
    order_sequence integer NOT NULL,
    rotation_degrees integer NOT NULL,
    created_at timestamp with time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    last_updated timestamp with time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    name text,
    qty_x_axis integer,
    qty_y_axis integer,
    well_relative_diameter numeric,
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL
);


ALTER TABLE public.trays OWNER TO postgres;

--
-- Name: treatments; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.treatments (
    notes text,
    sample_id uuid,
    last_updated timestamp with time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    created_at timestamp with time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    enzyme_volume_litres numeric(16,10),
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    name public.treatment_name NOT NULL
);


ALTER TABLE public.treatments OWNER TO postgres;

--
-- Name: well_phase_transitions; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.well_phase_transitions (
    well_id uuid NOT NULL,
    experiment_id uuid NOT NULL,
    temperature_reading_id uuid NOT NULL,
    "timestamp" timestamp with time zone NOT NULL,
    previous_state integer NOT NULL,
    new_state integer NOT NULL,
    created_at timestamp with time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL
);


ALTER TABLE public.well_phase_transitions OWNER TO postgres;

--
-- Name: wells; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.wells (
    tray_id uuid NOT NULL,
    column_number integer NOT NULL,
    row_number integer NOT NULL,
    created_at timestamp with time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    last_updated timestamp with time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL
);


ALTER TABLE public.wells OWNER TO postgres;

--
-- Name: experiments experiments_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.experiments
    ADD CONSTRAINT experiments_pkey PRIMARY KEY (id);


--
-- Name: locations locations_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.locations
    ADD CONSTRAINT locations_pkey PRIMARY KEY (id);


--
-- Name: projects projects_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.projects
    ADD CONSTRAINT projects_pkey PRIMARY KEY (id);


--
-- Name: regions regions_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.regions
    ADD CONSTRAINT regions_pkey PRIMARY KEY (id);


--
-- Name: s3_assets s3_assets_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.s3_assets
    ADD CONSTRAINT s3_assets_pkey PRIMARY KEY (id);


--
-- Name: samples samples_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.samples
    ADD CONSTRAINT samples_pkey PRIMARY KEY (id);


--
-- Name: seaql_migrations seaql_migrations_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.seaql_migrations
    ADD CONSTRAINT seaql_migrations_pkey PRIMARY KEY (version);


--
-- Name: temperature_readings temperature_readings_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.temperature_readings
    ADD CONSTRAINT temperature_readings_pkey PRIMARY KEY (id);


--
-- Name: tray_configurations tray_configurations_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.tray_configurations
    ADD CONSTRAINT tray_configurations_pkey PRIMARY KEY (id);


--
-- Name: trays trays_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.trays
    ADD CONSTRAINT trays_pkey PRIMARY KEY (id);


--
-- Name: treatments treatments_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.treatments
    ADD CONSTRAINT treatments_pkey PRIMARY KEY (id);


--
-- Name: well_phase_transitions well_phase_transitions_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.well_phase_transitions
    ADD CONSTRAINT well_phase_transitions_pkey PRIMARY KEY (id);


--
-- Name: wells wells_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.wells
    ADD CONSTRAINT wells_pkey PRIMARY KEY (id);


--
-- Name: experiments_name_key; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX experiments_name_key ON public.experiments USING btree (name);


--
-- Name: idx_experiments_tray_configuration_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_experiments_tray_configuration_id ON public.experiments USING btree (tray_configuration_id);


--
-- Name: idx_locations_project_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_locations_project_id ON public.locations USING btree (project_id);


--
-- Name: idx_regions_experiment_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_regions_experiment_id ON public.regions USING btree (experiment_id);


--
-- Name: idx_regions_treatment_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_regions_treatment_id ON public.regions USING btree (treatment_id);


--
-- Name: idx_s3_assets_experiment_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_s3_assets_experiment_id ON public.s3_assets USING btree (experiment_id);


--
-- Name: idx_samples_location_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_samples_location_id ON public.samples USING btree (location_id);


--
-- Name: idx_temperature_readings_experiment_timestamp; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_temperature_readings_experiment_timestamp ON public.temperature_readings USING btree (experiment_id, "timestamp");


--
-- Name: idx_trays_tray_configuration_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_trays_tray_configuration_id ON public.trays USING btree (tray_configuration_id);


--
-- Name: idx_well_phase_transitions_experiment; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_well_phase_transitions_experiment ON public.well_phase_transitions USING btree (experiment_id, "timestamp");


--
-- Name: idx_well_phase_transitions_well_timestamp; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_well_phase_transitions_well_timestamp ON public.well_phase_transitions USING btree (well_id, "timestamp");


--
-- Name: idx_wells_tray_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_wells_tray_id ON public.wells USING btree (tray_id);


--
-- Name: locations_name_key; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX locations_name_key ON public.locations USING btree (name);


--
-- Name: name_uniqueness; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX name_uniqueness ON public.tray_configurations USING btree (name);


--
-- Name: no_duplicate_sequences; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX no_duplicate_sequences ON public.trays USING btree (tray_configuration_id, order_sequence);


--
-- Name: projects_name_key; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX projects_name_key ON public.projects USING btree (name);


--
-- Name: s3_assets_s3_key_key; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX s3_assets_s3_key_key ON public.s3_assets USING btree (s3_key);


--
-- Name: experiments fk_experiment_tray_configuration; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.experiments
    ADD CONSTRAINT fk_experiment_tray_configuration FOREIGN KEY (tray_configuration_id) REFERENCES public.tray_configurations(id);


--
-- Name: locations fk_locations_project_id; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.locations
    ADD CONSTRAINT fk_locations_project_id FOREIGN KEY (project_id) REFERENCES public.projects(id) ON DELETE SET NULL;


--
-- Name: temperature_readings fk_temperature_readings_experiment; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.temperature_readings
    ADD CONSTRAINT fk_temperature_readings_experiment FOREIGN KEY (experiment_id) REFERENCES public.experiments(id) ON DELETE CASCADE;


--
-- Name: trays fk_tray_assignment_to_configuration; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.trays
    ADD CONSTRAINT fk_tray_assignment_to_configuration FOREIGN KEY (tray_configuration_id) REFERENCES public.tray_configurations(id);


--
-- Name: well_phase_transitions fk_well_phase_transitions_experiment; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.well_phase_transitions
    ADD CONSTRAINT fk_well_phase_transitions_experiment FOREIGN KEY (experiment_id) REFERENCES public.experiments(id) ON DELETE CASCADE;


--
-- Name: well_phase_transitions fk_well_phase_transitions_temperature_reading; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.well_phase_transitions
    ADD CONSTRAINT fk_well_phase_transitions_temperature_reading FOREIGN KEY (temperature_reading_id) REFERENCES public.temperature_readings(id) ON DELETE CASCADE;


--
-- Name: well_phase_transitions fk_well_phase_transitions_well; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.well_phase_transitions
    ADD CONSTRAINT fk_well_phase_transitions_well FOREIGN KEY (well_id) REFERENCES public.wells(id) ON DELETE CASCADE;


--
-- Name: wells fk_wells_tray_id; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.wells
    ADD CONSTRAINT fk_wells_tray_id FOREIGN KEY (tray_id) REFERENCES public.trays(id);


--
-- Name: regions regions_experiment_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.regions
    ADD CONSTRAINT regions_experiment_id_fkey FOREIGN KEY (experiment_id) REFERENCES public.experiments(id);


--
-- Name: regions regions_treatment_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.regions
    ADD CONSTRAINT regions_treatment_id_fkey FOREIGN KEY (treatment_id) REFERENCES public.treatments(id);


--
-- Name: s3_assets s3_assets_experiment_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.s3_assets
    ADD CONSTRAINT s3_assets_experiment_id_fkey FOREIGN KEY (experiment_id) REFERENCES public.experiments(id);


--
-- Name: treatments sample_treatments; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.treatments
    ADD CONSTRAINT sample_treatments FOREIGN KEY (sample_id) REFERENCES public.samples(id);


--
-- Name: samples samples_location_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.samples
    ADD CONSTRAINT samples_location_id_fkey FOREIGN KEY (location_id) REFERENCES public.locations(id);


--
-- PostgreSQL database dump complete
--

