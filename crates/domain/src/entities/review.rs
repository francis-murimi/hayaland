use crate::errors::DomainError;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use super::DealRole;

/// A validated 1–5 star rating.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ReviewRating(u8);

impl ReviewRating {
    pub fn new(value: i32) -> Result<Self, DomainError> {
        if !(1..=5).contains(&value) {
            return Err(DomainError::InvalidReviewRating {
                message: "rating must be between 1 and 5".to_string(),
            });
        }
        Ok(Self(value as u8))
    }

    pub fn value(&self) -> u8 {
        self.0
    }
}

impl From<ReviewRating> for i32 {
    fn from(r: ReviewRating) -> Self {
        r.0 as i32
    }
}

/// A validated review text (optional, max 2 000 chars).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ReviewText(String);

impl ReviewText {
    pub fn new(value: &str) -> Result<Self, DomainError> {
        let trimmed = value.trim();
        if trimmed.chars().count() > 2000 {
            return Err(DomainError::InvalidReviewText {
                message: "review text must be 2000 characters or fewer".to_string(),
            });
        }
        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A post-deal review left by one party about another.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Review {
    pub id: Uuid,
    pub deal_id: Uuid,
    pub reviewer_party_id: Uuid,
    pub reviewed_party_id: Uuid,
    pub reviewed_role: DealRole,
    pub overall_rating: ReviewRating,
    pub communication_rating: Option<ReviewRating>,
    pub reliability_rating: Option<ReviewRating>,
    pub quality_rating: Option<ReviewRating>,
    pub timeliness_rating: Option<ReviewRating>,
    pub review_text: Option<String>,
    pub is_verified: bool,
    pub is_public: bool,
    pub platform_response: Option<String>,
    pub created_at: OffsetDateTime,
}

impl Review {
    /// Create a review. Validation of deal state / roles happens in the application layer.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: Uuid,
        deal_id: Uuid,
        reviewer_party_id: Uuid,
        reviewed_party_id: Uuid,
        reviewed_role: DealRole,
        overall_rating: ReviewRating,
        communication_rating: Option<ReviewRating>,
        reliability_rating: Option<ReviewRating>,
        quality_rating: Option<ReviewRating>,
        timeliness_rating: Option<ReviewRating>,
        review_text: Option<ReviewText>,
        is_public: bool,
    ) -> Self {
        Self {
            id,
            deal_id,
            reviewer_party_id,
            reviewed_party_id,
            reviewed_role,
            overall_rating,
            communication_rating,
            reliability_rating,
            quality_rating,
            timeliness_rating,
            review_text: review_text.map(|t| t.as_str().to_owned()),
            is_verified: false,
            is_public,
            platform_response: None,
            created_at: OffsetDateTime::now_utc(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn review_rating_accepts_valid_values() {
        for v in 1..=5 {
            assert_eq!(ReviewRating::new(v).unwrap().value(), v as u8);
        }
    }

    #[test]
    fn review_rating_rejects_out_of_range() {
        assert!(ReviewRating::new(0).is_err());
        assert!(ReviewRating::new(6).is_err());
        assert!(ReviewRating::new(-1).is_err());
    }

    #[test]
    fn review_text_rejects_too_long() {
        let text = "a".repeat(2001);
        assert!(ReviewText::new(&text).is_err());
    }

    #[test]
    fn review_text_accepts_max_length() {
        let text = "a".repeat(2000);
        assert_eq!(ReviewText::new(&text).unwrap().as_str().len(), 2000);
    }

    #[test]
    fn review_new_stores_fields_and_defaults_unverified() {
        let id = Uuid::now_v7();
        let deal_id = Uuid::now_v7();
        let reviewer = Uuid::now_v7();
        let reviewed = Uuid::now_v7();
        let rating = ReviewRating::new(4).unwrap();

        let review = Review::new(
            id,
            deal_id,
            reviewer,
            reviewed,
            DealRole::Supplier,
            rating,
            None,
            None,
            None,
            None,
            None,
            true,
        );

        assert_eq!(review.id, id);
        assert_eq!(review.deal_id, deal_id);
        assert_eq!(review.reviewer_party_id, reviewer);
        assert_eq!(review.reviewed_party_id, reviewed);
        assert_eq!(review.reviewed_role, DealRole::Supplier);
        assert_eq!(review.overall_rating.value(), 4);
        assert!(!review.is_verified);
        assert!(review.is_public);
        assert!(review.platform_response.is_none());
    }

    #[test]
    fn review_new_trims_text() {
        let review = Review::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            DealRole::Consumer,
            ReviewRating::new(3).unwrap(),
            None,
            None,
            None,
            None,
            Some(ReviewText::new("  hello  ").unwrap()),
            true,
        );

        assert_eq!(review.review_text.as_deref(), Some("hello"));
    }
}
