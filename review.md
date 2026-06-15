# Review Feature — Implementation Specification

> **Scope:** Design and implement the post-deal review subsystem for Hayaland.  
> **Audience:** Backend engineers implementing the reputation pipeline.  
> **Depends on:** `Deal`, `DealParticipation`, `Party`, `User`, JWT/`X-Party-ID` auth, existing `reviews` table migration.  
> **Leads to:** Trust-score recalculation (`trust-score.md`) and, later, the matching engine.

---

## 1. Overview

A **Review** is a multi-dimensional rating left by one deal participant about another participant after a deal reaches a terminal or reviewable state.  Reviews feed the trust-score calculator and are the primary social signal used by the matching engine.

### 1.1 MVP goals

1. Any **member** of a deal's participating party can rate each of the **other** participating parties — exactly once per pair.
2. One review per `(deal, reviewer_party, reviewed_party)` tuple.
3. Reviews support a required `overall_rating` and optional dimension ratings (`communication`, `reliability`, `quality`, `timeliness`).
4. Reviews can be public or private; private reviews still count toward trust.
5. A deal is **only considered fully complete** when every participating party has reviewed every other participating party.
6. Submitting a review triggers a trust-score recalculation request for the reviewed party.
7. Platform admins (or users holding the `admin:reviews` / `reviews:admin` scope) can list, hide, and otherwise manage reviews.
8. All endpoints respect the existing `X-Party-ID` / scope / admin-bypass authorization model.

### 1.2 Out of scope (future)

- Review challenges / formal moderation workflow beyond admin hide/unhide.
- Media attachments.
- Review replies.
- Weighted scoring by reviewer trust (consumed later by `TrustCalculator`).
- In-app notifications prompting users to leave a review.
- Editing a review after submission.

---

## 2. Prerequisites & Context

The following already exist in the codebase and are used as-is:

| Component | Relevance |
|---|---|
| `deals` table + `Deal` aggregate | Reviews are scoped to a deal; the deal must be `COMPLETED` before a review can be submitted. |
| `deal_participations` table | Validates reviewer/reviewed roles and membership. |
| `parties` table | `PartyRepository::is_user_member_of_party` validates the caller. |
| `users` table | The authenticated user is recorded as the author. |
| `reviews` table | Created by migration `20260613014000_create_agreements_signatures_reviews_trust.sql`. |
| `AuthContext` / `X-Party-ID` | Existing acting-party resolution pattern. |
| Scope checks | `require_scope_or_admin` from `api/src/middleware/auth.rs`. |

No changes to existing source code are required by this document, but a small additive migration is recommended to enforce uniqueness and add lookup indexes.

---

## 3. Domain Model

### 3.1 New file: `crates/domain/src/entities/review.rs`

```rust
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
    /// Create a review.  Validation of deal state / roles happens in the application layer.
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
```

### 3.2 Update `crates/domain/src/entities/mod.rs`

Add:

```rust
pub mod review;
pub use review::*;
```

### 3.3 Review completeness rule

A deal has three participating parties: Supplier, Consumer, and Enhancer.  It is **fully reviewed** when there is a review from every party to every other party — i.e. six reviews total (A→B, A→C, B→A, B→C, C→A, C→B).

This rule is enforced by the `ExecuteTransition` use case when a deal attempts to move from `EXECUTING` to `COMPLETED`.  If any required review is missing, the transition is rejected with a `Validation` error listing the missing `(reviewer, reviewed)` pairs.

Reviews may be submitted while the deal is in `EXECUTING` (after milestones are verified) or already `COMPLETED`.  In practice, the normal flow is:

```text
Milestones verified
  → parties submit reviews
  → all six reviews present
  → transition to COMPLETED allowed
```

### 3.4 New error variants: `crates/domain/src/errors.rs`

Add to `DomainError`:

```rust
#[error("invalid review rating: {message}")]
InvalidReviewRating { message: String },

#[error("invalid review text: {message}")]
InvalidReviewText { message: String },

#[error("review not found")]
ReviewNotFound,

#[error("a review already exists for this deal and party")]
DuplicateReview,

#[error("review period has expired")]
ReviewPeriodExpired,
```

Map the new validation-style variants to `ApplicationError::Validation` in `crates/application/src/errors.rs`, and `DuplicateReview` to a new `ApplicationError::DuplicateReview` variant (or reuse `Validation`).  `ReviewNotFound` maps to `ApplicationError::NotFound`.

---

## 4. Repository Port

### 4.1 New file: `crates/domain/src/repositories/review_repository.rs`

```rust
use async_trait::async_trait;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::entities::{DealRole, Review};
use crate::errors::DomainError;

#[derive(Debug, Clone, Default)]
pub struct ReviewSearchCriteria {
    pub deal_id: Option<Uuid>,
    pub reviewer_party_id: Option<Uuid>,
    pub reviewed_party_id: Option<Uuid>,
    pub is_public: Option<bool>,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone)]
pub struct ReviewListResult {
    pub reviews: Vec<Review>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[async_trait]
pub trait ReviewRepository: Send + Sync {
    /// Persist a new review.
    async fn create(&self, review: &Review) -> Result<(), DomainError>;

    /// Find a review by id.
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Review>, DomainError>;

    /// Check whether a review already exists for the (deal, reviewer, reviewed) tuple.
    async fn exists(
        &self,
        deal_id: Uuid,
        reviewer_party_id: Uuid,
        reviewed_party_id: Uuid,
    ) -> Result<bool, DomainError>;

    /// Count how many reviews exist for a deal.
    async fn count_by_deal(&self, deal_id: Uuid) -> Result<i64, DomainError>;

    /// Return the (reviewer_party_id, reviewed_party_id) pairs that do NOT have a review yet.
    async fn find_missing_review_pairs(
        &self,
        deal_id: Uuid,
        participations: &[(Uuid, DealRole)],
    ) -> Result<Vec<(Uuid, Uuid)>, DomainError>;

    /// List reviews matching the criteria.
    async fn list(&self, criteria: &ReviewSearchCriteria) -> Result<ReviewListResult, DomainError>;

    /// Count reviews matching the criteria.
    async fn count(&self, criteria: &ReviewSearchCriteria) -> Result<i64, DomainError>;

    /// Update an existing review (text / ratings / visibility).  MVP optional.
    async fn update(&self, review: &Review) -> Result<(), DomainError>;

    /// Soft-delete a review by clearing public visibility and text.  MVP optional.
    async fn hide(&self, id: Uuid, platform_response: Option<String>) -> Result<(), DomainError>;
}
```

### 4.2 Update `crates/domain/src/repositories/mod.rs`

Add:

```rust
pub mod review_repository;
pub use review_repository::*;
```

---

## 5. Application Use Cases

Create a new module: `crates/application/src/reviews/`.

### 5.1 Module file: `crates/application/src/reviews/mod.rs`

```rust
pub mod dto;
pub mod get_deal_review_status;
pub mod get_review;
pub mod hide_review;
pub mod list_admin_reviews;
pub mod list_deal_reviews;
pub mod list_party_reviews;
pub mod submit_review;

pub use get_deal_review_status::GetDealReviewStatus;
pub use get_review::GetReview;
pub use hide_review::HideReview;
pub use list_admin_reviews::ListAdminReviews;
pub use list_deal_reviews::ListDealReviews;
pub use list_party_reviews::ListPartyReviews;
pub use submit_review::SubmitReview;
```

