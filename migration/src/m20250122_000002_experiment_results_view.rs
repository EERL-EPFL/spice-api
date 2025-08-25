use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create materialized view for experiment results status
        // This view checks if an experiment has any temperature readings or phase transitions
        let create_view_sql = match manager.get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => {
                r#"
                CREATE MATERIALIZED VIEW experiment_results_view AS
                SELECT 
                    e.id,
                    e.name,
                    e.username,
                    e.performed_at,
                    e.temperature_ramp,
                    e.temperature_start,
                    e.temperature_end,
                    e.is_calibration,
                    e.remarks,
                    e.tray_configuration_id,
                    e.created_at,
                    e.last_updated,
                    (
                        EXISTS (
                            SELECT 1 FROM temperature_readings tr WHERE tr.experiment_id = e.id
                        ) OR EXISTS (
                            SELECT 1 FROM well_phase_transitions wpt WHERE wpt.experiment_id = e.id
                        )
                    ) as has_results
                FROM experiments e;

                -- Create unique index on id for better performance and to support refreshes
                CREATE UNIQUE INDEX idx_experiment_results_view_id ON experiment_results_view (id);
                
                -- Create indexes for common query patterns
                CREATE INDEX idx_experiment_results_view_has_results ON experiment_results_view (has_results);
                CREATE INDEX idx_experiment_results_view_performed_at ON experiment_results_view (performed_at);
                CREATE INDEX idx_experiment_results_view_username ON experiment_results_view (username);
                "#
            }
            sea_orm::DatabaseBackend::Sqlite => {
                // SQLite doesn't support materialized views, so create a regular view
                r#"
                CREATE VIEW experiment_results_view AS
                SELECT 
                    e.id,
                    e.name,
                    e.username,
                    e.performed_at,
                    e.temperature_ramp,
                    e.temperature_start,
                    e.temperature_end,
                    e.is_calibration,
                    e.remarks,
                    e.tray_configuration_id,
                    e.created_at,
                    e.last_updated,
                    (
                        EXISTS (
                            SELECT 1 FROM temperature_readings tr WHERE tr.experiment_id = e.id
                        ) OR EXISTS (
                            SELECT 1 FROM well_phase_transitions wpt WHERE wpt.experiment_id = e.id
                        )
                    ) as has_results
                FROM experiments e;
                "#
            }
            _ => {
                return Err(DbErr::Custom("Unsupported database backend".to_string()));
            }
        };

        manager.get_connection().execute_unprepared(create_view_sql).await?;

        // For PostgreSQL, create a function to refresh the materialized view
        if manager.get_database_backend() == sea_orm::DatabaseBackend::Postgres {
            let refresh_function_sql = r#"
                CREATE OR REPLACE FUNCTION refresh_experiment_results_view() 
                RETURNS void AS $$
                BEGIN
                    REFRESH MATERIALIZED VIEW CONCURRENTLY experiment_results_view;
                END;
                $$ LANGUAGE plpgsql;
            "#;
            
            manager.get_connection().execute_unprepared(refresh_function_sql).await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop the view and associated objects
        let drop_sql = match manager.get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => {
                r#"
                DROP FUNCTION IF EXISTS refresh_experiment_results_view();
                DROP MATERIALIZED VIEW IF EXISTS experiment_results_view;
                "#
            }
            sea_orm::DatabaseBackend::Sqlite => {
                "DROP VIEW IF EXISTS experiment_results_view;"
            }
            _ => {
                return Err(DbErr::Custom("Unsupported database backend".to_string()));
            }
        };

        manager.get_connection().execute_unprepared(drop_sql).await?;
        Ok(())
    }
}