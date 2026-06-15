use domain::entities::{DealRole, Review};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// Command to submit a review.
#[derive(Debug, Clone, Deserialize)]
pub struct SubmitReviewCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Uuid,
    pub is_admin: bool,
    pub deal_id: Uuid,
    pub reviewed_party_id: Uuid,
    pub overall_rating: i32,
    pub communication_rating: Option<i32>,
    pub reliability_rating: Option<i32>,
    pub quality_rating: Option<i32>,
    pub timeliness_rating: Option<i32>,
    pub review_text: Option<String>,
    pub is_public: Option<bool>,
}

/// Single review as returned by application use cases.
#[derive(Debug, Clone, Serialize)]
pub struct ReviewResult {
    pub id: Uuid,
    pub deal_id: Uuid,
    pub reviewer_party_id: Uuid,
    pub reviewed_party_id: Uuid,
    pub reviewed_role: DealRole,
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

impl From<Review> for ReviewResult {
    fn from(r: Review) -> Self {
        Self {
            id: r.id,
            deal_id: r.deal_id,
            reviewer_party_id: r.reviewer_party_id,
            reviewed_party_id: r.reviewed_party_id,
            reviewed_role: r.reviewed_role,
            overall_rating: r.overall_rating.value() as i32,
            communication_rating: r.communication_rating.map(|x| x.value() as i32),
            reliability_rating: r.reliability_rating.map(|x| x.value() as i32),
            quality_rating: r.quality_rating.map(|x| x.value() as i32),
            timeliness_rating: r.timeliness_rating.map(|x| x.value() as i32),
            review_text: r.review_text,
            is_verified: r.is_verified,
            is_public: r.is_public,
            platform_response: r.platform_response,
            created_at: r.created_at,
        }
    }
}

/// Paginated list of reviews.
#[derive(Debug, Clone, Serialize)]
pub struct ReviewListResult {
    pub reviews: Vec<ReviewResult>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// Query for listing a deal's reviews.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ListDealReviewsQuery {
    pub reviewed_party_id: Option<Uuid>,
    pub is_public: Option<bool>,
    pub limit: i64,
    pub offset: i64,
}

/// Query for listing a party's public (or own) reviews.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ListPartyReviewsQuery {
    pub actor_user_id: Uuid,
    pub actor_party_id: Option<Uuid>,
    pub is_admin: bool,
    pub limit: i64,
    pub offset: i64,
}

/// Query to fetch a single review.
#[derive(Debug, Clone, Deserialize)]
pub struct GetReviewQuery {
    pub actor_user_id: Uuid,
    pub actor_party_id: Option<Uuid>,
    pub is_admin: bool,
}

/// Result of checking a deal's review completeness.
#[derive(Debug, Clone, Serialize)]
pub struct DealReviewStatusResult {
    pub deal_id: Uuid,
    pub total_required: i64,
    pub total_received: i64,
    pub is_complete: bool,
    pub missing_pairs: Vec<(Uuid, Uuid)>,
}

/// Query for admin review listing.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AdminReviewListQuery {
    pub deal_id: Option<Uuid>,
    pub reviewer_party_id: Option<Uuid>,
    pub reviewed_party_id: Option<Uuid>,
    pub is_public: Option<bool>,
    pub limit: i64,
    pub offset: i64,
}