Add `pub mod reviews;` to `crates/application/src/lib.rs`.

### 5.2 DTOs: `crates/application/src/reviews/dto.rs`

```rust
use domain::entities::{DealRole, Review, ReviewRating};
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
```


### 5.3 `SubmitReview` use case: `crates/application/src/reviews/submit_review.rs`

```rust
use crate::errors::ApplicationError;
use crate::reviews::dto::{ReviewResult, SubmitReviewCommand};
use domain::entities::{DealRole, DealStatus, Review, ReviewRating, ReviewText};
use domain::repositories::{DealRepository, PartyRepository, ReviewRepository};
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

/// Outbound port used to request trust-score recalculation after a review is saved.
#[async_trait::async_trait]
pub trait TrustScoreRecalculationPort: Send + Sync {
    async fn request_recalculation(&self, party_id: Uuid) -> Result<(), ApplicationError>;
}

/// No-op implementation for the review milestone; replace with the real trust-score use case.
pub struct NoOpTrustScoreRecalculation;

#[async_trait::async_trait]
impl TrustScoreRecalculationPort for NoOpTrustScoreRecalculation {
    async fn request_recalculation(&self, _party_id: Uuid) -> Result<(), ApplicationError> {
        Ok(())
    }
}

#[derive(Clone)]
pub struct SubmitReview {
    review_repo: Arc<dyn ReviewRepository>,
    deal_repo: Arc<dyn DealRepository>,
    party_repo: Arc<dyn PartyRepository>,
    recalc: Arc<dyn TrustScoreRecalculationPort>,
}

impl SubmitReview {
    pub fn new(
        review_repo: Arc<dyn ReviewRepository>,
        deal_repo: Arc<dyn DealRepository>,
        party_repo: Arc<dyn PartyRepository>,
        recalc: Arc<dyn TrustScoreRecalculationPort>,
    ) -> Self {
        Self {
            review_repo,
            deal_repo,
            party_repo,
            recalc,
        }
    }

    #[instrument(skip(self, cmd), fields(deal_id = %cmd.deal_id))]
    pub async fn execute(&self, cmd: SubmitReviewCommand) -> Result<ReviewResult, ApplicationError> {
        // 1. Load deal aggregate.
        let aggregate = self
            .deal_repo
            .find_aggregate_by_id(cmd.deal_id)
            .await?
            .ok_or(ApplicationError::DealNotFound)?;

        let deal = aggregate.deal;
        let participations = aggregate.participations;

        // 2. Reviews can be submitted while executing (after milestones are verified)
        //    or after the deal is already completed.
        if deal.deal_status != DealStatus::Executing && deal.deal_status != DealStatus::Completed {
            return Err(ApplicationError::Validation(vec![
                "reviews can only be submitted while the deal is executing or completed".to_string(),
            ]));
        }

        // 3. Caller must be a member of the acting party (reviewer), unless admin.
        if !cmd.is_admin
            && !self
                .party_repo
                .is_user_member_of_party(cmd.actor_user_id, cmd.actor_party_id)
                .await?
        {
            return Err(ApplicationError::Forbidden);
        }

        // 4. Reviewer and reviewed must both be participants, and must be distinct.
        let reviewer = participations
            .iter()
            .find(|p| p.party_id == cmd.actor_party_id)
            .ok_or(ApplicationError::DealAccessDenied)?;

        let reviewed = participations
            .iter()
            .find(|p| p.party_id == cmd.reviewed_party_id)
            .ok_or(ApplicationError::DealAccessDenied)?;

        if reviewer.party_id == reviewed.party_id {
            return Err(ApplicationError::Validation(vec![
                "parties cannot review themselves".to_string(),
            ]));
        }

        // 5. No duplicate review.
        if self
            .review_repo
            .exists(cmd.deal_id, reviewer.party_id, reviewed.party_id)
            .await?
        {
            return Err(ApplicationError::Validation(vec![
                "review already exists for this deal and party".to_string(),
            ]));
        }

        // 6. Build domain value objects.
        let overall = ReviewRating::new(cmd.overall_rating)?;
        let communication = cmd.communication_rating.map(ReviewRating::new).transpose()?;
        let reliability = cmd.reliability_rating.map(ReviewRating::new).transpose()?;
        let quality = cmd.quality_rating.map(ReviewRating::new).transpose()?;
        let timeliness = cmd.timeliness_rating.map(ReviewRating::new).transpose()?;
        let text = cmd.review_text.map(|t| ReviewText::new(&t)).transpose()?;

        let mut review = Review::new(
            Uuid::now_v7(),
            cmd.deal_id,
            reviewer.party_id,
            reviewed.party_id,
            reviewed.role,
            overall,
            communication,
            reliability,
            quality,
            timeliness,
            text,
            cmd.is_public.unwrap_or(true),
        );

        // A review is "verified" when it is tied to a completed deal with accepted participations.
        review.is_verified = true;

        // 7. Persist.
        self.review_repo.create(&review).await?;

        info!(
            review_id = %review.id,
            reviewed_party_id = %review.reviewed_party_id,
            "review submitted"
        );

        // 8. Request trust-score recalculation for the reviewed party.
        self.recalc
            .request_recalculation(review.reviewed_party_id)
            .await?;

        Ok(review.into())
    }
}
```

Key points:

- The reviewer is derived from `actor_party_id` (the `X-Party-ID` header), not from the request body.
- `reviewed_role` is taken from the deal participation record, not from user input.
- Duplicate detection is done in the application layer before inserting; the unique index in the database is the final guard.
- Trust-score recalculation is triggered through a small outbound port so the review module does not depend on the not-yet-implemented trust calculator.

### 5.4 `ListDealReviews` use case: `crates/application/src/reviews/list_deal_reviews.rs`

```rust
use crate::errors::ApplicationError;
use crate::reviews::dto::{ListDealReviewsQuery, ReviewListResult};
use domain::repositories::{DealRepository, ReviewRepository};
use domain::repositories::ReviewSearchCriteria;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct ListDealReviews {
    deal_repo: Arc<dyn DealRepository>,
    review_repo: Arc<dyn ReviewRepository>,
}

impl ListDealReviews {
    pub fn new(deal_repo: Arc<dyn DealRepository>, review_repo: Arc<dyn ReviewRepository>) -> Self {
        Self { deal_repo, review_repo }
    }

    pub async fn execute(
        &self,
        deal_id: Uuid,
        actor_user_id: Uuid,
        actor_party_id: Option<Uuid>,
        is_admin: bool,
        query: ListDealReviewsQuery,
    ) -> Result<ReviewListResult, ApplicationError> {
        // Visibility: the caller must be a participant or an admin.
        let visible = if is_admin {
            true
        } else if let Some(party_id) = actor_party_id {
            self.deal_repo.is_party_participant(deal_id, party_id).await?
        } else {
            false
        };

        if !visible {
            return Err(ApplicationError::DealAccessDenied);
        }

        let criteria = ReviewSearchCriteria {
            deal_id: Some(deal_id),
            reviewed_party_id: query.reviewed_party_id,
            is_public: query.is_public,
            limit: query.limit.max(1),
            offset: query.offset.max(0),
        };

        let result = self.review_repo.list(&criteria).await?;
        Ok(ReviewListResult {
            reviews: result.reviews.into_iter().map(Into::into).collect(),
            total: result.total,
            limit: result.limit,
            offset: result.offset,
        })
    }
}
```

