use domain::entities::trust_score::TrustScoreConfig;
use domain::entities::{DealRole, Party, RoleProfile};

/// Compute a 0–100 profile-completeness score for a party.
pub struct ProfileCompletenessCalculator;

impl ProfileCompletenessCalculator {
    pub fn calculate(
        party: &Party,
        roles: &[(DealRole, RoleProfile)],
        config: &TrustScoreConfig,
    ) -> f64 {
        let cfg = &config.profile_completeness;
        let mut score = 0.0;

        // Basic info: display name, email, phone.
        let mut basic = 0.0;
        if !party.display_name.as_str().is_empty() {
            basic += cfg.basic_info_points / 2.0;
        }
        if !party.email.as_str().is_empty() {
            basic += cfg.basic_info_points / 4.0;
        }
        if party.phone.is_some() {
            basic += cfg.basic_info_points / 4.0;
        }
        score += basic.min(cfg.basic_info_points);

        // Location.
        if party.location.is_some() {
            score += cfg.location_points;
        }

        // Business details.
        if party.tax_id.is_some() {
            score += cfg.business_details_points / 2.0;
        }
        if party.primary_domain_id.is_some() {
            score += cfg.business_details_points / 2.0;
        }

        // Service radius / preferences.
        if party.service_radius_km.is_some() {
            score += cfg.service_radius_points / 2.0;
        }

        // Role profiles.
        let role_score = Self::role_profile_score(roles, cfg.role_profile_points);
        // Only award service-radius second half if at least one role has preferences.
        if roles.iter().any(|(_, profile)| has_preferences(profile)) {
            score += cfg.service_radius_points / 2.0;
        }
        score += role_score;

        score.clamp(0.0, 100.0)
    }

    fn role_profile_score(roles: &[(DealRole, RoleProfile)], max_per_role: f64) -> f64 {
        if roles.is_empty() {
            return 0.0;
        }

        let total: f64 = roles
            .iter()
            .map(|(_, profile)| {
                let populated = populated_role_fields(profile);
                let total = total_role_fields(profile).max(1);
                max_per_role * (populated as f64 / total as f64)
            })
            .sum();

        // Average across roles, but cap at max_per_role per role.
        total
    }
}

fn has_preferences(profile: &RoleProfile) -> bool {
    match profile {
        RoleProfile::Supplier(p) => {
            p.availability_schedule.is_some() || p.typical_capacity.is_some()
        }
        RoleProfile::Consumer(p) => {
            p.budget_range_min.is_some()
                || p.budget_range_max.is_some()
                || !p.preferred_payment_terms.is_empty()
        }
        RoleProfile::Enhancer(p) => {
            p.availability.is_some()
                || p.hourly_rate.is_some()
                || p.fixed_rate.is_some()
                || p.typical_engagement_duration.is_some()
        }
    }
}

fn total_role_fields(profile: &RoleProfile) -> usize {
    match profile {
        RoleProfile::Supplier(_) => 5,
        RoleProfile::Consumer(_) => 6,
        RoleProfile::Enhancer(_) => 8,
    }
}

