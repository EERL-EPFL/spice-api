use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add a new numeric row_number column
        manager
            .alter_table(
                Table::alter()
                    .table(Wells::Table)
                    .add_column(ColumnDef::new(Wells::RowNumber).integer().null())
                    .to_owned(),
            )
            .await?;

        // Update existing data: convert row_label to numeric values
        // This assumes row labels are like "A", "B", "C", etc.
        // A=1, B=2, C=3, etc.
        let db = manager.get_connection();
        db.execute_unprepared(
            r#"
            UPDATE wells 
            SET row_number = CASE 
                WHEN LENGTH(row_label) = 1 THEN ASCII(UPPER(row_label)) - ASCII('A') + 1
                ELSE NULL
            END
            WHERE row_label IS NOT NULL
            "#,
        )
        .await?;

        // Make row_number NOT NULL after data migration
        manager
            .alter_table(
                Table::alter()
                    .table(Wells::Table)
                    .modify_column(ColumnDef::new(Wells::RowNumber).integer().not_null())
                    .to_owned(),
            )
            .await?;

        // Drop the old row_label column
        manager
            .alter_table(
                Table::alter()
                    .table(Wells::Table)
                    .drop_column(Wells::RowLabel)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add back the row_label column
        manager
            .alter_table(
                Table::alter()
                    .table(Wells::Table)
                    .add_column(ColumnDef::new(Wells::RowLabel).string().null())
                    .to_owned(),
            )
            .await?;

        // Convert numeric row_number back to letters
        let db = manager.get_connection();
        db.execute_unprepared(
            r#"
            UPDATE wells 
            SET row_label = CHR(row_number + ASCII('A') - 1)
            WHERE row_number IS NOT NULL AND row_number BETWEEN 1 AND 26
            "#,
        )
        .await?;

        // Make row_label NOT NULL after data migration
        manager
            .alter_table(
                Table::alter()
                    .table(Wells::Table)
                    .modify_column(ColumnDef::new(Wells::RowLabel).string().not_null())
                    .to_owned(),
            )
            .await?;

        // Drop the row_number column
        manager
            .alter_table(
                Table::alter()
                    .table(Wells::Table)
                    .drop_column(Wells::RowNumber)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Wells {
    Table,
    RowLabel,
    RowNumber,
}