### 5.5 `ListPartyReviews` use case: `crates/application/src/reviews/list_party_reviews.rs`

```rust
use crate::errors::ApplicationError;
use crate::reviews::dto::{ListPartyReviewsQuery, ReviewListResult};
use domain::repositories::{PartyRepository, ReviewRepository};
use domain::repositories::ReviewSearchCriteria;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct ListPartyReviews {
    party_repo: Arc<dyn PartyRepository>,
    review_repo: Arc<dyn ReviewRepository>,
}

impl ListPartyReviews {
    pub fn new(
        party_repo: Arc<dyn PartyRepository>,
        review_repo: Arc<dyn ReviewRepository>,
    ) -> Self {
        Self {
            party_repo,
            review_repo,
        }
    }

    pub async fn execute(
        &self,
        party_id: Uuid,
        query: ListPartyReviewsQuery,
    ) -> Result<ReviewListResult, ApplicationError> {
        // Verify the target party exists.
        if self.party_repo.find_by_id(party_id).await?.is_none() {
            return Err(ApplicationError::PartyNotFound);
        }

        let viewing_own = query
            .actor_party_id
            .map(|id| id == party_id)
            .unwrap_or(false);

        let criteria = ReviewSearchCriteria {
            reviewed_party_id: Some(party_id),
            is_public: if query.is_admin || viewing_own {
                None
            } else {
                Some(true)
            },
            limit: query.limit.max(1),
            offset: query.offset.max(0),
            ..Default::default()
        };

        let result = self.review_repo.list(&criteria).await?;
        Ok(ReviewListResult {
            reviews: result.reviews.into_iter().map(Into::into).collect(),
            total: result.total,
            limit: result.limit,
            offset: result.offset,
        })
    }
}
```

### 5.6 `GetReview` use case: `crates/application/src/reviews/get_review.rs`

```rust
use crate::errors::ApplicationError;
use crate::reviews::dto::{GetReviewQuery, ReviewResult};
use domain::repositories::{DealRepository, ReviewRepository};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct GetReview {
    deal_repo: Arc<dyn DealRepository>,
    review_repo: Arc<dyn ReviewRepository>,
}

impl GetReview {
    pub fn new(deal_repo: Arc<dyn DealRepository>, review_repo: Arc<dyn ReviewRepository>) -> Self {
        Self { deal_repo, review_repo }
    }

    pub async fn execute(
        &self,
        review_id: Uuid,
        query: GetReviewQuery,
    ) -> Result<ReviewResult, ApplicationError> {
        let review = self
            .review_repo
            .find_by_id(review_id)
            .await?
            .ok_or(ApplicationError::NotFound)?;

        // Private reviews are visible only to the reviewer, the reviewed party, and admins.
        if !review.is_public {
            let allowed = query.is_admin
                || query.actor_party_id == Some(review.reviewer_party_id)
                || query.actor_party_id == Some(review.reviewed_party_id);
            if !allowed {
                return Err(ApplicationError::DealAccessDenied);
            }
        }

        // Everyone else must at least be a deal participant or admin to see a public review.
        if !query.is_admin {
            let party_id = query.actor_party_id.ok_or(ApplicationError::Forbidden)?;
            if !self.deal_repo.is_party_participant(review.deal_id, party_id).await? {
                return Err(ApplicationError::DealAccessDenied);
            }
        }

        Ok(review.into())
    }
}
```

### 5.7 `GetDealReviewStatus` use case

Used by clients to check which reviews are still missing before attempting the `EXECUTING → COMPLETED` transition.

```rust
use crate::errors::ApplicationError;
use crate::reviews::dto::DealReviewStatusResult;
use domain::repositories::{DealRepository, ReviewRepository};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct GetDealReviewStatus {
    deal_repo: Arc<dyn DealRepository>,
    review_repo: Arc<dyn ReviewRepository>,
}

impl GetDealReviewStatus {
    pub fn new(
        deal_repo: Arc<dyn DealRepository>,
        review_repo: Arc<dyn ReviewRepository>,
    ) -> Self {
        Self { deal_repo, review_repo }
    }

    pub async fn execute(
        &self,
        deal_id: Uuid,
        actor_user_id: Uuid,
        actor_party_id: Option<Uuid>,
        is_admin: bool,
    ) -> Result<DealReviewStatusResult, ApplicationError> {
        // Visibility: only participants or admins may see review status.
        let visible = if is_admin {
            true
        } else if let Some(party_id) = actor_party_id {
            self.deal_repo.is_party_participant(deal_id, party_id).await?
        } else {
            false
        };

        if !visible {
            return Err(ApplicationError::DealAccessDenied);
        }

        let aggregate = self
            .deal_repo
            .find_aggregate_by_id(deal_id)
            .await?
            .ok_or(ApplicationError::DealNotFound)?;

        let pairs: Vec<(Uuid, _)> = aggregate
            .participations
            .iter()
            .map(|p| (p.party_id, p.role))
            .collect();

        let missing = self
            .review_repo
            .find_missing_review_pairs(deal_id, &pairs)
            .await?;

        let total_required = pairs.len().saturating_sub(1) * pairs.len();
        let total_received = total_required - missing.len();

        Ok(DealReviewStatusResult {
            deal_id,
            total_required: total_required as i64,
            total_received: total_received as i64,
            is_complete: missing.is_empty(),
            missing_pairs,
        })
    }
}
```

Add `DealReviewStatusResult` to `crates/application/src/reviews/dto.rs`:

```rust
#[derive(Debug, Clone, Serialize)]
pub struct DealReviewStatusResult {
    pub deal_id: Uuid,
    pub total_required: i64,
    pub total_received: i64,
    pub is_complete: bool,
    pub missing_pairs: Vec<(Uuid, Uuid)>,
}
```

### 5.8 `HideReview` (admin management)

```rust
use crate::errors::ApplicationError;
use domain::repositories::ReviewRepository;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct HideReview {
    review_repo: Arc<dyn ReviewRepository>,
}

impl HideReview {
    pub fn new(review_repo: Arc<dyn ReviewRepository>) -> Self {
        Self { review_repo }
    }

    pub async fn execute(
        &self,
        review_id: Uuid,
        platform_response: Option<String>,
    ) -> Result<(), ApplicationError> {
        // hide() sets is_public = false, clears review_text, and records the admin response.
        self.review_repo.hide(review_id, platform_response).await?;
        Ok(())
    }
}
```

### 5.9 `ListAdminReviews` (admin management)

