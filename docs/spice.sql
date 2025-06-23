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
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    name text NOT NULL,
    username text,
    performed_at timestamp with time zone,
    temperature_ramp numeric,
    temperature_start numeric,
    temperature_end numeric,
    is_calibration boolean DEFAULT false NOT NULL,
    remarks text,
    tray_configuration_id uuid,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    last_updated timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.experiments OWNER TO postgres;

--
-- Name: freezing_results; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.freezing_results (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    well_id uuid NOT NULL,
    freezing_temperature_celsius numeric,
    is_frozen boolean,
    region_id uuid,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    last_updated timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.freezing_results OWNER TO postgres;

--
-- Name: inp_concentrations; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.inp_concentrations (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    region_id uuid NOT NULL,
    temperature_celsius numeric,
    nm_value numeric,
    error numeric,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    last_updated timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.inp_concentrations OWNER TO postgres;

--
-- Name: locations; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.locations (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    name character varying NOT NULL,
    comment text,
    start_date timestamp with time zone,
    end_date timestamp with time zone,
    last_updated timestamp with time zone DEFAULT now() NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    project_id uuid
);


ALTER TABLE public.locations OWNER TO postgres;

--
-- Name: projects; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.projects (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    name character varying NOT NULL,
    note text,
    colour character varying,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    last_updated timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.projects OWNER TO postgres;

--
-- Name: regions; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.regions (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    experiment_id uuid NOT NULL,
    treatment_id uuid,
    name text,
    display_colour_hex text,
    tray_id smallint,
    col_min smallint,
    row_min smallint,
    col_max smallint,
    row_max smallint,
    dilution_factor smallint,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    last_updated timestamp with time zone DEFAULT now() NOT NULL,
    is_background_key boolean DEFAULT false NOT NULL
);


ALTER TABLE public.regions OWNER TO postgres;

--
-- Name: s3_assets; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.s3_assets (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    experiment_id uuid,
    original_filename text NOT NULL,
    s3_key text NOT NULL,
    size_bytes bigint,
    uploaded_by text,
    uploaded_at timestamp with time zone DEFAULT now() NOT NULL,
    is_deleted boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    last_updated timestamp with time zone DEFAULT now() NOT NULL,
    type text NOT NULL,
    role text
);


ALTER TABLE public.s3_assets OWNER TO postgres;

--
-- Name: samples; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.samples (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    name text NOT NULL,
    type public.sample_type NOT NULL,
    start_time timestamp with time zone,
    stop_time timestamp with time zone,
    flow_litres_per_minute numeric(20,10),
    total_volume numeric(20,10),
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
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    last_updated timestamp with time zone DEFAULT now() NOT NULL
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
-- Name: temperature_probes; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.temperature_probes (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    experiment_id uuid NOT NULL,
    probe_name text,
    column_index integer,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    last_updated timestamp with time zone DEFAULT now() NOT NULL,
    correction_factor numeric
);


ALTER TABLE public.temperature_probes OWNER TO postgres;

--
-- Name: tray_configuration_assignments; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.tray_configuration_assignments (
    tray_id uuid NOT NULL,
    tray_configuration_id uuid NOT NULL,
    order_sequence smallint NOT NULL,
    rotation_degrees smallint NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    last_updated timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.tray_configuration_assignments OWNER TO postgres;

--
-- Name: tray_configurations; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.tray_configurations (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    name text,
    experiment_default boolean NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    last_updated timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.tray_configurations OWNER TO postgres;

--
-- Name: trays; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.trays (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    name text,
    qty_x_axis integer DEFAULT 8,
    qty_y_axis integer DEFAULT 12,
    well_relative_diameter numeric,
    last_updated timestamp with time zone DEFAULT now() NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.trays OWNER TO postgres;

--
-- Name: treatments; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.treatments (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    notes text,
    sample_id uuid,
    last_updated timestamp with time zone DEFAULT now() NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    enzyme_volume_litres numeric(20,10),
    name public.treatment_name NOT NULL
);


ALTER TABLE public.treatments OWNER TO postgres;

--
-- Name: COLUMN treatments.enzyme_volume_litres; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.treatments.enzyme_volume_litres IS 'Only applicable to the peroxide treatment';


--
-- Name: well_temperatures; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.well_temperatures (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    well_id uuid NOT NULL,
    "timestamp" timestamp with time zone,
    temperature_celsius numeric,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    last_updated timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.well_temperatures OWNER TO postgres;

--
-- Name: wells; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.wells (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    tray_id uuid NOT NULL,
    column_number integer NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    last_updated timestamp with time zone DEFAULT now() NOT NULL,
    row_number integer NOT NULL
);


ALTER TABLE public.wells OWNER TO postgres;

--
-- Name: locations campaign_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.locations
    ADD CONSTRAINT campaign_pkey PRIMARY KEY (id);


--
-- Name: experiments experiments_name_key; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.experiments
    ADD CONSTRAINT experiments_name_key UNIQUE (name);


--
-- Name: experiments experiments_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.experiments
    ADD CONSTRAINT experiments_pkey PRIMARY KEY (id);


--
-- Name: freezing_results freezing_results_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.freezing_results
    ADD CONSTRAINT freezing_results_pkey PRIMARY KEY (id);


--
-- Name: inp_concentrations inp_concentrations_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.inp_concentrations
    ADD CONSTRAINT inp_concentrations_pkey PRIMARY KEY (id);


--
-- Name: tray_configurations name_uniqueness; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.tray_configurations
    ADD CONSTRAINT name_uniqueness UNIQUE (name);


--
-- Name: tray_configuration_assignments no_duplicate_sequences; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.tray_configuration_assignments
    ADD CONSTRAINT no_duplicate_sequences UNIQUE (tray_configuration_id, order_sequence);


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
-- Name: s3_assets s3_assets_s3_key_key; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.s3_assets
    ADD CONSTRAINT s3_assets_s3_key_key UNIQUE (s3_key);


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
-- Name: temperature_probes temperature_probes_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.temperature_probes
    ADD CONSTRAINT temperature_probes_pkey PRIMARY KEY (id);


--
-- Name: tray_configuration_assignments tray_configuration_assignments_pk; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.tray_configuration_assignments
    ADD CONSTRAINT tray_configuration_assignments_pk PRIMARY KEY (tray_id, tray_configuration_id, order_sequence);


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
-- Name: well_temperatures well_temperatures_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.well_temperatures
    ADD CONSTRAINT well_temperatures_pkey PRIMARY KEY (id);


--
-- Name: wells wells_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.wells
    ADD CONSTRAINT wells_pkey PRIMARY KEY (id);


--
-- Name: idx_experiment_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_experiment_id ON public.experiments USING btree (id) WITH (fillfactor='90');


--
-- Name: idx_experiments_tray_configuration_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_experiments_tray_configuration_id ON public.experiments USING btree (tray_configuration_id) WITH (fillfactor='90');


--
-- Name: idx_freezing_results_region_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_freezing_results_region_id ON public.freezing_results USING btree (region_id) WITH (fillfactor='90');


--
-- Name: idx_freezing_results_well_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_freezing_results_well_id ON public.freezing_results USING btree (well_id) WITH (fillfactor='90');


--
-- Name: idx_inp_concentrations_region_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_inp_concentrations_region_id ON public.inp_concentrations USING btree (region_id) WITH (fillfactor='90');


--
-- Name: idx_locations_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_locations_id ON public.locations USING btree (id);


--
-- Name: idx_locations_name; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_locations_name ON public.locations USING btree (name);


--
-- Name: idx_locations_project_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_locations_project_id ON public.locations USING btree (project_id);


--
-- Name: idx_projects_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_projects_id ON public.projects USING btree (id);


--
-- Name: idx_projects_name; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_projects_name ON public.projects USING btree (name);


--
-- Name: idx_regions_experiment_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_regions_experiment_id ON public.regions USING btree (experiment_id) WITH (fillfactor='90');


--
-- Name: idx_regions_treatment_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_regions_treatment_id ON public.regions USING btree (treatment_id) WITH (fillfactor='90');


--
-- Name: idx_s3_asset_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_s3_asset_id ON public.s3_assets USING btree (id) WITH (fillfactor='90');


--
-- Name: idx_s3_assets_experiment_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_s3_assets_experiment_id ON public.s3_assets USING btree (experiment_id) WITH (fillfactor='90');


--
-- Name: idx_samples_location_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_samples_location_id ON public.samples USING btree (location_id);


--
-- Name: idx_temperature_probes_experiment_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_temperature_probes_experiment_id ON public.temperature_probes USING btree (experiment_id) WITH (fillfactor='90');


--
-- Name: idx_tray_configuration_assignments_tray_configuration_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_tray_configuration_assignments_tray_configuration_id ON public.tray_configuration_assignments USING btree (tray_configuration_id) WITH (fillfactor='90');


--
-- Name: idx_tray_configuration_assignments_tray_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_tray_configuration_assignments_tray_id ON public.tray_configuration_assignments USING btree (tray_id) WITH (fillfactor='90');


--
-- Name: idx_well_temperatures_well_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_well_temperatures_well_id ON public.well_temperatures USING btree (well_id) WITH (fillfactor='90');


--
-- Name: idx_wells_tray_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_wells_tray_id ON public.wells USING btree (tray_id) WITH (fillfactor='90');


--
-- Name: locations_name_key; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX locations_name_key ON public.locations USING btree (name);


--
-- Name: projects_name_key; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX projects_name_key ON public.projects USING btree (name);


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
-- Name: tray_configuration_assignments fk_tray_assignment_to_configuration; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.tray_configuration_assignments
    ADD CONSTRAINT fk_tray_assignment_to_configuration FOREIGN KEY (tray_configuration_id) REFERENCES public.tray_configurations(id);


--
-- Name: tray_configuration_assignments fk_tray_assignments_to_tray; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.tray_configuration_assignments
    ADD CONSTRAINT fk_tray_assignments_to_tray FOREIGN KEY (tray_id) REFERENCES public.trays(id);


--
-- Name: freezing_results freezing_results_region_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.freezing_results
    ADD CONSTRAINT freezing_results_region_id_fkey FOREIGN KEY (region_id) REFERENCES public.regions(id);


--
-- Name: freezing_results freezing_results_well_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.freezing_results
    ADD CONSTRAINT freezing_results_well_id_fkey FOREIGN KEY (well_id) REFERENCES public.wells(id);


--
-- Name: inp_concentrations inp_concentrations_region_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.inp_concentrations
    ADD CONSTRAINT inp_concentrations_region_id_fkey FOREIGN KEY (region_id) REFERENCES public.regions(id);


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
-- Name: temperature_probes temperature_probes_experiment_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.temperature_probes
    ADD CONSTRAINT temperature_probes_experiment_id_fkey FOREIGN KEY (experiment_id) REFERENCES public.experiments(id);


--
-- Name: well_temperatures well_temperatures_well_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.well_temperatures
    ADD CONSTRAINT well_temperatures_well_id_fkey FOREIGN KEY (well_id) REFERENCES public.wells(id);


--
-- Name: wells wells_tray_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.wells
    ADD CONSTRAINT wells_tray_id_fkey FOREIGN KEY (tray_id) REFERENCES public.trays(id);


--
-- PostgreSQL database dump complete
--

