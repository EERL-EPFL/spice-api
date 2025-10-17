use crate::treatments::models::TreatmentList;
use chrono::{DateTime, Utc};
use crudcrate::{CRUDResource, EntityToModels, traits::MergeIntoActiveModel};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryOrder, QuerySelect,
    entity::prelude::*,
};
// Import after EntityToModels to avoid conflicts
use uuid::Uuid;

#[derive(
    Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, ToSchema, Serialize, Deserialize,
)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "sample_type")]
#[serde(rename_all = "snake_case")]
pub enum SampleType {
    #[sea_orm(string_value = "bulk")]
    Bulk,
    #[sea_orm(string_value = "filter")]
    Filter,
    #[sea_orm(string_value = "blank")]
    Blank,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, EntityToModels)]
#[sea_orm(table_name = "samples")]
#[crudcrate(
    generate_router,
    api_struct = "Sample",
    name_singular = "sample",
    name_plural = "samples",
    description = "This resource manages samples associated with experiments.",
    fn_get_one = get_one_sample,
    fn_create = create_sample_with_treatments,
    fn_update = update_sample_with_treatments,
    fn_get_all = get_all_samples,
)]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    #[crudcrate(primary_key, update_model = false, create_model = false, on_create = Uuid::new_v4())]
    pub id: Uuid,
    #[sea_orm(column_type = "Text")]
    #[crudcrate(sortable, filterable, fulltext)]
    pub name: String,
    #[crudcrate(sortable, filterable, enum_field)]
    pub r#type: SampleType,
    #[crudcrate(sortable)]
    pub start_time: Option<DateTime<Utc>>,
    #[crudcrate(sortable)]
    pub stop_time: Option<DateTime<Utc>>,
    #[sea_orm(column_type = "Decimal(Some((16, 10)))", nullable)]
    #[crudcrate(sortable, filterable)]
    pub flow_litres_per_minute: Option<Decimal>,
    #[sea_orm(column_type = "Decimal(Some((16, 10)))", nullable)]
    #[crudcrate(sortable, filterable)]
    pub total_volume: Option<Decimal>,
    #[sea_orm(column_type = "Text", nullable)]
    #[crudcrate(sortable, filterable, fulltext)]
    pub material_description: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    #[crudcrate(sortable, filterable, fulltext)]
    pub extraction_procedure: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    #[crudcrate(sortable, filterable, fulltext)]
    pub filter_substrate: Option<String>,
    #[crudcrate(sortable, filterable)]
    pub suspension_volume_litres: Option<Decimal>,
    #[crudcrate(sortable, filterable)]
    pub air_volume_litres: Option<Decimal>,
    #[crudcrate(sortable, filterable)]
    pub water_volume_litres: Option<Decimal>,
    #[crudcrate(sortable, filterable)]
    pub initial_concentration_gram_l: Option<Decimal>,
    #[crudcrate(sortable, filterable)]
    pub well_volume_litres: Option<Decimal>,
    #[sea_orm(column_type = "Text", nullable)]
    #[crudcrate(sortable, filterable, fulltext)]
    pub remarks: Option<String>,
    #[sea_orm(column_type = "Decimal(Some((9, 6)))", nullable)]
    #[crudcrate(sortable)]
    pub longitude: Option<Decimal>,
    #[sea_orm(column_type = "Decimal(Some((9, 6)))", nullable)]
    #[crudcrate(sortable)]
    pub latitude: Option<Decimal>,
    #[crudcrate(sortable, filterable)]
    pub location_id: Option<Uuid>,
    #[crudcrate(update_model = false, create_model = false, on_create = chrono::Utc::now(), sortable, list_model=false)]
    pub created_at: DateTime<Utc>,
    #[crudcrate(update_model = false, create_model = false, on_update = chrono::Utc::now(), on_create = chrono::Utc::now(), sortable)]
    pub last_updated: DateTime<Utc>,
    #[sea_orm(ignore)]
    #[crudcrate(non_db_attr = true, default = vec![], use_target_models)]
    pub treatments: Vec<crate::treatments::models::Treatment>,
    #[sea_orm(ignore)]
    #[crudcrate(non_db_attr = true, default = None, list_model=false)]
    pub location: Option<crate::locations::models::Location>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::locations::models::Entity",
        from = "Column::LocationId",
        to = "crate::locations::models::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    Locations,
    #[sea_orm(has_many = "crate::treatments::models::Entity")]
    Treatments,
}

impl Related<crate::locations::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Locations.def()
    }
}

impl Related<crate::treatments::models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Treatments.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

async fn get_one_sample(db: &DatabaseConnection, id: Uuid) -> Result<Sample, DbErr> {
    let model = Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| DbErr::RecordNotFound("Sample not found".to_string()))?;

    let treatment_models = model
        .find_related(crate::treatments::models::Entity)
        .all(db)
        .await?;

    let all_experimental_results =
        super::services::fetch_experimental_results_for_sample(db, id).await?;

    let mut treatments_with_results = Vec::new();

    for treatment_model in treatment_models {
        let treatment = super::services::treatment_to_treatment_with_results(
            treatment_model,
            id,
            &all_experimental_results,
            db,
        );
        treatments_with_results.push(treatment);
    }

    let mut sample: Sample = model.into();
    sample.treatments = treatments_with_results;

    Ok(sample)
}