```rust
use crate::errors::ApplicationError;
use crate::reviews::dto::{AdminReviewListQuery, ReviewListResult};
use domain::repositories::{ReviewListResult as RepoListResult, ReviewRepository};
use domain::repositories::ReviewSearchCriteria;
use std::sync::Arc;

#[derive(Clone)]
pub struct ListAdminReviews {
    review_repo: Arc<dyn ReviewRepository>,
}

impl ListAdminReviews {
    pub fn new(review_repo: Arc<dyn ReviewRepository>) -> Self {
        Self { review_repo }
    }

    pub async fn execute(
        &self,
        query: AdminReviewListQuery,
    ) -> Result<ReviewListResult, ApplicationError> {
        let criteria = ReviewSearchCriteria {
            deal_id: query.deal_id,
            reviewer_party_id: query.reviewer_party_id,
            reviewed_party_id: query.reviewed_party_id,
            is_public: query.is_public,
            limit: query.limit.max(1),
            offset: query.offset.max(0),
        };

        let result = self.review_repo.list(&criteria).await?;
        Ok(ReviewListResult {
            reviews: result.reviews.into_iter().map(Into::into).collect(),
            total: result.total,
            limit: result.limit,
            offset: result.offset,
        })
    }
}
```

Add `AdminReviewListQuery` to DTOs:

```rust
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AdminReviewListQuery {
    pub deal_id: Option<Uuid>,
    pub reviewer_party_id: Option<Uuid>,
    pub reviewed_party_id: Option<Uuid>,
    pub is_public: Option<bool>,
    pub limit: i64,
    pub offset: i64,
}
```

### 5.10 Trust-score integration

`SubmitReview` depends on `TrustScoreRecalculationPort`.  Two wiring options:

1. **No-op stub (recommended for the review milestone):** use `NoOpTrustScoreRecalculation` until the trust-score use case exists.  This keeps the review PR self-contained and green.
2. **Direct coupling:** once `RecalculateTrustScore` is implemented, replace the no-op with a thin adapter that calls it for the reviewed party.

The no-op must still be wired in `AppState` so the port type is exercised by tests.

---

## 6. Infrastructure

### 6.1 Postgres repository: `crates/infrastructure/src/repositories/postgres_review_repository.rs`

Add the file and expose it from `crates/infrastructure/src/repositories/mod.rs`.

```rust
use async_trait::async_trait;
use domain::entities::{DealRole, Review, ReviewRating};
use domain::errors::DomainError;
use domain::repositories::{ReviewListResult, ReviewRepository, ReviewSearchCriteria};
use sqlx::{Error as SqlxError, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

pub struct PostgresReviewRepository {
    pool: PgPool,
}

impl PostgresReviewRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ReviewRepository for PostgresReviewRepository {
    async fn create(&self, review: &Review) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO reviews (
                id, deal_id, reviewer_party_id, reviewed_party_id, reviewed_role,
                overall_rating, communication_rating, reliability_rating, quality_rating,
                timeliness_rating, review_text, is_verified, is_public, platform_response, created_at
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15
            )
            "#,
            review.id,
            review.deal_id,
            review.reviewer_party_id,
            review.reviewed_party_id,
            review.reviewed_role.as_str(),
            review.overall_rating.value() as i32,
            review.communication_rating.map(|r| r.value() as i32),
            review.reliability_rating.map(|r| r.value() as i32),
            review.quality_rating.map(|r| r.value() as i32),
            review.timeliness_rating.map(|r| r.value() as i32),
            review.review_text,
            review.is_verified,
            review.is_public,
            review.platform_response,
            review.created_at
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Review>, DomainError> {
        let row = sqlx::query_as!(
            ReviewRow,
            r#"
            SELECT
                id as "id!",
                deal_id as "deal_id!",
                reviewer_party_id as "reviewer_party_id!",
                reviewed_party_id as "reviewed_party_id!",
                reviewed_role as "reviewed_role!",
                overall_rating as "overall_rating!",
                communication_rating as "communication_rating: _",
                reliability_rating as "reliability_rating: _",
                quality_rating as "quality_rating: _",
                timeliness_rating as "timeliness_rating: _",
                review_text,
                is_verified as "is_verified!",
                is_public as "is_public!",
                platform_response,
                created_at as "created_at!"
            FROM reviews
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.map(build_review_from_row))
    }

    async fn exists(
        &self,
        deal_id: Uuid,
        reviewer_party_id: Uuid,
        reviewed_party_id: Uuid,
    ) -> Result<bool, DomainError> {
        let row = sqlx::query_scalar!(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM reviews
                WHERE deal_id = $1
                  AND reviewer_party_id = $2
                  AND reviewed_party_id = $3
            ) as "exists!"
            "#,
            deal_id,
            reviewer_party_id,
            reviewed_party_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row)
    }

    async fn count_by_deal(&self, deal_id: Uuid) -> Result<i64, DomainError> {
        let row = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM reviews
            WHERE deal_id = $1
            "#,
            deal_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row)
    }

    async fn find_missing_review_pairs(
        &self,
        deal_id: Uuid,
        participations: &[(Uuid, DealRole)],
    ) -> Result<Vec<(Uuid, Uuid)>, DomainError> {
        let party_ids: Vec<Uuid> = participations.iter().map(|(id, _)| *id).collect();

        let rows = sqlx::query_as!(
            ExistingReviewPair,
            r#"
            SELECT reviewer_party_id as "reviewer_party_id!", reviewed_party_id as "reviewed_party_id!"
            FROM reviews
            WHERE deal_id = $1
              AND reviewer_party_id = ANY($2)
              AND reviewed_party_id = ANY($2)
            "#,
            deal_id,
            &party_ids
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        let existing: std::collections::HashSet<(Uuid, Uuid)> = rows
            .into_iter()
            .map(|r| (r.reviewer_party_id, r.reviewed_party_id))
            .collect();

        let mut missing = Vec::new();
        for (reviewer, _) in participations {
            for (reviewed, _) in participations {
                if reviewer != reviewed && !existing.contains(&(*reviewer, *reviewed)) {
                    missing.push((*reviewer, *reviewed));
                }
            }
        }

        Ok(missing)
    }

    async fn list(&self, criteria: &ReviewSearchCriteria) -> Result<ReviewListResult, DomainError> {
        let reviews = sqlx::query_as!(
            ReviewRow,
            r#"
            SELECT
                id as "id!",
                deal_id as "deal_id!",
                reviewer_party_id as "reviewer_party_id!",
                reviewed_party_id as "reviewed_party_id!",
                reviewed_role as "reviewed_role!",
                overall_rating as "overall_rating!",
                communication_rating as "communication_rating: _",
                reliability_rating as "reliability_rating: _",
                quality_rating as "quality_rating: _",
                timeliness_rating as "timeliness_rating: _",
                review_text,
                is_verified as "is_verified!",
                is_public as "is_public!",
                platform_response,
                created_at as "created_at!"
            FROM reviews
            WHERE ($1::uuid IS NULL OR deal_id = $1)
              AND ($2::uuid IS NULL OR reviewer_party_id = $2)
              AND ($3::uuid IS NULL OR reviewed_party_id = $3)
              AND ($4::bool IS NULL OR is_public = $4)
            ORDER BY created_at DESC
            LIMIT $5
            OFFSET $6
            "#,
            criteria.deal_id,
            criteria.reviewer_party_id,
            criteria.reviewed_party_id,
            criteria.is_public,
            criteria.limit,
            criteria.offset
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        let total = self.count(criteria).await?;

        Ok(ReviewListResult {
            reviews: reviews.into_iter().map(build_review_from_row).collect(),
            total,
            limit: criteria.limit,
            offset: criteria.offset,
        })
    }

    async fn count(&self, criteria: &ReviewSearchCriteria) -> Result<i64, DomainError> {
        let row = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM reviews
            WHERE ($1::uuid IS NULL OR deal_id = $1)
              AND ($2::uuid IS NULL OR reviewer_party_id = $2)
              AND ($3::uuid IS NULL OR reviewed_party_id = $3)
              AND ($4::bool IS NULL OR is_public = $4)
            "#,
            criteria.deal_id,
            criteria.reviewer_party_id,
            criteria.reviewed_party_id,
            criteria.is_public
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row)
    }

    async fn update(&self, review: &Review) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            UPDATE reviews
            SET overall_rating = $1,
                communication_rating = $2,
                reliability_rating = $3,
                quality_rating = $4,
                timeliness_rating = $5,
                review_text = $6,
                is_public = $7,
                platform_response = $8
            WHERE id = $9
            "#,
            review.overall_rating.value() as i32,
            review.communication_rating.map(|r| r.value() as i32),
            review.reliability_rating.map(|r| r.value() as i32),
            review.quality_rating.map(|r| r.value() as i32),
            review.timeliness_rating.map(|r| r.value() as i32),
            review.review_text,
            review.is_public,
            review.platform_response,
            review.id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn hide(&self, id: Uuid, platform_response: Option<String>) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            UPDATE reviews
            SET is_public = false,
                review_text = NULL,
                platform_response = $1
            WHERE id = $2
            "#,
            platform_response,
            id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct ExistingReviewPair {
    reviewer_party_id: Uuid,
    reviewed_party_id: Uuid,
}

#[derive(sqlx::FromRow)]
struct ReviewRow {
    id: Uuid,
    deal_id: Uuid,
    reviewer_party_id: Uuid,
    reviewed_party_id: Uuid,
    reviewed_role: String,
    overall_rating: i32,
    communication_rating: Option<i32>,
    reliability_rating: Option<i32>,
    quality_rating: Option<i32>,
    timeliness_rating: Option<i32>,
    review_text: Option<String>,
    is_verified: bool,
    is_public: bool,
    platform_response: Option<String>,
    created_at: OffsetDateTime,
}

fn build_review_from_row(row: ReviewRow) -> Review {
    Review {
        id: row.id,
        deal_id: row.deal_id,
        reviewer_party_id: row.reviewer_party_id,
        reviewed_party_id: row.reviewed_party_id,
        reviewed_role: DealRole::try_from(row.reviewed_role.as_str())
            .expect("database contains valid deal roles"),
        overall_rating: ReviewRating::new(row.overall_rating)
            .expect("database contains valid ratings"),
        communication_rating: row.communication_rating.and_then(|v| ReviewRating::new(v).ok()),
        reliability_rating: row.reliability_rating.and_then(|v| ReviewRating::new(v).ok()),
        quality_rating: row.quality_rating.and_then(|v| ReviewRating::new(v).ok()),
        timeliness_rating: row.timeliness_rating.and_then(|v| ReviewRating::new(v).ok()),
        review_text: row.review_text,
        is_verified: row.is_verified,
        is_public: row.is_public,
        platform_response: row.platform_response,
        created_at: row.created_at,
    }
}

fn map_err(err: SqlxError) -> DomainError {
    DomainError::RepositoryError(err.to_string())
}
```

