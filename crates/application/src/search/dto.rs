use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub r#type: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchResultDto {
    pub items: Vec<SearchResultItemDto>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum SearchResultItemDto {
    Party {
        id: Uuid,
        display_name: String,
        party_type: String,
        verification_status: String,
        trust_score: f64,
        primary_domain_id: Option<Uuid>,
        is_active: bool,
        created_at: time::OffsetDateTime,
    },
    Resource {
        id: Uuid,
        owner_party_id: Uuid,
        category_id: Uuid,
        title: String,
        description: Option<String>,
        is_active: bool,
        verified_by_platform: bool,
        created_at: time::OffsetDateTime,
    },
    Need {
        id: Uuid,
        owner_party_id: Uuid,
        category_id: Uuid,
        title: String,
        description: Option<String>,
        is_active: bool,
        verified_by_platform: bool,
        created_at: time::OffsetDateTime,
    },
    Enhancement {
        id: Uuid,
        owner_party_id: Uuid,
        category_id: Uuid,
        title: String,
        description: Option<String>,
        is_active: bool,
        verified_by_platform: bool,
        created_at: time::OffsetDateTime,
    },
    Deal {
        id: Uuid,
        deal_reference: String,
        deal_title: String,
        deal_description: Option<String>,
        deal_status: String,
        domain_category_id: Uuid,
        initiator_party_id: Uuid,
        is_public: bool,
        created_at: time::OffsetDateTime,
    },
}
