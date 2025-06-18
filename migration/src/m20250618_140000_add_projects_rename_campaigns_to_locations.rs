use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // First, create the projects table
        manager
            .create_table(
                Table::create()
                    .table(Projects::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Projects::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT uuid_generate_v4()".to_owned()),
                    )
                    .col(ColumnDef::new(Projects::Name).string().not_null())
                    .col(ColumnDef::new(Projects::Note).text())
                    .col(ColumnDef::new(Projects::Colour).string())
                    .col(
                        ColumnDef::new(Projects::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()".to_owned()),
                    )
                    .col(
                        ColumnDef::new(Projects::LastUpdated)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()".to_owned()),
                    )
                    .to_owned(),
            )
            .await?;

        // Add unique constraint on project name
        manager
            .create_index(
                Index::create()
                    .name("projects_name_key")
                    .table(Projects::Table)
                    .col(Projects::Name)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Add indexes for projects table
        manager
            .create_index(
                Index::create()
                    .name("idx_projects_id")
                    .table(Projects::Table)
                    .col(Projects::Id)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_projects_name")
                    .table(Projects::Table)
                    .col(Projects::Name)
                    .to_owned(),
            )
            .await?;

        // Rename campaign table to locations
        manager
            .rename_table(
                Table::rename()
                    .table(Alias::new("campaign"), Locations::Table)
                    .to_owned(),
            )
            .await?;

        // Add project_id column to locations table
        manager
            .alter_table(
                Table::alter()
                    .table(Locations::Table)
                    .add_column(ColumnDef::new(Locations::ProjectId).uuid())
                    .to_owned(),
            )
            .await?;

        // Rename existing campaign indexes to location indexes
        manager
            .drop_index(Index::drop().name("idx_campaign_id").to_owned())
            .await?;

        manager
            .drop_index(Index::drop().name("idx_campaign_name").to_owned())
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_locations_id")
                    .table(Locations::Table)
                    .col(Locations::Id)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_locations_name")
                    .table(Locations::Table)
                    .col(Locations::Name)
                    .to_owned(),
            )
            .await?;

        // Add index for project_id in locations
        manager
            .create_index(
                Index::create()
                    .name("idx_locations_project_id")
                    .table(Locations::Table)
                    .col(Locations::ProjectId)
                    .to_owned(),
            )
            .await?;

        // Add foreign key constraint for project_id in locations
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_locations_project_id")
                    .from(Locations::Table, Locations::ProjectId)
                    .to(Projects::Table, Projects::Id)
                    .on_delete(ForeignKeyAction::SetNull)
                    .on_update(ForeignKeyAction::NoAction)
                    .to_owned(),
            )
            .await?;

        // Update the existing foreign key constraint in samples table
        // First drop the old constraint
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("samples_campaign_id_fkey")
                    .table(Samples::Table)
                    .to_owned(),
            )
            .await?;

        // Rename the campaign_id column to location_id in samples table
        manager
            .alter_table(
                Table::alter()
                    .table(Samples::Table)
                    .rename_column(Alias::new("campaign_id"), Samples::LocationId)
                    .to_owned(),
            )
            .await?;

        // Drop old index and create new one
        manager
            .drop_index(Index::drop().name("idx_samples_campaign_id").to_owned())
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_samples_location_id")
                    .table(Samples::Table)
                    .col(Samples::LocationId)
                    .to_owned(),
            )
            .await?;

        // Add new foreign key constraint
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("samples_location_id_fkey")
                    .from(Samples::Table, Samples::LocationId)
                    .to(Locations::Table, Locations::Id)
                    .on_delete(ForeignKeyAction::NoAction)
                    .on_update(ForeignKeyAction::NoAction)
                    .to_owned(),
            )
            .await?;

        // Rename the unique constraint on campaign name to locations name
        // Use raw SQL to drop the constraint
        manager
            .get_connection()
            .execute_unprepared("ALTER TABLE locations DROP CONSTRAINT campaign_name_key")
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("locations_name_key")
                    .table(Locations::Table)
                    .col(Locations::Name)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Reverse all the changes

        // Drop foreign key constraint from locations to projects
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_locations_project_id")
                    .table(Locations::Table)
                    .to_owned(),
            )
            .await?;

        // Drop foreign key constraint from samples to locations
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("samples_location_id_fkey")
                    .table(Samples::Table)
                    .to_owned(),
            )
            .await?;

        // Rename location_id back to campaign_id in samples
        manager
            .alter_table(
                Table::alter()
                    .table(Samples::Table)
                    .rename_column(Samples::LocationId, Alias::new("campaign_id"))
                    .to_owned(),
            )
            .await?;

        // Drop location indexes and recreate campaign indexes
        manager
            .drop_index(Index::drop().name("idx_samples_location_id").to_owned())
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_samples_campaign_id")
                    .table(Samples::Table)
                    .col(Alias::new("campaign_id"))
                    .to_owned(),
            )
            .await?;

        // Remove project_id column from locations
        manager
            .alter_table(
                Table::alter()
                    .table(Locations::Table)
                    .drop_column(Locations::ProjectId)
                    .to_owned(),
            )
            .await?;

        // Rename locations table back to campaign
        manager
            .rename_table(
                Table::rename()
                    .table(Locations::Table, Alias::new("campaign"))
                    .to_owned(),
            )
            .await?;

        // Drop location indexes and recreate campaign indexes
        manager
            .drop_index(Index::drop().name("idx_locations_id").to_owned())
            .await?;

        manager
            .drop_index(Index::drop().name("idx_locations_name").to_owned())
            .await?;

        manager
            .drop_index(Index::drop().name("locations_name_key").to_owned())
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_campaign_id")
                    .table(Alias::new("campaign"))
                    .col(Alias::new("id"))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_campaign_name")
                    .table(Alias::new("campaign"))
                    .col(Alias::new("name"))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("campaign_name_key")
                    .table(Alias::new("campaign"))
                    .col(Alias::new("name"))
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Recreate the original foreign key constraint
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("samples_campaign_id_fkey")
                    .from(Samples::Table, Alias::new("campaign_id"))
                    .to(Alias::new("campaign"), Alias::new("id"))
                    .on_delete(ForeignKeyAction::NoAction)
                    .on_update(ForeignKeyAction::NoAction)
                    .to_owned(),
            )
            .await?;

        // Drop the projects table
        manager
            .drop_table(Table::drop().table(Projects::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Projects {
    Table,
    Id,
    Name,
    Note,
    Colour,
    CreatedAt,
    LastUpdated,
}

#[derive(DeriveIden)]
enum Locations {
    Table,
    Id,
    Name,
    ProjectId,
}

#[derive(DeriveIden)]
enum Samples {
    Table,
    LocationId,
}