### 6.2 Error mapping for duplicates

The unique index on `(deal_id, reviewer_party_id, reviewed_party_id)` will raise a PostgreSQL unique-violation.  Map it in `create` to `DomainError::DuplicateReview` (then `ApplicationError::Validation` or a dedicated conflict error) so the API returns `409 Conflict` or `422 Unprocessable Entity` depending on the chosen application mapping.

### 6.3 Recommended migration

Create `migrations/20260615..._review_indexes_constraints_and_scopes.sql`:

```sql
-- Enforce one review per (deal, reviewer, reviewed) tuple.
CREATE UNIQUE INDEX IF NOT EXISTS idx_reviews_unique_pair
    ON reviews(deal_id, reviewer_party_id, reviewed_party_id);

-- Lookup public reviews for a party.
CREATE INDEX IF NOT EXISTS idx_reviews_reviewed_party
    ON reviews(reviewed_party_id, is_public, created_at DESC);

-- Lookup reviews authored by a party.
CREATE INDEX IF NOT EXISTS idx_reviews_reviewer_party
    ON reviews(reviewer_party_id, created_at DESC);

-- Grant regular users the review scopes.
UPDATE role_definitions
SET scopes = array(
    SELECT DISTINCT unnest(scopes || ARRAY['reviews:read', 'reviews:write'])
)
WHERE name = 'user';

-- Grant admins review management scope.
UPDATE role_definitions
SET scopes = array(
    SELECT DISTINCT unnest(scopes || ARRAY['admin:reviews'])
)
WHERE name = 'admin';
```

> **Note:** If you prefer not to add new scopes, reuse `deals:read`/`deals:write` for user endpoints and `admin:*` for admin endpoints.  The route-level checks in this document use `admin:reviews` for clarity.

Run `sqlx migrate run` and then `cargo sqlx prepare --workspace` to update `.sqlx/`.


---

## 7. API Layer

### 7.1 Routes: `crates/api/src/routes/reviews.rs`

```rust
use crate::handlers::reviews::{
    admin_list_reviews, admin_hide_review, create_review, get_deal_review_status, get_review,
    list_deal_reviews, list_party_reviews,
};
use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/deals/{deal_id}/reviews")
            .route(web::post().to(create_review::create_review))
            .route(web::get().to(list_deal_reviews::list_deal_reviews)),
    )
    .service(
        web::resource("/deals/{deal_id}/reviews/status")
            .route(web::get().to(get_deal_review_status::get_deal_review_status)),
    )
    .service(
        web::resource("/parties/{party_id}/reviews")
            .route(web::get().to(list_party_reviews::list_party_reviews)),
    )
    .service(
        web::resource("/reviews/{review_id}")
            .route(web::get().to(get_review::get_review)),
    )
    .service(
        web::resource("/admin/reviews")
            .route(web::get().to(admin_list_reviews::admin_list_reviews)),
    )
    .service(
        web::resource("/admin/reviews/{review_id}/hide")
            .route(web::post().to(admin_hide_review::admin_hide_review)),
    );
}
```

Register the module in `crates/api/src/routes/mod.rs`:

```rust
pub mod reviews;
```

and inside `configure`, call:

```rust
cfg.configure(reviews::configure);
```

Also register the handlers module in `crates/api/src/handlers/mod.rs`:

```rust
pub mod reviews;
```

### 7.2 Handlers directory

Create:

```text
crates/api/src/handlers/reviews/
├── mod.rs
├── admin_hide_review.rs
├── admin_list_reviews.rs
├── create_review.rs
├── dto.rs
├── get_deal_review_status.rs
├── get_review.rs
├── list_deal_reviews.rs
└── list_party_reviews.rs
```