fn populated_role_fields(profile: &RoleProfile) -> usize {
    match profile {
        RoleProfile::Supplier(p) => {
            let mut count = 0;
            if p.resource_type_ids.len() >= 3 {
                count += 1;
            }
            if p.typical_capacity.is_some() {
                count += 1;
            }
            if p.availability_schedule.is_some() {
                count += 1;
            }
            if !p.preferred_compensation.is_empty() {
                count += 1;
            }
            if p.insurance_verified {
                count += 1;
            }
            count
        }
        RoleProfile::Consumer(p) => {
            let mut count = 0;
            if p.need_category_ids.len() >= 3 {
                count += 1;
            }
            if p.typical_volume.is_some() {
                count += 1;
            }
            if p.preferred_quality_standard.is_some() {
                count += 1;
            }
            if p.budget_range_min.is_some() || p.budget_range_max.is_some() {
                count += 1;
            }
            if !p.preferred_payment_terms.is_empty() {
                count += 1;
            }
            count
        }
        RoleProfile::Enhancer(p) => {
            let mut count = 0;
            if p.enhancement_type_ids.len() >= 3 {
                count += 1;
            }
            if p.skills.len() >= 3 {
                count += 1;
            }
            if p.certifications.is_some() {
                count += 1;
            }
            if p.hourly_rate.is_some() || p.fixed_rate.is_some() {
                count += 1;
            }
            if !p.equipment_owned.is_empty() {
                count += 1;
            }
            if p.availability.is_some() {
                count += 1;
            }
            if p.typical_engagement_duration.is_some() {
                count += 1;
            }
            count
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::entities::{DisplayName, Email, GeoPoint, Party, PartyType, RoleProfile};
    use uuid::Uuid;

    fn empty_party() -> Party {
        Party::new(
            Uuid::now_v7(),
            PartyType::Organization,
            DisplayName::new("Test").unwrap(),
            Email::new("test@example.com").unwrap(),
        )
    }

    fn default_config() -> TrustScoreConfig {
        TrustScoreConfig::default()
    }

    #[test]
    fn empty_party_has_partial_basic_info() {
        let party = empty_party();
        let score = ProfileCompletenessCalculator::calculate(&party, &[], &default_config());
        // display_name + email = 15 / 20
        assert_eq!(score, 15.0);
    }

    #[test]
    fn complete_party_scores_high() {
        let mut party = empty_party();
        party.phone = Some(domain::entities::Phone::new("+1234567890").unwrap());
        party.location = Some(GeoPoint::new(1.0, 2.0).unwrap());
        party.tax_id = Some("TAX".to_string());
        party.primary_domain_id = Some(Uuid::now_v7());
        party.service_radius_km = Some(10.0);

        let role_pair = (
            domain::entities::DealRole::Supplier,
            RoleProfile::Supplier(domain::entities::SupplierProfile {
                resource_type_ids: vec![Uuid::now_v7(), Uuid::now_v7(), Uuid::now_v7()],
                typical_capacity: Some("high".to_string()),
                availability_schedule: Some(serde_json::json!({})),
                preferred_compensation: vec!["cash".to_string()],
                insurance_verified: true,
            }),
        );

        let score =
            ProfileCompletenessCalculator::calculate(&party, &[role_pair], &default_config());
        assert!(score >= 90.0);
    }

    #[test]
    fn phone_only_adds_quarter_of_basic_points() {
        let mut party = empty_party();
        party.phone = Some(domain::entities::Phone::new("+1234567890").unwrap());
        // 15 (name+email) + 5 (phone) = 20 (capped at basic_info_points)
        let score = ProfileCompletenessCalculator::calculate(&party, &[], &default_config());
        assert_eq!(score, 20.0);
    }

    #[test]
    fn location_and_business_details_add_points() {
        let mut party = empty_party();
        party.location = Some(GeoPoint::new(1.0, 2.0).unwrap());
        party.tax_id = Some("TAX".to_string());
        party.primary_domain_id = Some(Uuid::now_v7());
        party.service_radius_km = Some(5.0);
        // 15 basic + 20 location + 20 business + 5 radius (no preferences) = 60
        let score = ProfileCompletenessCalculator::calculate(&party, &[], &default_config());
        assert_eq!(score, 60.0);
    }

    #[test]
    fn consumer_profile_with_preferences_scores_role_and_radius_bonus() {
        let mut party = empty_party();
        party.service_radius_km = Some(5.0);
        let consumer = RoleProfile::Consumer(domain::entities::ConsumerProfile {
            need_category_ids: vec![Uuid::now_v7(), Uuid::now_v7(), Uuid::now_v7()],
            typical_volume: Some("10 tons".to_string()),
            preferred_quality_standard: Some("organic".to_string()),
            budget_range_min: Some(100.0),
            budget_range_max: None,
            preferred_payment_terms: vec!["net30".to_string()],
        });
        let score = ProfileCompletenessCalculator::calculate(
            &party,
            &[(domain::entities::DealRole::Consumer, consumer)],
            &default_config(),
        );
        // 15 basic + 5 radius + 5 radius-bonus + 25 role (5/6 fields) = 50
        assert_eq!(score, 50.0);
    }

    #[test]
    fn enhancer_profile_full_scores_full_role_points() {
        let party = empty_party();
        let enhancer = RoleProfile::Enhancer(domain::entities::EnhancerProfile {
            enhancement_type_ids: vec![Uuid::now_v7(), Uuid::now_v7(), Uuid::now_v7()],
            skills: vec!["a".to_string(), "b".to_string(), "c".to_string()],
            certifications: Some(serde_json::json!(["ISO"])),
            hourly_rate: Some(50.0),
            fixed_rate: None,
            equipment_owned: vec!["drill".to_string()],
            availability: Some(serde_json::json!({"days": "weekdays"})),
            typical_engagement_duration: Some("2 weeks".to_string()),
        });
        let score = ProfileCompletenessCalculator::calculate(
            &party,
            &[(domain::entities::DealRole::Enhancer, enhancer)],
            &default_config(),
        );
        // 15 basic + 5 radius-bonus + 26.25 role (7/8 fields) = 46.25
        assert_eq!(score, 46.25);
    }

    #[test]
    fn empty_role_profiles_score_zero_role_points() {
        let party = empty_party();
        let supplier = RoleProfile::Supplier(domain::entities::SupplierProfile::default());
        let score = ProfileCompletenessCalculator::calculate(
            &party,
            &[(domain::entities::DealRole::Supplier, supplier)],
            &default_config(),
        );
        assert_eq!(score, 15.0);
    }

    #[test]
    fn supplier_partial_fields_scale_role_points() {
        let party = empty_party();
        let supplier = RoleProfile::Supplier(domain::entities::SupplierProfile {
            resource_type_ids: vec![Uuid::now_v7(), Uuid::now_v7(), Uuid::now_v7()],
            typical_capacity: None,
            availability_schedule: None,
            preferred_compensation: vec![],
            insurance_verified: false,
        });
        // 1 of 5 supplier fields => 6 role points
        let score = ProfileCompletenessCalculator::calculate(
            &party,
            &[(domain::entities::DealRole::Supplier, supplier)],
            &default_config(),
        );
        assert_eq!(score, 21.0);
    }

    #[test]
    fn multiple_roles_average_across_roles() {
        let party = empty_party();
        let full_supplier = RoleProfile::Supplier(domain::entities::SupplierProfile {
            resource_type_ids: vec![Uuid::now_v7(), Uuid::now_v7(), Uuid::now_v7()],
            typical_capacity: Some("x".to_string()),
            availability_schedule: Some(serde_json::json!({})),
            preferred_compensation: vec!["cash".to_string()],
            insurance_verified: true,
        });
        let empty_consumer = RoleProfile::Consumer(domain::entities::ConsumerProfile::default());
        let score = ProfileCompletenessCalculator::calculate(
            &party,
            &[
                (domain::entities::DealRole::Supplier, full_supplier),
                (domain::entities::DealRole::Consumer, empty_consumer),
            ],
            &default_config(),
        );
        // 15 basic + 5 radius-bonus + (30 supplier + 0 consumer) = 50
        assert_eq!(score, 50.0);
    }

    #[test]
    fn score_is_clamped_to_100() {
        let mut config = default_config();
        config.profile_completeness.basic_info_points = 200.0;
        let mut party = empty_party();
        party.phone = Some(domain::entities::Phone::new("+1234567890").unwrap());
        party.location = Some(GeoPoint::new(1.0, 2.0).unwrap());
        let score = ProfileCompletenessCalculator::calculate(&party, &[], &config);
        assert_eq!(score, 100.0);
    }
}
