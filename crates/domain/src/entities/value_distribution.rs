use crate::entities::DealRole;
use crate::errors::DomainError;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// How the total deal value is structured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DistributionModel {
    FixedPrice,
    RevenueShare,
    CostPlus,
    Barter,
    Hybrid,
}

impl DistributionModel {
    pub fn as_str(&self) -> &'static str {
        match self {
            DistributionModel::FixedPrice => "FIXED_PRICE",
            DistributionModel::RevenueShare => "REVENUE_SHARE",
            DistributionModel::CostPlus => "COST_PLUS",
            DistributionModel::Barter => "BARTER",
            DistributionModel::Hybrid => "HYBRID",
        }
    }
}

impl TryFrom<&str> for DistributionModel {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "FIXED_PRICE" => Ok(DistributionModel::FixedPrice),
            "REVENUE_SHARE" => Ok(DistributionModel::RevenueShare),
            "COST_PLUS" => Ok(DistributionModel::CostPlus),
            "BARTER" => Ok(DistributionModel::Barter),
            "HYBRID" => Ok(DistributionModel::Hybrid),
            _ => Err(DomainError::InvalidValueDistribution {
                message: format!("unknown distribution model: {value}"),
            }),
        }
    }
}

/// When a scheduled payment is triggered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaymentTrigger {
    Upfront,
    Milestone,
    OnDelivery,
    Deferred,
}

impl PaymentTrigger {
    pub fn as_str(&self) -> &'static str {
        match self {
            PaymentTrigger::Upfront => "UPFRONT",
            PaymentTrigger::Milestone => "MILESTONE",
            PaymentTrigger::OnDelivery => "ON_DELIVERY",
            PaymentTrigger::Deferred => "DEFERRED",
        }
    }
}

impl TryFrom<&str> for PaymentTrigger {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "UPFRONT" => Ok(PaymentTrigger::Upfront),
            "MILESTONE" => Ok(PaymentTrigger::Milestone),
            "ON_DELIVERY" => Ok(PaymentTrigger::OnDelivery),
            "DEFERRED" => Ok(PaymentTrigger::Deferred),
            _ => Err(DomainError::InvalidValueDistribution {
                message: format!("unknown payment trigger: {value}"),
            }),
        }
    }
}

/// A single entry in the payment schedule.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaymentScheduleEntry {
    pub sequence: i32,
    pub trigger: PaymentTrigger,
    pub due_at: Option<time::Date>,
    pub amount: Decimal,
    pub recipient_role: DealRole,
    pub milestone_id: Option<Uuid>,
}

/// Allocation of total deal value among the platform and the three parties.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValueDistribution {
    pub id: Uuid,
    pub deal_id: Uuid,
    pub total_value: Decimal,
    pub currency: String,
    pub distribution_model: DistributionModel,
    pub supplier_share_percentage: Decimal,
    pub supplier_share_amount: Decimal,
    pub consumer_cost_percentage: Decimal,
    pub consumer_cost_amount: Decimal,
    pub enhancer_share_percentage: Decimal,
    pub enhancer_share_amount: Decimal,
    pub platform_fee_percentage: Decimal,
    pub platform_fee_amount: Decimal,
    pub payment_schedule: Vec<PaymentScheduleEntry>,
    pub win_win_win_score: Option<Decimal>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl ValueDistribution {
    /// Validate the invariants of the distribution.
    pub fn validate(&self) -> Result<(), DomainError> {
        let epsilon = Decimal::from(1) / Decimal::from(10000);

        let sum_shares = self.supplier_share_percentage
            + self.enhancer_share_percentage
            + self.platform_fee_percentage;
        if (sum_shares - Decimal::from(100)).abs() > epsilon {
            return Err(DomainError::InvalidValueDistribution {
                message: format!("share percentages must sum to 100, got {}", sum_shares),
            });
        }

        let sum_amounts =
            self.supplier_share_amount + self.enhancer_share_amount + self.platform_fee_amount;
        if (sum_amounts - self.total_value).abs() > epsilon {
            return Err(DomainError::InvalidValueDistribution {
                message: "share amounts must sum to total value".to_string(),
            });
        }

        if self.consumer_cost_amount > self.total_value {
            return Err(DomainError::InvalidValueDistribution {
                message: "consumer cost cannot exceed total value".to_string(),
            });
        }

        if self.supplier_share_percentage <= Decimal::ZERO
            || self.enhancer_share_percentage <= Decimal::ZERO
        {
            return Err(DomainError::InvalidValueDistribution {
                message: "supplier and enhancer shares must be positive".to_string(),
            });
        }

        if self.platform_fee_percentage < Decimal::ZERO {
            return Err(DomainError::InvalidValueDistribution {
                message: "platform fee cannot be negative".to_string(),
            });
        }

        Ok(())
    }

    /// Recompute derived amounts from percentages and total value.
    pub fn recalculate_amounts(&mut self) {
        self.supplier_share_amount =
            (self.total_value * self.supplier_share_percentage) / Decimal::from(100);
        self.enhancer_share_amount =
            (self.total_value * self.enhancer_share_percentage) / Decimal::from(100);
        self.platform_fee_amount =
            (self.total_value * self.platform_fee_percentage) / Decimal::from(100);
        self.consumer_cost_amount =
            (self.total_value * self.consumer_cost_percentage) / Decimal::from(100);
        self.updated_at = OffsetDateTime::now_utc();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_distribution() -> ValueDistribution {
        ValueDistribution {
            id: Uuid::now_v7(),
            deal_id: Uuid::now_v7(),
            total_value: Decimal::from(10000),
            currency: "POINTS".to_string(),
            distribution_model: DistributionModel::FixedPrice,
            supplier_share_percentage: Decimal::from(60),
            supplier_share_amount: Decimal::from(6000),
            consumer_cost_percentage: Decimal::from(100),
            consumer_cost_amount: Decimal::from(10000),
            enhancer_share_percentage: Decimal::from(30),
            enhancer_share_amount: Decimal::from(3000),
            platform_fee_percentage: Decimal::from(10),
            platform_fee_amount: Decimal::from(1000),
            payment_schedule: vec![],
            win_win_win_score: None,
            created_at: OffsetDateTime::now_utc(),
            updated_at: OffsetDateTime::now_utc(),
        }
    }

    #[test]
    fn valid_distribution_passes() {
        assert!(valid_distribution().validate().is_ok());
    }

    #[test]
    fn rejects_percentage_sum_not_100() {
        let mut vd = valid_distribution();
        vd.supplier_share_percentage = Decimal::from(50);
        assert!(vd.validate().is_err());
    }

    #[test]
    fn rejects_amounts_not_matching_total() {
        let mut vd = valid_distribution();
        vd.supplier_share_amount = Decimal::from(5000);
        assert!(vd.validate().is_err());
    }

    #[test]
    fn rejects_negative_shares() {
        let mut vd = valid_distribution();
        vd.enhancer_share_percentage = Decimal::from(-5);
        assert!(vd.validate().is_err());
    }

    #[test]
    fn recalculate_amounts_is_consistent() {
        let mut vd = valid_distribution();
        vd.supplier_share_amount = Decimal::ZERO;
        vd.recalculate_amounts();
        assert_eq!(vd.supplier_share_amount, Decimal::from(6000));
        assert_eq!(vd.enhancer_share_amount, Decimal::from(3000));
        assert_eq!(vd.platform_fee_amount, Decimal::from(1000));
        assert!(vd.validate().is_ok());
    }
}
