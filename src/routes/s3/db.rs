use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "s3_assets")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub experiment_id: Option<Uuid>,
    #[sea_orm(column_type = "Text")]
    pub original_filename: String,
    #[sea_orm(column_type = "Text", unique)]
    pub s3_key: String,
    pub size_bytes: Option<i64>,
    #[sea_orm(column_type = "Text", nullable)]
    pub uploaded_by: Option<String>,
    pub uploaded_at: DateTimeWithTimeZone,
    pub is_deleted: bool,
    #[sea_orm(column_type = "Text")]
    pub r#type: String,
    #[sea_orm(column_type = "Text", nullable)]
    pub role: Option<String>,
    pub last_updated: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::routes::experiments::db::Entity",
        from = "Column::ExperimentId",
        to = "crate::routes::experiments::db::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    Experiments,
}

impl Related<crate::routes::experiments::db::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Experiments.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