`mod.rs`:

```rust
pub mod admin_hide_review;
pub mod admin_list_reviews;
pub mod create_review;
pub mod dto;
pub mod get_deal_review_status;
pub mod get_review;
pub mod list_deal_reviews;
pub mod list_party_reviews;
```

### 7.3 API DTOs: `crates/api/src/handlers/reviews/dto.rs`

```rust
use rust_decimal::Decimal;
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
```

### 7.4 `create_review.rs`

```rust
use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::reviews::dto::SubmitReviewCommand;
use application::users::token::AuthContext;
use uuid::Uuid;

use crate::errors::ApiError;
use crate::handlers::deals::create_deal::resolve_actor_party_id;
use crate::handlers::reviews::dto::{CreateReviewRequest, ReviewResponse};
use crate::middleware::auth::require_scope_or_admin;
use crate::AppState;

pub async fn create_review(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    body: web::Json<CreateReviewRequest>,
) -> Result<HttpResponse, ApiError> {
    body.validate().map_err(ApiError::from)?;

    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;

    require_scope_or_admin(&ctx, "reviews:write", "admin:reviews")?;

    let actor_party_id = resolve_actor_party_id(&req, &ctx)?;
    let is_admin = ctx.has_scope("admin:reviews") || ctx.has_scope("admin:*");

    let cmd = SubmitReviewCommand {
        actor_user_id: ctx.user_id,
        actor_party_id,
        is_admin,
        deal_id: path.into_inner(),
        reviewed_party_id: body.reviewed_party_id,
        overall_rating: body.overall_rating,
        communication_rating: body.communication_rating,
        reliability_rating: body.reliability_rating,
        quality_rating: body.quality_rating,
        timeliness_rating: body.timeliness_rating,
        review_text: body.review_text.clone(),
        is_public: body.is_public,
    };

    let result = state.submit_review.execute(cmd).await?;
    Ok(HttpResponse::Created().json(ReviewResponse::from(result)))
}
```

### 7.5 `list_deal_reviews.rs`

```rust
use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::reviews::dto::ListDealReviewsQuery as AppQuery;
use application::users::token::AuthContext;
use uuid::Uuid;

use crate::errors::ApiError;
use crate::handlers::deals::create_deal::resolve_actor_party_id;
use crate::handlers::reviews::dto::ReviewsResponse;
use crate::middleware::auth::require_scope_or_admin;
use crate::AppState;

#[derive(Debug, serde::Deserialize)]
pub struct Query {
    #[serde(rename = "reviewedPartyId")]
    pub reviewed_party_id: Option<Uuid>,
    #[serde(rename = "isPublic")]
    pub is_public: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_deal_reviews(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    query: web::Query<Query>,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;

    require_scope_or_admin(&ctx, "reviews:read", "admin:reviews")?;

    let actor_party_id = resolve_actor_party_id(&req, &ctx).ok();
    let is_admin = ctx.has_scope("admin:reviews") || ctx.has_scope("admin:*");

    let app_query = AppQuery {
        reviewed_party_id: query.reviewed_party_id,
        is_public: query.is_public,
        limit: query.limit.unwrap_or(20).clamp(1, 100),
        offset: query.offset.unwrap_or(0).max(0),
    };

    let result = state
        .list_deal_reviews
        .execute(path.into_inner(), ctx.user_id, actor_party_id, is_admin, app_query)
        .await?;

    Ok(HttpResponse::Ok().json(ReviewsResponse::from(result)))
}
```

### 7.6 `list_party_reviews.rs`

```rust
use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::reviews::dto::ListPartyReviewsQuery as AppQuery;
use application::users::token::AuthContext;
use uuid::Uuid;

use crate::errors::ApiError;
use crate::handlers::deals::create_deal::resolve_actor_party_id;
use crate::handlers::reviews::dto::ReviewsResponse;
use crate::middleware::auth::require_scope_or_admin;
use crate::AppState;

#[derive(Debug, serde::Deserialize)]
pub struct Query {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_party_reviews(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    query: web::Query<Query>,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;

    require_scope_or_admin(&ctx, "reviews:read", "admin:reviews")?;

    let actor_party_id = resolve_actor_party_id(&req, &ctx).ok();
    let is_admin = ctx.has_scope("admin:reviews") || ctx.has_scope("admin:*");

    let app_query = AppQuery {
        actor_user_id: ctx.user_id,
        actor_party_id,
        is_admin,
        limit: query.limit.unwrap_or(20).clamp(1, 100),
        offset: query.offset.unwrap_or(0).max(0),
    };

    let result = state
        .list_party_reviews
        .execute(path.into_inner(), app_query)
        .await?;

    Ok(HttpResponse::Ok().json(ReviewsResponse::from(result)))
}
```

### 7.7 `get_review.rs`

```rust
use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::reviews::dto::GetReviewQuery;
use application::users::token::AuthContext;
use uuid::Uuid;

use crate::errors::ApiError;
use crate::handlers::deals::create_deal::resolve_actor_party_id;
use crate::handlers::reviews::dto::ReviewResponse;
use crate::middleware::auth::require_scope_or_admin;
use crate::AppState;

pub async fn get_review(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;

    require_scope_or_admin(&ctx, "reviews:read", "admin:reviews")?;

    let actor_party_id = resolve_actor_party_id(&req, &ctx).ok();
    let is_admin = ctx.has_scope("admin:reviews") || ctx.has_scope("admin:*");

    let query = GetReviewQuery {
        actor_user_id: ctx.user_id,
        actor_party_id,
        is_admin,
    };

    let result = state.get_review.execute(path.into_inner(), query).await?;
    Ok(HttpResponse::Ok().json(ReviewResponse::from(result)))
}
```

### 7.8 `get_deal_review_status.rs`

```rust
use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::users::token::AuthContext;
use uuid::Uuid;

use crate::errors::ApiError;
use crate::handlers::deals::create_deal::resolve_actor_party_id;
use crate::handlers::reviews::dto::DealReviewStatusResponse;
use crate::middleware::auth::require_scope_or_admin;
use crate::AppState;

pub async fn get_deal_review_status(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;

    require_scope_or_admin(&ctx, "reviews:read", "admin:reviews")?;

    let actor_party_id = resolve_actor_party_id(&req, &ctx).ok();
    let is_admin = ctx.has_scope("admin:reviews") || ctx.has_scope("admin:*");

    let result = state
        .get_deal_review_status
        .execute(path.into_inner(), ctx.user_id, actor_party_id, is_admin)
        .await?;
    Ok(HttpResponse::Ok().json(DealReviewStatusResponse::from(result)))
}
```

### 7.9 `admin_list_reviews.rs`

