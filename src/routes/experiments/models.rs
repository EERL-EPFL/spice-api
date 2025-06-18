use async_trait::async_trait;
use chrono::{DateTime, Utc};
use crudcrate::{CRUDResource, ToCreateModel, ToUpdateModel, traits::MergeIntoActiveModel};
use sea_orm::{
    ActiveModelTrait, ActiveValue, Condition, ConnectionTrait, DatabaseConnection, EntityTrait,
    Order, QueryOrder, QuerySelect, TransactionTrait, entity::prelude::*,
};
use serde::{Deserialize, Serialize};
use spice_entity::experiments::Model;
use utoipa::ToSchema;
use uuid::Uuid;

// Utility function to convert coordinate string (e.g., "A1") to x,y coordinates
fn parse_coordinate(coord: &str) -> Option<(i16, i16)> {
    if coord.is_empty() {
        return None;
    }

    let mut chars = coord.chars();
    let col_char = chars.next()?;
    let row_str: String = chars.collect();

    // Convert column letter to number (A=1, B=2, etc.)
    let col = i16::from(col_char.to_ascii_uppercase() as u8 - b'A' + 1);
    let row = row_str.parse::<i16>().ok()?;

    Some((col, row))
}

// Convert regions input to active models
fn create_region_active_models(
    experiment_id: Uuid,
    regions: Vec<RegionInput>,
    _db: &impl ConnectionTrait,
) -> Vec<spice_entity::regions::ActiveModel> {
    let mut active_models = Vec::new();

    for region in regions {
        let (upper_left_x, upper_left_y) = region
            .upper_left
            .as_ref()
            .and_then(|s| parse_coordinate(s))
            .map_or((None, None), |(x, y)| (Some(x), Some(y)));

        let (lower_right_x, lower_right_y) = region
            .lower_right
            .as_ref()
            .and_then(|s| parse_coordinate(s))
            .map_or((None, None), |(x, y)| (Some(x), Some(y)));

        let dilution_factor = region.dilution.as_ref().and_then(|s| s.parse::<i16>().ok());

        let active_model = spice_entity::regions::ActiveModel {
            id: ActiveValue::Set(Uuid::new_v4()),
            experiment_id: ActiveValue::Set(experiment_id),
            treatment_id: ActiveValue::Set(region.treatment_id),
            name: ActiveValue::Set(region.name),
            display_colour_hex: ActiveValue::Set(region.color),
            tray_id: ActiveValue::Set(None), // TODO: Look up tray by name if needed
            upper_left_corner_x: ActiveValue::Set(upper_left_x),
            upper_left_corner_y: ActiveValue::Set(upper_left_y),
            lower_right_corner_x: ActiveValue::Set(lower_right_x),
            lower_right_corner_y: ActiveValue::Set(lower_right_y),
            dilution_factor: ActiveValue::Set(dilution_factor),
            is_background_key: ActiveValue::Set(region.is_background_key.unwrap_or(false)),
            created_at: ActiveValue::Set(chrono::Utc::now().into()),
            last_updated: ActiveValue::Set(chrono::Utc::now().into()),
        };

        active_models.push(active_model);
    }

    active_models
}

// Fetch treatment information with sample and campaign data
async fn fetch_treatment_info(
    treatment_id: Uuid,
    db: &impl ConnectionTrait,
) -> Result<Option<TreatmentInfo>, DbErr> {
    let treatment = spice_entity::treatments::Entity::find_by_id(treatment_id)
        .one(db)
        .await?;

    if let Some(treatment) = treatment {
        let sample_info = if let Some(sample_id) = treatment.sample_id {
            let sample = spice_entity::samples::Entity::find_by_id(sample_id)
                .one(db)
                .await?;

            if let Some(sample) = sample {
                let campaign_info = if let Some(campaign_id) = sample.campaign_id {
                    let campaign = spice_entity::campaign::Entity::find_by_id(campaign_id)
                        .one(db)
                        .await?;

                    campaign.map(|c| CampaignInfo {
                        id: c.id,
                        name: c.name,
                    })
                } else {
                    None
                };

                Some(SampleInfo {
                    id: sample.id,
                    name: sample.name,
                    campaign: campaign_info,
                })
            } else {
                None
            }
        } else {
            None
        };

        Ok(Some(TreatmentInfo {
            id: treatment.id,
            name: treatment.name,
            notes: treatment.notes,
            enzyme_volume_litres: treatment.enzyme_volume_litres,
            sample: sample_info,
        }))
    } else {
        Ok(None)
    }
}

