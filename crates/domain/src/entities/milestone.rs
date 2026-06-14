use crate::errors::DomainError;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// Lifecycle status of a deal milestone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MilestoneStatus {
    Pending,
    InProgress,
    Completed,
    Verified,
    Missed,
}

impl MilestoneStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            MilestoneStatus::Pending => "PENDING",
            MilestoneStatus::InProgress => "IN_PROGRESS",
            MilestoneStatus::Completed => "COMPLETED",
            MilestoneStatus::Verified => "VERIFIED",
            MilestoneStatus::Missed => "MISSED",
        }
    }
}

impl TryFrom<&str> for MilestoneStatus {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "PENDING" => Ok(MilestoneStatus::Pending),
            "IN_PROGRESS" => Ok(MilestoneStatus::InProgress),
            "COMPLETED" => Ok(MilestoneStatus::Completed),
            "VERIFIED" => Ok(MilestoneStatus::Verified),
            "MISSED" => Ok(MilestoneStatus::Missed),
            _ => Err(DomainError::Validation(vec![format!(
                "unknown milestone status: {value}"
            )])),
        }
    }
}

/// A single deliverable within a deal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Milestone {
    pub id: Uuid,
    pub deal_id: Uuid,
    pub milestone_name: String,
    pub description: Option<String>,
    pub assigned_to_party_id: Uuid,
    pub verified_by_party_id: Uuid,
    pub due_date: Option<time::Date>,
    pub completion_criteria: String,
    pub milestone_status: MilestoneStatus,
    pub completion_percentage: Decimal,
    pub payment_trigger_amount: Option<Decimal>,
    pub completed_at: Option<OffsetDateTime>,
    pub display_order: i32,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl Milestone {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: Uuid,
        deal_id: Uuid,
        milestone_name: String,
        description: Option<String>,
        assigned_to_party_id: Uuid,
        verified_by_party_id: Uuid,
        due_date: Option<time::Date>,
        completion_criteria: String,
        payment_trigger_amount: Option<Decimal>,
        display_order: i32,
    ) -> Result<Self, DomainError> {
        Self::validate_name(&milestone_name)?;
        Self::validate_criteria(&completion_criteria)?;
        if let Some(amount) = payment_trigger_amount {
            if amount <= Decimal::ZERO {
                return Err(DomainError::Validation(vec![
                    "payment trigger amount must be positive".to_string(),
                ]));
            }
        }
        if assigned_to_party_id == verified_by_party_id {
            return Err(DomainError::Validation(vec![
                "assigned party and verifier must be different".to_string(),
            ]));
        }

        let now = OffsetDateTime::now_utc();
        Ok(Self {
            id,
            deal_id,
            milestone_name,
            description,
            assigned_to_party_id,
            verified_by_party_id,
            due_date,
            completion_criteria,
            milestone_status: MilestoneStatus::Pending,
            completion_percentage: Decimal::ZERO,
            payment_trigger_amount,
            completed_at: None,
            display_order,
            created_at: now,
            updated_at: now,
        })
    }

    fn validate_name(name: &str) -> Result<(), DomainError> {
        let len = name.trim().chars().count();
        if !(3..=200).contains(&len) {
            return Err(DomainError::Validation(vec![
                "milestone name must be between 3 and 200 characters".to_string(),
            ]));
        }
        Ok(())
    }

    fn validate_criteria(criteria: &str) -> Result<(), DomainError> {
        if criteria.trim().is_empty() {
            return Err(DomainError::Validation(vec![
                "completion criteria must not be empty".to_string(),
            ]));
        }
        Ok(())
    }

    pub fn set_name(&mut self, name: String) -> Result<(), DomainError> {
        Self::validate_name(&name)?;
        self.milestone_name = name;
        self.updated_at = OffsetDateTime::now_utc();
        Ok(())
    }

    pub fn set_completion_criteria(&mut self, criteria: String) -> Result<(), DomainError> {
        Self::validate_criteria(&criteria)?;
        self.completion_criteria = criteria;
        self.updated_at = OffsetDateTime::now_utc();
        Ok(())
    }

    pub fn set_payment_trigger_amount(
        &mut self,
        amount: Option<Decimal>,
    ) -> Result<(), DomainError> {
        if let Some(a) = amount {
            if a <= Decimal::ZERO {
                return Err(DomainError::Validation(vec![
                    "payment trigger amount must be positive".to_string(),
                ]));
            }
        }
        self.payment_trigger_amount = amount;
        self.updated_at = OffsetDateTime::now_utc();
        Ok(())
    }

    pub fn start(&mut self, party_id: Uuid) -> Result<(), DomainError> {
        self.ensure_mutable()?;
        if party_id != self.assigned_to_party_id {
            return Err(DomainError::InsufficientPermissions);
        }
        if self.milestone_status != MilestoneStatus::Pending {
            return Err(DomainError::Validation(vec![
                "only pending milestones can be started".to_string(),
            ]));
        }
        self.milestone_status = MilestoneStatus::InProgress;
        self.completion_percentage = Decimal::from(25);
        self.updated_at = OffsetDateTime::now_utc();
        Ok(())
    }

    pub fn update_progress(
        &mut self,
        party_id: Uuid,
        percentage: Decimal,
    ) -> Result<(), DomainError> {
        self.ensure_mutable()?;
        if party_id != self.assigned_to_party_id {
            return Err(DomainError::InsufficientPermissions);
        }
        if percentage < Decimal::ZERO || percentage > Decimal::from(100) {
            return Err(DomainError::Validation(vec![
                "completion percentage must be between 0 and 100".to_string(),
            ]));
        }
        self.completion_percentage = percentage;
        self.updated_at = OffsetDateTime::now_utc();
        Ok(())
    }

    pub fn complete(&mut self, party_id: Uuid) -> Result<(), DomainError> {
        self.ensure_mutable()?;
        if party_id != self.assigned_to_party_id {
            return Err(DomainError::InsufficientPermissions);
        }
        if self.milestone_status != MilestoneStatus::InProgress {
            return Err(DomainError::Validation(vec![
                "only in-progress milestones can be completed".to_string(),
            ]));
        }
        self.milestone_status = MilestoneStatus::Completed;
        self.completion_percentage = Decimal::from(100);
        self.completed_at = Some(OffsetDateTime::now_utc());
        self.updated_at = OffsetDateTime::now_utc();
        Ok(())
    }

    pub fn verify(&mut self, party_id: Uuid) -> Result<(), DomainError> {
        self.ensure_mutable()?;
        if party_id != self.verified_by_party_id {
            return Err(DomainError::InsufficientPermissions);
        }
        if self.milestone_status != MilestoneStatus::Completed {
            return Err(DomainError::Validation(vec![
                "only completed milestones can be verified".to_string(),
            ]));
        }
        self.milestone_status = MilestoneStatus::Verified;
        self.updated_at = OffsetDateTime::now_utc();
        Ok(())
    }

    pub fn mark_missed(&mut self) -> Result<(), DomainError> {
        self.ensure_mutable()?;
        if self.milestone_status != MilestoneStatus::Pending
            && self.milestone_status != MilestoneStatus::InProgress
        {
            return Err(DomainError::Validation(vec![
                "only pending or in-progress milestones can be marked missed".to_string(),
            ]));
        }
        self.milestone_status = MilestoneStatus::Missed;
        self.updated_at = OffsetDateTime::now_utc();
        Ok(())
    }

    fn ensure_mutable(&self) -> Result<(), DomainError> {
        if self.milestone_status == MilestoneStatus::Verified {
            return Err(DomainError::Validation(vec![
                "verified milestones cannot be modified".to_string(),
            ]));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_milestone() -> Milestone {
        Milestone::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            "Build prototype".to_string(),
            None,
            Uuid::now_v7(),
            Uuid::now_v7(),
            None,
            "Working prototype delivered".to_string(),
            Some(Decimal::from(100)),
            1,
        )
        .unwrap()
    }

    #[test]
    fn milestone_status_round_trips() {
        for status in [
            MilestoneStatus::Pending,
            MilestoneStatus::InProgress,
            MilestoneStatus::Completed,
            MilestoneStatus::Verified,
            MilestoneStatus::Missed,
        ] {
            assert_eq!(MilestoneStatus::try_from(status.as_str()).unwrap(), status);
        }
    }

    #[test]
    fn rejects_short_name_and_empty_criteria() {
        let assigned = Uuid::now_v7();
        let verifier = Uuid::now_v7();
        assert!(Milestone::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            "AB".to_string(),
            None,
            assigned,
            verifier,
            None,
            "criteria".to_string(),
            None,
            1,
        )
        .is_err());

        assert!(Milestone::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            "Valid name".to_string(),
            None,
            assigned,
            verifier,
            None,
            "".to_string(),
            None,
            1,
        )
        .is_err());
    }

    #[test]
    fn rejects_same_assigned_and_verifier() {
        let party_id = Uuid::now_v7();
        assert!(Milestone::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            "Name".to_string(),
            None,
            party_id,
            party_id,
            None,
            "criteria".to_string(),
            None,
            1,
        )
        .is_err());
    }

    #[test]
    fn lifecycle_transitions() {
        let assigned = Uuid::now_v7();
        let verifier = Uuid::now_v7();
        let mut m = Milestone::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            "Name".to_string(),
            None,
            assigned,
            verifier,
            None,
            "criteria".to_string(),
            None,
            1,
        )
        .unwrap();

        assert_eq!(m.milestone_status, MilestoneStatus::Pending);
        m.start(assigned).unwrap();
        assert_eq!(m.milestone_status, MilestoneStatus::InProgress);
        m.update_progress(assigned, Decimal::from(75)).unwrap();
        assert_eq!(m.completion_percentage, Decimal::from(75));
        m.complete(assigned).unwrap();
        assert_eq!(m.milestone_status, MilestoneStatus::Completed);
        m.verify(verifier).unwrap();
        assert_eq!(m.milestone_status, MilestoneStatus::Verified);
        assert!(m.start(assigned).is_err());
    }

    #[test]
    fn wrong_party_cannot_act() {
        let mut m = sample_milestone();
        let other = Uuid::now_v7();
        assert!(m.start(other).is_err());
        assert!(m.complete(other).is_err());
        assert!(m.verify(other).is_err());
    }

    #[test]
    fn mark_missed_only_from_pending_or_in_progress() {
        let mut m = sample_milestone();
        m.mark_missed().unwrap();
        assert_eq!(m.milestone_status, MilestoneStatus::Missed);

        let mut m = sample_milestone();
        m.start(m.assigned_to_party_id).unwrap();
        m.mark_missed().unwrap();
        assert_eq!(m.milestone_status, MilestoneStatus::Missed);

        let mut m = sample_milestone();
        m.start(m.assigned_to_party_id).unwrap();
        m.complete(m.assigned_to_party_id).unwrap();
        assert!(m.mark_missed().is_err());
    }
}