```rust
use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::reviews::dto::AdminReviewListQuery as AppQuery;
use application::users::token::AuthContext;
use uuid::Uuid;

use crate::errors::ApiError;
use crate::handlers::reviews::dto::ReviewsResponse;
use crate::middleware::auth::require_any_scope;
use crate::AppState;

#[derive(Debug, serde::Deserialize)]
pub struct Query {
    #[serde(rename = "dealId")]
    pub deal_id: Option<Uuid>,
    #[serde(rename = "reviewerPartyId")]
    pub reviewer_party_id: Option<Uuid>,
    #[serde(rename = "reviewedPartyId")]
    pub reviewed_party_id: Option<Uuid>,
    #[serde(rename = "isPublic")]
    pub is_public: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn admin_list_reviews(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<Query>,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;

    require_any_scope(&ctx, &["admin:reviews", "admin:*"])?;

    let app_query = AppQuery {
        deal_id: query.deal_id,
        reviewer_party_id: query.reviewer_party_id,
        reviewed_party_id: query.reviewed_party_id,
        is_public: query.is_public,
        limit: query.limit.unwrap_or(20).clamp(1, 100),
        offset: query.offset.unwrap_or(0).max(0),
    };

    let result = state.list_admin_reviews.execute(app_query).await?;
    Ok(HttpResponse::Ok().json(ReviewsResponse::from(result)))
}
```

### 7.10 `admin_hide_review.rs`

```rust
use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use application::users::token::AuthContext;
use uuid::Uuid;

use crate::errors::ApiError;
use crate::middleware::auth::require_any_scope;
use crate::AppState;

#[derive(Debug, serde::Deserialize)]
pub struct HideRequest {
    #[serde(rename = "platformResponse")]
    pub platform_response: Option<String>,
}

pub async fn admin_hide_review(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    body: web::Json<HideRequest>,
) -> Result<HttpResponse, ApiError> {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Application(
            application::errors::ApplicationError::Unauthorized,
        ))?;

    require_any_scope(&ctx, &["admin:reviews", "admin:*"])?;

    state
        .hide_review
        .execute(path.into_inner(), body.platform_response.clone())
        .await?;
    Ok(HttpResponse::NoContent().finish())
}
```

---

## 8. Wiring

### 8.1 `AppState` additions (`crates/api/src/lib.rs`)

Add to imports:

```rust
use application::reviews::{
    GetDealReviewStatus, GetReview, HideReview, ListAdminReviews, ListDealReviews,
    ListPartyReviews, SubmitReview,
};
```

Add fields:

```rust
pub submit_review: SubmitReview,
pub list_deal_reviews: ListDealReviews,
pub list_party_reviews: ListPartyReviews,
pub get_review: GetReview,
pub get_deal_review_status: GetDealReviewStatus,
pub hide_review: HideReview,
pub list_admin_reviews: ListAdminReviews,
```

### 8.2 `main.rs` construction (`crates/api/src/main.rs`)

After constructing the review repository:

```rust
let review_repo: Arc<dyn ReviewRepository> =
    Arc::new(infrastructure::repositories::PostgresReviewRepository::new(pool.clone()));

let submit_review = application::reviews::SubmitReview::new(
    review_repo.clone(),
    deal_repo.clone(),
    party_repo.clone(),
    Arc::new(application::reviews::submit_review::NoOpTrustScoreRecalculation),
);
let list_deal_reviews = application::reviews::ListDealReviews::new(
    deal_repo.clone(),
    review_repo.clone(),
);
let list_party_reviews = application::reviews::ListPartyReviews::new(
    party_repo.clone(),
    review_repo.clone(),
);
let get_review = application::reviews::GetReview::new(
    deal_repo.clone(),
    review_repo.clone(),
);
let get_deal_review_status = application::reviews::GetDealReviewStatus::new(
    deal_repo.clone(),
    review_repo.clone(),
);
let hide_review = application::reviews::HideReview::new(review_repo.clone());
let list_admin_reviews = application::reviews::ListAdminReviews::new(review_repo.clone());
```

Then pass the seven review fields into `AppState { ... }`.

### 8.3 Fake repository for tests

Extend `crates/application/src/test_helpers.rs` with `FakeReviewRepository` implementing `ReviewRepository`.  This mirrors the existing `FakePartyRepository` / `FakeDealRepository` patterns and lets application-layer tests run without PostgreSQL.

### 8.4 Integration with `ExecuteTransition` (deal completion precondition)

The `EXECUTING → COMPLETED` transition must not succeed until all required reviews are present.  Extend `crates/application/src/deals/execute_transition.rs`:

1. Add an optional `review_repo: Option<Arc<dyn ReviewRepository>>` field, matching the existing `milestone_repo` pattern.
2. Add a constructor `new_with_reviews(...)`.
3. In the `DealStatus::Completed` branch, after `ensure_all_milestones_verified`, call `ensure_all_reviews_submitted(deal_id).await?`.

```rust
async fn ensure_all_reviews_submitted(&self, deal_id: Uuid) -> Result<(), ApplicationError> {
    let review_repo = self
        .review_repo
        .as_ref()
        .ok_or_else(|| ApplicationError::Infrastructure(
            "review repository not configured".to_string()
        ))?;

    let aggregate = self
        .deal_repo
        .find_aggregate_by_id(deal_id)
        .await?
        .ok_or(ApplicationError::DealNotFound)?;

    let pairs: Vec<(Uuid, _)> = aggregate
        .participations
        .iter()
        .map(|p| (p.party_id, p.role))
        .collect();

    let missing = review_repo.find_missing_review_pairs(deal_id, &pairs).await?;

    if !missing.is_empty() {
        return Err(ApplicationError::Validation(vec![
            "deal cannot be completed until all parties have reviewed each other".to_string(),
        ]));
    }

    Ok(())
}
```

Because this is a new dependency, `api/src/main.rs` should construct `ExecuteTransition` with `new_with_reviews(...)` once the review repository is available.

> **Note:** The review completeness check is enforced at the application layer.  The domain `Deal::can_transition` does **not** need to change; reviews are a cross-cutting concern, not part of the core state machine.

---

## 9. Authorization & Visibility Rules

| Endpoint | Required scope | Who can call |
|---|---|---|
| `POST /deals/{id}/reviews` | `reviews:write` or `admin:reviews` | Authenticated member of a participating party, or a review admin. |
| `GET /deals/{id}/reviews` | `reviews:read` or `admin:reviews` | Participants of the deal or review admins. |
| `GET /deals/{id}/reviews/status` | `reviews:read` or `admin:reviews` | Participants of the deal or review admins. |
| `GET /parties/{id}/reviews` | `reviews:read` or `admin:reviews` | Any authenticated user; response is filtered to public reviews unless caller is the reviewed party or an admin. |
| `GET /reviews/{id}` | `reviews:read` or `admin:reviews` | Participants of the underlying deal, or the reviewed/reviewer party, or an admin. |
| `GET /admin/reviews` | `admin:reviews` or `admin:*` | Platform admins / review managers only. |
| `POST /admin/reviews/{id}/hide` | `admin:reviews` or `admin:*` | Platform admins / review managers only. |

Notes:

- `X-Party-ID` is mandatory for `POST` and strongly recommended for `GET` endpoints.  If a user belongs to only one active party, the header may default to that party (existing pattern; see `resolve_actor_party_id`).
- Admins use the same endpoints; `is_admin` is set from `admin:reviews` / `admin:*` scopes.

---

## 10. Validation Rules

