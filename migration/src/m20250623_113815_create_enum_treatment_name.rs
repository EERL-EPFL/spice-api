use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Create the treatment_name enum type using raw SQL
        db.execute_unprepared("CREATE TYPE treatment_name AS ENUM ('none', 'heat', 'h2o2')")
            .await?;

        // Add a temporary column with the enum type
        manager
            .alter_table(
                Table::alter()
                    .table(Treatments::Table)
                    .add_column(
                        ColumnDef::new(Treatments::NameEnum)
                            .custom(Alias::new("treatment_name"))
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Convert existing string values to enum values (case insensitive)
        db.execute_unprepared(
            r#"
            UPDATE treatments 
            SET name_enum = CASE 
                WHEN LOWER(name) = 'none' THEN 'none'::treatment_name
                WHEN LOWER(name) = 'heat' THEN 'heat'::treatment_name
                WHEN LOWER(name) = 'h2o2' THEN 'h2o2'::treatment_name
                ELSE 'none'::treatment_name
            END
            WHERE name IS NOT NULL;
            "#,
        )
        .await?;

        // Drop the old text column
        manager
            .alter_table(
                Table::alter()
                    .table(Treatments::Table)
                    .drop_column(Treatments::Name)
                    .to_owned(),
            )
            .await?;

        // Rename the enum column to 'name'
        manager
            .alter_table(
                Table::alter()
                    .table(Treatments::Table)
                    .rename_column(Treatments::NameEnum, Treatments::Name)
                    .to_owned(),
            )
            .await?;

        // Set the "name" column to NOT NULL
        manager
            .alter_table(
                Table::alter()
                    .table(Treatments::Table)
                    .modify_column(
                        ColumnDef::new(Treatments::Name)
                            .custom(Alias::new("treatment_name"))
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Add back the text column
        manager
            .alter_table(
                Table::alter()
                    .table(Treatments::Table)
                    .add_column(ColumnDef::new(Treatments::NameText).text().null())
                    .to_owned(),
            )
            .await?;

        // Convert enum values back to text
        db.execute_unprepared(
            r#"
            UPDATE treatments 
            SET name_text = name::text
            WHERE name IS NOT NULL;
            "#,
        )
        .await?;

        // Drop the enum column
        manager
            .alter_table(
                Table::alter()
                    .table(Treatments::Table)
                    .drop_column(Treatments::Name)
                    .to_owned(),
            )
            .await?;

        // Rename the text column back to 'name'
        manager
            .alter_table(
                Table::alter()
                    .table(Treatments::Table)
                    .rename_column(Treatments::NameText, Treatments::Name)
                    .to_owned(),
            )
            .await?;

        // Drop the enum type
        db.execute_unprepared("DROP TYPE IF EXISTS treatment_name")
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Treatments {
    Table,
    Name,
    NameEnum,
    NameText,
}
