use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Deserialize, validator::Validate)]
pub struct CreateReviewRequest {
    #[serde(rename = "reviewedPartyId")]
    pub reviewed_party_id: Uuid,
    #[serde(rename = "overallRating")]
    #[validate(range(min = 1, max = 5))]
    pub overall_rating: i32,
    #[serde(rename = "communicationRating")]
    #[validate(range(min = 1, max = 5))]
    pub communication_rating: Option<i32>,
    #[serde(rename = "reliabilityRating")]
    #[validate(range(min = 1, max = 5))]
    pub reliability_rating: Option<i32>,
    #[serde(rename = "qualityRating")]
    #[validate(range(min = 1, max = 5))]
    pub quality_rating: Option<i32>,
    #[serde(rename = "timelinessRating")]
    #[validate(range(min = 1, max = 5))]
    pub timeliness_rating: Option<i32>,
    #[serde(rename = "reviewText")]
    #[validate(length(max = 2000))]
    pub review_text: Option<String>,
    #[serde(rename = "isPublic")]
    pub is_public: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewResponse {
    pub id: Uuid,
    pub deal_id: Uuid,
    pub reviewer_party_id: Uuid,
    pub reviewed_party_id: Uuid,
    pub reviewed_role: String,
    pub overall_rating: i32,
    pub communication_rating: Option<i32>,
    pub reliability_rating: Option<i32>,
    pub quality_rating: Option<i32>,
    pub timeliness_rating: Option<i32>,
    pub review_text: Option<String>,
    pub is_verified: bool,
    pub is_public: bool,
    pub platform_response: Option<String>,
    pub created_at: OffsetDateTime,
}

impl From<application::reviews::dto::ReviewResult> for ReviewResponse {
    fn from(r: application::reviews::dto::ReviewResult) -> Self {
        Self {
            id: r.id,
            deal_id: r.deal_id,
            reviewer_party_id: r.reviewer_party_id,
            reviewed_party_id: r.reviewed_party_id,
            reviewed_role: r.reviewed_role.as_str().to_string(),
            overall_rating: r.overall_rating,
            communication_rating: r.communication_rating,
            reliability_rating: r.reliability_rating,
            quality_rating: r.quality_rating,
            timeliness_rating: r.timeliness_rating,
            review_text: r.review_text,
            is_verified: r.is_verified,
            is_public: r.is_public,
            platform_response: r.platform_response,
            created_at: r.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewsResponse {
    pub reviews: Vec<ReviewResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

impl From<application::reviews::dto::ReviewListResult> for ReviewsResponse {
    fn from(r: application::reviews::dto::ReviewListResult) -> Self {
        Self {
            reviews: r.reviews.into_iter().map(Into::into).collect(),
            total: r.total,
            limit: r.limit,
            offset: r.offset,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DealReviewStatusResponse {
    pub deal_id: Uuid,
    pub total_required: i64,
    pub total_received: i64,
    pub is_complete: bool,
    pub missing_pairs: Vec<(Uuid, Uuid)>,
}

impl From<application::reviews::dto::DealReviewStatusResult> for DealReviewStatusResponse {
    fn from(r: application::reviews::dto::DealReviewStatusResult) -> Self {
        Self {
            deal_id: r.deal_id,
            total_required: r.total_required,
            total_received: r.total_received,
            is_complete: r.is_complete,
            missing_pairs: r.missing_pairs,
        }
    }
}
