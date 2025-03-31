use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Create the campaign table
        let create_campaign_table = r#"
            CREATE TABLE IF NOT EXISTS public.campaign (
                id uuid NOT NULL PRIMARY KEY,
                name character varying NOT NULL,
                latitude numeric(9, 6),
                longitude numeric(9, 6),
                comment character varying,
                start_date timestamptz,
                end_date timestamptz,
                last_updated timestamptz NOT NULL,
                user_id uuid
            );
        "#;
        db.execute_unprepared(create_campaign_table).await?;

        // Create indexes for the campaign table
        let create_campaign_id_index = r#"
            CREATE INDEX IF NOT EXISTS idx_campaign_id ON public.campaign (id);
        "#;
        db.execute_unprepared(create_campaign_id_index).await?;

        let create_campaign_name_index = r#"
            CREATE INDEX IF NOT EXISTS idx_campaign_name ON public.campaign (name);
        "#;
        db.execute_unprepared(create_campaign_name_index).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Drop the campaign table
        let drop_campaign_table = r#"
            DROP TABLE IF EXISTS public.campaign;
        "#;
        db.execute_unprepared(drop_campaign_table).await?;

        Ok(())
    }
}