async fn get_all_samples(
    db: &DatabaseConnection,
    condition: &sea_orm::Condition,
    order_column: Column,
    order_direction: sea_orm::Order,
    offset: u64,
    limit: u64,
) -> Result<Vec<SampleList>, DbErr> {
    let models = Entity::find()
        .filter(condition.clone())
        .order_by(order_column, order_direction)
        .offset(offset)
        .limit(limit)
        .all(db)
        .await?;
    // For each sample, fetch related treatments and convert to TreatmentList
    let mut samples: Vec<SampleList> = Vec::new();

    for model in models {
        // Fetch related treatments for this sample
        let treatment_models = model
            .find_related(crate::treatments::models::Entity)
            .all(db)
            .await?;

        let treatment_lists: Vec<TreatmentList> = treatment_models
            .into_iter()
            .map(TreatmentList::from)
            .collect();

        // Convert model to SampleList and attach treatments
        let mut sample_list = SampleList::from(model);
        sample_list.treatments = treatment_lists;
        samples.push(sample_list);
    }

    Ok(samples)
}

async fn create_sample_with_treatments(
    db: &DatabaseConnection,
    create_data: SampleCreate,
) -> Result<Sample, DbErr> {
    // Extract treatments before creating sample
    let treatments_to_create = if create_data.treatments.is_empty() {
        None
    } else {
        Some(create_data.treatments.clone())
    };

    // Use the auto-generated default create logic by creating ActiveModel directly
    let active_model: ActiveModel = create_data.into();
    let inserted = active_model.insert(db).await?;
    let sample_id = inserted.id;

    // Create treatments using CRUDResource methods
    if let Some(treatments) = treatments_to_create {
        for treatment_create in treatments {
            let mut treatment_with_sample = treatment_create;
            treatment_with_sample.sample_id = Some(sample_id);
            let _ = crate::treatments::models::Treatment::create(db, treatment_with_sample).await?;
        }
    }

    // Return the created sample with treatments loaded
    Sample::get_one(db, sample_id).await
}

async fn update_sample_with_treatments(
    db: &DatabaseConnection,
    id: Uuid,
    update_data: SampleUpdate,
) -> Result<Sample, DbErr> {
    // Extract treatments before updating sample (always process treatments, even if empty to handle deletions)
    let treatments_to_update = Some(update_data.treatments.clone());

    // Update the sample using the proper CRUDResource pattern to avoid infinite recursion
    // First get the existing model, then use merge_into_activemodel like the default CRUDResource::update
    let existing_model = Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| DbErr::RecordNotFound("Sample not found".to_string()))?;

    let existing_active: ActiveModel = existing_model.into_active_model();
    let updated_active_model = update_data.merge_into_activemodel(existing_active)?;
    let _updated_sample = updated_active_model.update(db).await?;

    // Handle complete treatment list replacement: create new, update existing, delete missing
    if let Some(treatments) = treatments_to_update {
        // Get current treatment IDs for this sample
        let current_treatments = crate::treatments::models::Entity::find()
            .filter(crate::treatments::models::Column::SampleId.eq(id))
            .all(db)
            .await?;
        let current_treatment_ids: Vec<Uuid> = current_treatments.iter().map(|t| t.id).collect();

        // Track which treatments are being updated
        let mut updated_treatment_ids = Vec::new();

        for treatment_update in treatments {
            if let Some(Some(treatment_id)) = treatment_update.id {
                // Update existing treatment
                let existing_treatment =
                    crate::treatments::models::Entity::find_by_id(treatment_id)
                        .one(db)
                        .await?
                        .ok_or_else(|| DbErr::RecordNotFound("Treatment not found".to_string()))?;

                let existing_treatment_active = existing_treatment.into_active_model();
                let updated_treatment_active =
                    treatment_update.merge_into_activemodel(existing_treatment_active)?;
                let _ = updated_treatment_active.update(db).await?;
                updated_treatment_ids.push(treatment_id);
            } else {
                // Create new treatment (following the same pattern as create_sample_with_treatments)
                let treatment_create = crate::treatments::models::TreatmentCreate {
                    name: treatment_update
                        .name
                        .flatten()
                        .unwrap_or(crate::treatments::models::TreatmentName::None),
                    notes: treatment_update.notes.flatten(),
                    sample_id: Some(id),
                    enzyme_volume_litres: treatment_update.enzyme_volume_litres.flatten(),
                };
                let new_treatment =
                    crate::treatments::models::Treatment::create(db, treatment_create).await?;
                updated_treatment_ids.push(new_treatment.id);
            }
        }

        // Delete treatments that are no longer in the update list
        let treatments_to_delete: Vec<Uuid> = current_treatment_ids
            .into_iter()
            .filter(|id| !updated_treatment_ids.contains(id))
            .collect();

        if !treatments_to_delete.is_empty() {
            crate::treatments::models::Entity::delete_many()
                .filter(crate::treatments::models::Column::Id.is_in(treatments_to_delete))
                .exec(db)
                .await?;
        }
    }

    // Return the updated sample with treatments loaded
    Sample::get_one(db, id).await
}