| # | Rule | Layer | Error |
|---|---|---|---|
| 1 | `overall_rating` is an integer 1–5. | API + Domain | `validation_error` |
| 2 | Optional dimension ratings, if present, are integers 1–5. | API + Domain | `validation_error` |
| 3 | `review_text` ≤ 2 000 characters. | API + Domain | `validation_error` |
| 4 | `reviewed_party_id` is a participant in the deal. | Application | `deal_access_denied` |
| 5 | `actor_party_id` is a participant in the deal. | Application | `deal_access_denied` |
| 6 | Reviewer and reviewed are not the same party. | Application | `validation_error` |
| 7 | The deal status is `EXECUTING` or `COMPLETED`. | Application | `validation_error` |
| 8 | No review already exists for `(deal, reviewer, reviewed)`. | Application + DB | `validation_error` / `409` |
| 9 | Caller is a member of `actor_party_id` (unless admin). | Application | `forbidden` |
| 10 | `reviewed_role` is taken from the participation record, not user input. | Application | — |
| 11 | `EXECUTING → COMPLETED` transition: every party has reviewed every other party. | Application | `validation_error` |

---

## 11. Error Mapping

| Situation | Domain error | Application error | HTTP status |
|---|---|---|---|
| Invalid rating | `InvalidReviewRating` | `Validation` | 400 |
| Invalid review text | `InvalidReviewText` | `Validation` | 400 |
| Reviewer == reviewed | `Validation` | `Validation` | 400 |
| Deal not completed | `Validation` | `Validation` | 400 |
| Duplicate review | `DuplicateReview` | `Validation` (or new `DuplicateReview`) | 409 / 422 |
| Review not found | `ReviewNotFound` | `NotFound` | 404 |
| Deal not found | `DealNotFound` | `DealNotFound` | 404 |
| Party not found | `PartyNotFound` | `PartyNotFound` | 404 |
| Caller not a deal participant | `InsufficientPermissions` | `DealAccessDenied` | 403 |
| Caller not a member of acting party | — | `Forbidden` | 403 |
| Review period expired | `ReviewPeriodExpired` | `Validation` | 400 |
| Deal cannot complete: missing reviews | — | `Validation` | 400 |
| Database failure | `RepositoryError` | `Infrastructure` | 500 |

Update `crates/api/src/errors.rs` to map the new `ApplicationError::DuplicateReview` variant to `StatusCode::CONFLICT` if a dedicated variant is added.

---

## 12. Testing Strategy

### 12.1 Domain unit tests (`crates/domain/src/entities/review.rs`)

- `ReviewRating::new` accepts 1–5 and rejects 0, 6, negatives.
- `ReviewText::new` rejects > 2 000 chars.
- `Review::new` stores all fields correctly and defaults `is_verified` to `false`.
- `find_missing_review_pairs` returns the correct missing tuples for a 3-party deal.

### 12.2 Application tests (`crates/application/src/reviews/tests.rs`)

Use the existing fake-repository helpers.

- **Happy path:** A party submits a review for another participant after deal completion.
- **Duplicate:** Submitting twice returns an error.
- **Self-review:** Rejected.
- **Non-participant reviewer:** Rejected with `DealAccessDenied`.
- **Deal not completed:** Rejected.
- **Invalid ratings:** Rejected.
- **List deal reviews:** Only visible to participants; non-participants get `DealAccessDenied`.
- **List party reviews:** Public/private filtering works.
- **Get private review:** Visible only to reviewer/reviewed/admins.
- **Get deal review status:** Correctly reports missing pairs until all six reviews exist.
- **Completion precondition:** Transitioning a deal to `COMPLETED` without all reviews fails; with all reviews succeeds.
- **Admin hide:** An admin can hide a review; after hiding it is no longer public.

### 12.3 Infrastructure integration tests (`crates/infrastructure/tests/postgres_review_repository.rs`)

- Create + find by id.
- Exists check.
- List with filters (`deal_id`, `reviewed_party_id`, `is_public`).
- Count.
- Duplicate insertion fails with unique violation.

### 12.4 API integration tests (`crates/api/tests/reviews.rs`)

- Create an executing deal with three parties and verified milestones.
- Submit all six required reviews.
- Assert the `GET /deals/{id}/reviews/status` endpoint reports `is_complete: true`.
- Assert the deal can transition to `COMPLETED`.
- Assert a deal missing reviews cannot transition to `COMPLETED`.
- Assert 201 and response shape for a single review.
- Assert non-participant cannot list reviews (403).
- Assert duplicate review returns conflict/validation error.
- Assert admin can hide a review and it becomes private / text cleared.

### 12.5 Coverage

Target: maintain > 80% region coverage (current gates require 81%+).  Fake-repo application tests are the fastest way to cover error paths.

---

## 13. Quality Gates

Same gates as the rest of the workspace (from `AGENTS.md`):

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo sqlx prepare --workspace
# run against local PostgreSQL on port 5432
cargo test
cargo llvm-cov --workspace --fail-under-lines 80 --fail-under-region 80
```

Run `cargo sqlx prepare --workspace` after adding the new migration and writing the Postgres repository, then commit the generated `.sqlx/` metadata.

---

## 14. Implementation Checklist

1. Add `Review`, `ReviewRating`, `ReviewText` to `domain/src/entities/review.rs`.
2. Add new `DomainError` variants and update application error mapping.
3. Add `ReviewRepository` port in `domain/src/repositories/review_repository.rs`.
4. Add `reviews` module under `application/src/reviews/` with DTOs and use cases (including admin management and deal-review status).
5. Add `PostgresReviewRepository` in `infrastructure`.
6. Add migration for unique constraint, indexes, and review scopes.
7. Add API routes, handlers, and DTOs under `api/src/handlers/reviews/` and `api/src/routes/reviews.rs`; register `pub mod reviews;` in `api/src/handlers/mod.rs`.
8. Extend `AppState` and wire construction in `api/src/main.rs`.
8b. Wire `review_repo` into `ExecuteTransition` and enforce the review completeness precondition on `EXECUTING → COMPLETED`.
9. Add fake repository to `application/src/test_helpers.rs`.
10. Write domain, application fake-repo, infrastructure, and API tests.
11. Run `cargo fmt`, `cargo clippy`, `sqlx migrate run`, `cargo sqlx prepare --workspace`, `cargo test`, and `cargo llvm-cov`.
12. Update this document if any design decisions change during implementation.

---

## 15. Open Points & Future Extensions

- **Review challenges:** A reviewed party may challenge a review; this requires a `review_challenges` table and moderation workflow.
- **Review edit window:** Allow the reviewer to edit a review within N days.  This complicates trust-score recalculation history.
- **Media attachments:** Store URLs in a `review_attachments` table.
- **Review prompts:** When a deal transitions to `COMPLETED`, send an email or in-app notification asking each party to leave reviews.
- **Weighted scoring:** The trust-score calculator will weight each review by the reviewer's own trust score, recency, and deal value (see `trust-score.md` §5.2).
- **Soft deletion:** Admins may hide reviews; the `hide` repository method is provided but no admin endpoint is required for MVP.

---

## 16. References

- `trust-score.md` — Trust Score & Transactions specification.
- `hayaland-deal-plan.md` — High-level roadmap (Phase 5: Reviews & Matching).
- `deal-plan.md` — Deal execution action plan.
- `AGENTS.md` — Project conventions and quality gates.
