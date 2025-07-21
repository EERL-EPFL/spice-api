use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop unused normalized time-based tables (3 tables)

        manager
            .drop_table(
                Table::drop()
                    .table(TimePointTemperatures::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(
                Table::drop()
                    .table(TimePointWellStates::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(TimePoints::Table)
                    .if_exists()
                    .cascade()
                    .to_owned(),
            )
            .await?;
        // Drop unused results/debug tables (7 tables)
        manager
            .drop_table(
                Table::drop()
                    .table(PhaseChangeEvents::Table)
                    .if_exists()
                    .cascade()
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
            .drop_table(
                Table::drop()
                    .table(TemperatureProbeConfigurations::Table)
                    .if_exists()
                    .to_owned(),
            )
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
            .drop_table(
                Table::drop()
                    .table(InpConcentrations::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(
                Table::drop()
                    .table(PhaseChangeTemperatures::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(
                Table::drop()
                    .table(WellTemperatures::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // Note: This migration drops tables completely.
        // The down migration would need to recreate all these tables with their full schemas.
        // Since these tables are unused and we're removing them intentionally,
        // we don't implement the down migration (would be very complex).
        // If needed, the tables can be recreated from the original schema migrations.

        println!("Warning: Cannot recreate dropped tables in down migration.");
        println!("If you need to restore these tables, please restore from backup or run the original schema migrations:");
        println!("- time_points, time_point_temperatures, time_point_well_states");
        println!("- phase_change_events, temperature_probes, temperature_probe_configurations");
        println!(
            "- freezing_results, inp_concentrations, phase_change_temperatures, well_temperatures"
        );

        Ok(())
    }
}

// Table identifiers for the tables we're dropping
#[derive(DeriveIden)]
enum TimePoints {
    Table,
}

#[derive(DeriveIden)]
enum TimePointTemperatures {
    Table,
}

#[derive(DeriveIden)]
enum TimePointWellStates {
    Table,
}

#[derive(DeriveIden)]
enum PhaseChangeEvents {
    Table,
}

#[derive(DeriveIden)]
enum TemperatureProbes {
    Table,
}

#[derive(DeriveIden)]
enum TemperatureProbeConfigurations {
    Table,
}

#[derive(DeriveIden)]
enum FreezingResults {
    Table,
}

#[derive(DeriveIden)]
enum InpConcentrations {
    Table,
}

#[derive(DeriveIden)]
enum PhaseChangeTemperatures {
    Table,
}

#[derive(DeriveIden)]
enum WellTemperatures {
    Table,
}