// Convert region model back to RegionInput for response
async fn region_model_to_input_with_treatment(
    region: spice_entity::regions::Model,
    db: &impl ConnectionTrait,
) -> Result<RegionInput, DbErr> {
    let upper_left = match (region.upper_left_corner_x, region.upper_left_corner_y) {
        (Some(x), Some(y)) if x > 0 && x <= 26 => {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let col_char = (b'A' + (x - 1) as u8) as char;
            Some(format!("{col_char}{y}"))
        }
        _ => None,
    };

    let lower_right = match (region.lower_right_corner_x, region.lower_right_corner_y) {
        (Some(x), Some(y)) if x > 0 && x <= 26 => {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let col_char = (b'A' + (x - 1) as u8) as char;
            Some(format!("{col_char}{y}"))
        }
        _ => None,
    };

    let treatment_info = if let Some(treatment_id) = region.treatment_id {
        fetch_treatment_info(treatment_id, db).await?
    } else {
        None
    };

    Ok(RegionInput {
        name: region.name,
        tray_name: None, // TODO: Look up tray name by tray_id if needed
        upper_left,
        lower_right,
        color: region.display_colour_hex,
        dilution: region.dilution_factor.map(|d| d.to_string()),
        treatment_id: region.treatment_id,
        is_background_key: Some(region.is_background_key),
        treatment: treatment_info,
    })
}

#[derive(ToSchema, Serialize, Deserialize, Clone)]
pub struct TreatmentInfo {
    pub id: Uuid,
    pub name: Option<String>,
    pub notes: Option<String>,
    pub enzyme_volume_litres: Option<Decimal>,
    pub sample: Option<SampleInfo>,
}

#[derive(ToSchema, Serialize, Deserialize, Clone)]
pub struct SampleInfo {
    pub id: Uuid,
    pub name: String,
    pub campaign: Option<CampaignInfo>,
}

#[derive(ToSchema, Serialize, Deserialize, Clone)]
pub struct CampaignInfo {
    pub id: Uuid,
    pub name: String,
}

#[derive(ToSchema, Serialize, Deserialize, Clone)]
pub struct RegionInput {
    pub name: Option<String>,
    pub tray_name: Option<String>,
    pub upper_left: Option<String>,  // e.g., "A1"
    pub lower_right: Option<String>, // e.g., "C5"
    pub color: Option<String>,       // hex color
    pub dilution: Option<String>,
    pub treatment_id: Option<Uuid>,
    pub is_background_key: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub treatment: Option<TreatmentInfo>,
}

#[derive(ToSchema, Serialize, Deserialize, Clone)]
struct TrayRegions {
    pub id: Uuid,
    pub experiment_id: Uuid,
    pub treatment_id: Option<Uuid>,
    pub name: Option<String>,
    pub display_colour_hex: Option<String>,
    pub tray_id: Option<i16>,
    pub upper_left_corner_x: Option<i16>,
    pub upper_left_corner_y: Option<i16>,
    pub lower_right_corner_x: Option<i16>,
    pub lower_right_corner_y: Option<i16>,
    pub dilution_factor: Option<i16>,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
}

impl From<spice_entity::regions::Model> for TrayRegions {
    fn from(region: spice_entity::regions::Model) -> Self {
        Self {
            id: region.id,
            experiment_id: region.experiment_id,
            treatment_id: region.treatment_id,
            name: region.name,
            display_colour_hex: region.display_colour_hex,
            tray_id: region.tray_id,
            upper_left_corner_x: region.upper_left_corner_x,
            upper_left_corner_y: region.upper_left_corner_y,
            lower_right_corner_x: region.lower_right_corner_x,
            lower_right_corner_y: region.lower_right_corner_y,
            dilution_factor: region.dilution_factor,
            created_at: region.created_at.into(),
            last_updated: region.last_updated.into(),
        }
    }
}

#[derive(ToSchema, Serialize, Deserialize, ToUpdateModel, ToCreateModel, Clone)]
#[active_model = "spice_entity::experiments::ActiveModel"]
pub struct Experiment {
    #[crudcrate(update_model = false, create_model = false, on_create = Uuid::new_v4())]
    id: Uuid,
    name: String,
    // sample_id: Uuid,
    tray_configuration_id: Option<Uuid>,
    username: Option<String>,
    performed_at: Option<DateTime<Utc>>,
    #[crudcrate(update_model = false, create_model = false, on_create = chrono::Utc::now())]
    created_at: DateTime<Utc>,
    #[crudcrate(update_model = false, create_model = false, on_update = chrono::Utc::now(), on_create = chrono::Utc::now())]
    last_updated: DateTime<Utc>,
    temperature_ramp: Option<Decimal>,
    temperature_start: Option<Decimal>,
    temperature_end: Option<Decimal>,
    is_calibration: bool,
    remarks: Option<String>,
    #[crudcrate(non_db_attr = true, default = vec![])]
    assets: Vec<crate::routes::assets::models::Asset>,
    #[crudcrate(non_db_attr = true, default = vec![])]
    regions: Vec<RegionInput>,
}

impl From<Model> for Experiment {
    fn from(model: Model) -> Self {
        Self {
            id: model.id,
            name: model.name,
            tray_configuration_id: model.tray_configuration_id,
            // sample_id: model.sample_id,
            username: model.username,
            performed_at: model.performed_at.map(|dt| dt.with_timezone(&Utc)),
            created_at: model.created_at.into(),
            last_updated: model.last_updated.into(),
            temperature_ramp: model.temperature_ramp,
            temperature_start: model.temperature_start,
            temperature_end: model.temperature_end,
            is_calibration: model.is_calibration,
            remarks: model.remarks,
            assets: vec![],
            regions: vec![],
        }
    }
}

#[async_trait]
impl CRUDResource for Experiment {
    type EntityType = spice_entity::experiments::Entity;
    type ColumnType = spice_entity::experiments::Column;
    type ActiveModelType = spice_entity::experiments::ActiveModel;
    type CreateModel = ExperimentCreate;
    type UpdateModel = ExperimentUpdate;

    const ID_COLUMN: Self::ColumnType = spice_entity::experiments::Column::Id;
    const RESOURCE_NAME_PLURAL: &'static str = "experiments";
    const RESOURCE_NAME_SINGULAR: &'static str = "experiment";
    const RESOURCE_DESCRIPTION: &'static str =
        "This resource manages experiments associated with sample data collected during campaigns.";

    async fn get_one(db: &DatabaseConnection, id: Uuid) -> Result<Self, DbErr> {
        let model =
            Self::EntityType::find_by_id(id)
                .one(db)
                .await?
                .ok_or(DbErr::RecordNotFound(format!(
                    "{} not found",
                    Self::RESOURCE_NAME_SINGULAR
                )))?;

        let s3_assets = model
            .find_related(spice_entity::s3_assets::Entity)
            .all(db)
            .await?;

        let regions = model
            .find_related(spice_entity::regions::Entity)
            .all(db)
            .await?;

        let mut regions_with_treatment = Vec::new();
        for region in regions {
            let region_input = region_model_to_input_with_treatment(region, db).await?;
            regions_with_treatment.push(region_input);
        }

        let mut model: Self = model.into();
        model.assets = s3_assets.into_iter().map(Into::into).collect();
        model.regions = regions_with_treatment;

        Ok(model)
    }

    async fn create(db: &DatabaseConnection, data: Self::CreateModel) -> Result<Self, DbErr> {
        let txn = db.begin().await?;

        // Store regions before conversion since they're not part of the DB model
        let regions_to_create = data.regions.clone();

        // Create the experiment first
        let experiment_model: Self::ActiveModelType = data.into();
        let experiment = experiment_model.insert(&txn).await?;

        // Handle regions if provided
        if !regions_to_create.is_empty() {
            let region_models = create_region_active_models(experiment.id, regions_to_create, &txn);

            for region_model in region_models {
                region_model.insert(&txn).await?;
            }
        }

        txn.commit().await?;

        // Return the complete experiment with regions
        Self::get_one(db, experiment.id).await
    }

    async fn update(
        db: &DatabaseConnection,
        id: Uuid,
        update_data: Self::UpdateModel,
    ) -> Result<Self, DbErr> {
        let txn = db.begin().await?;

        let existing: Self::ActiveModelType = Self::EntityType::find_by_id(id)
            .one(&txn)
            .await?
            .ok_or(DbErr::RecordNotFound(format!(
                "{} not found",
                Self::RESOURCE_NAME_PLURAL
            )))?
            .into();

        let updated_model = update_data.clone().merge_into_activemodel(existing);
        let _updated = updated_model.update(&txn).await?;

        // Handle regions update - delete existing regions and create new ones
        if !update_data.regions.is_empty() {
            // Delete existing regions for this experiment
            spice_entity::regions::Entity::delete_many()
                .filter(spice_entity::regions::Column::ExperimentId.eq(id))
                .exec(&txn)
                .await?;

            // Create new regions
            let region_models = create_region_active_models(id, update_data.regions.clone(), &txn);

            for region_model in region_models {
                region_model.insert(&txn).await?;
            }
        }

        txn.commit().await?;

        // Return the complete experiment with regions
        Self::get_one(db, id).await
    }

    async fn get_all(
        db: &DatabaseConnection,
        condition: Condition,
        order_column: Self::ColumnType,
        order_direction: Order,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<Self>, DbErr> {
        let models = Self::EntityType::find()
            .filter(condition)
            .order_by(order_column, order_direction)
            .offset(offset)
            .limit(limit)
            .all(db)
            .await?;

        let mut experiments = Vec::new();

        for model in models {
            let s3_assets = model
                .find_related(spice_entity::s3_assets::Entity)
                .all(db)
                .await?;

            let regions = model
                .find_related(spice_entity::regions::Entity)
                .all(db)
                .await?;

            let mut regions_with_treatment = Vec::new();
            for region in regions {
                let region_input = region_model_to_input_with_treatment(region, db).await?;
                regions_with_treatment.push(region_input);
            }

            let mut experiment: Self = model.into();
            experiment.assets = s3_assets.into_iter().map(Into::into).collect();
            experiment.regions = regions_with_treatment;

            experiments.push(experiment);
        }

        Ok(experiments)
    }

    fn sortable_columns() -> Vec<(&'static str, Self::ColumnType)> {
        vec![
            ("id", Self::ColumnType::Id),
            ("name", Self::ColumnType::Name),
            ("performed_at", Self::ColumnType::PerformedAt),
            ("username", Self::ColumnType::Username),
            ("created_at", Self::ColumnType::CreatedAt),
            ("temperature_ramp", Self::ColumnType::TemperatureRamp),
            ("temperature_start", Self::ColumnType::TemperatureStart),
            ("temperature_end", Self::ColumnType::TemperatureEnd),
        ]
    }

    fn filterable_columns() -> Vec<(&'static str, Self::ColumnType)> {
        vec![
            ("name", Self::ColumnType::Name),
            ("performed_at", Self::ColumnType::PerformedAt),
            ("username", Self::ColumnType::Username),
            ("created_at", Self::ColumnType::CreatedAt),
            ("temperature_ramp", Self::ColumnType::TemperatureRamp),
            ("temperature_start", Self::ColumnType::TemperatureStart),
            ("temperature_end", Self::ColumnType::TemperatureEnd),
        ]
    }
}
