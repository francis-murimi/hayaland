#[cfg(test)]
mod tests {
    use super::super::*;
    use domain::entities::{DisplayName, Email, GeoPoint, PartyType};
    use rust_decimal::Decimal;
    use uuid::Uuid;

    fn party(id: Uuid, name: &str, trust: f64) -> Party {
        let mut p = Party::new(
            id,
            PartyType::Organization,
            DisplayName::new(name).unwrap(),
            Email::new(&format!("{}@example.com", name.replace(' ', "_"))).unwrap(),
        );
        p.trust_score = trust;
        p.total_deals_completed = 5;
        p.total_deals_initiated = 5;
        p
    }

    fn resource(party_id: Uuid, category_id: Uuid, cost: Option<Decimal>) -> Resource {
        let mut r = Resource::new(
            Uuid::now_v7(),
            party_id,
            category_id,
            "Resource".to_string(),
            Decimal::from(10),
            "unit".to_string(),
        )
        .unwrap();
        r.opportunity_cost = cost;
        r
    }

    fn need(party_id: Uuid, category_id: Uuid, budget: Option<Decimal>) -> Need {
        let mut n = Need::new(
            Uuid::now_v7(),
            party_id,
            category_id,
            "Need description".to_string(),
            Decimal::from(5),
            "unit".to_string(),
        )
        .unwrap();
        n.max_budget = budget;
        n
    }

    fn enhancement(party_id: Uuid, category_id: Uuid) -> Enhancement {
        Enhancement::new(
            Uuid::now_v7(),
            party_id,
            category_id,
            "Enhancement".to_string(),
        )
        .unwrap()
    }

    #[test]
    fn score_candidate_validates_weights() {
        let supplier = party(Uuid::now_v7(), "Supplier", 0.8);
        let consumer = party(Uuid::now_v7(), "Consumer", 0.8);
        let enhancer = party(Uuid::now_v7(), "Enhancer", 0.8);
        let category = Uuid::now_v7();
        let r = resource(supplier.id, category, None);
        let n = need(consumer.id, category, None);
        let e = enhancement(enhancer.id, category);

        let inputs = CandidateInputs {
            supplier_party: &supplier,
            consumer_party: &consumer,
            enhancer_party: &enhancer,
            resources: &[r],
            needs: &[n],
            enhancements: &[e],
            supplier_trust_score: 0.8,
            consumer_trust_score: 0.8,
            enhancer_trust_score: 0.8,
        };

        let mut bad_weights = MatchScoreWeights::default();
        bad_weights.resource_need_alignment = 0.5;
        assert!(score_candidate(&inputs, bad_weights).is_err());

        let breakdown = score_candidate(&inputs, MatchScoreWeights::default()).unwrap();
        assert!(breakdown.total() >= 0.0 && breakdown.total() <= 1.0);
    }

    #[test]
    fn generate_candidates_matches_need_to_resource_and_enhancement() {
        let supplier = party(Uuid::now_v7(), "Supplier", 0.8);
        let consumer = party(Uuid::now_v7(), "Consumer", 0.8);
        let enhancer = party(Uuid::now_v7(), "Enhancer", 0.8);
        let category = Uuid::now_v7();

        let suppliers = vec![(
            supplier.clone(),
            vec![resource(supplier.id, category, None)],
        )];
        let consumers = vec![(consumer.clone(), vec![need(consumer.id, category, None)])];
        let enhancers = vec![(enhancer.clone(), vec![enhancement(enhancer.id, category)])];
        let trust = std::collections::HashMap::new();

        let suggestions = generate_candidates(
            &suppliers,
            &consumers,
            &enhancers,
            &trust,
            MatchScoreWeights::default(),
            0.0,
            10,
        )
        .unwrap();

        assert_eq!(suggestions.len(), 1);
        let s = &suggestions[0];
        assert_eq!(s.supplier_party_id, supplier.id);
        assert_eq!(s.consumer_party_id, consumer.id);
        assert_eq!(s.enhancer_party_id, enhancer.id);
        assert_eq!(s.need_category_id, Some(category));
    }

    #[test]
    fn generate_candidates_filters_by_min_score() {
        let supplier = party(Uuid::now_v7(), "Supplier", 0.1);
        let consumer = party(Uuid::now_v7(), "Consumer", 0.1);
        let enhancer = party(Uuid::now_v7(), "Enhancer", 0.1);
        let category = Uuid::now_v7();

        let suppliers = vec![(supplier, vec![resource(Uuid::now_v7(), category, None)])];
        let consumers = vec![(consumer, vec![need(Uuid::now_v7(), category, None)])];
        let enhancers = vec![(enhancer, vec![enhancement(Uuid::now_v7(), category)])];
        let trust = std::collections::HashMap::new();

        let suggestions = generate_candidates(
            &suppliers,
            &consumers,
            &enhancers,
            &trust,
            MatchScoreWeights::default(),
            0.95,
            10,
        )
        .unwrap();

        assert!(suggestions.is_empty());
    }

    #[test]
    fn generate_candidates_respects_max_suggestions() {
        let category = Uuid::now_v7();
        let mut suppliers = Vec::new();
        let mut consumers = Vec::new();
        let mut enhancers = Vec::new();
        for i in 0..3 {
            let s = party(Uuid::now_v7(), &format!("Supplier {i}"), 0.8);
            let c = party(Uuid::now_v7(), &format!("Consumer {i}"), 0.8);
            let e = party(Uuid::now_v7(), &format!("Enhancer {i}"), 0.8);
            suppliers.push((s.clone(), vec![resource(s.id, category, None)]));
            consumers.push((c.clone(), vec![need(c.id, category, None)]));
            enhancers.push((e.clone(), vec![enhancement(e.id, category)]));
        }
        let trust = std::collections::HashMap::new();

        let suggestions = generate_candidates(
            &suppliers,
            &consumers,
            &enhancers,
            &trust,
            MatchScoreWeights::default(),
            0.0,
            2,
        )
        .unwrap();

        assert_eq!(suggestions.len(), 2);
    }

    #[test]
    fn geographic_fit_high_when_close() {
        let mut supplier = party(Uuid::now_v7(), "Supplier", 0.5);
        supplier.location = Some(GeoPoint::new(0.0, 0.0).unwrap());
        supplier.service_radius_km = Some(100.0);
        let mut consumer = party(Uuid::now_v7(), "Consumer", 0.5);
        consumer.location = Some(GeoPoint::new(0.1, 0.1).unwrap());
        let mut enhancer = party(Uuid::now_v7(), "Enhancer", 0.5);
        enhancer.location = Some(GeoPoint::new(0.1, 0.0).unwrap());

        let inputs = CandidateInputs {
            supplier_party: &supplier,
            consumer_party: &consumer,
            enhancer_party: &enhancer,
            resources: &[],
            needs: &[],
            enhancements: &[],
            supplier_trust_score: 0.5,
            consumer_trust_score: 0.5,
            enhancer_trust_score: 0.5,
        };

        let breakdown = score_candidate(&inputs, MatchScoreWeights::default()).unwrap();
        assert!(
            breakdown.geographic_fit > 0.5,
            "geographic fit should be high"
        );
    }

    #[test]
    fn value_alignment_prefers_cost_below_budget() {
        let supplier = party(Uuid::now_v7(), "Supplier", 0.5);
        let consumer = party(Uuid::now_v7(), "Consumer", 0.5);
        let enhancer = party(Uuid::now_v7(), "Enhancer", 0.5);
        let category = Uuid::now_v7();

        let r = resource(supplier.id, category, Some(Decimal::from(50)));
        let n = need(consumer.id, category, Some(Decimal::from(100)));

        let inputs = CandidateInputs {
            supplier_party: &supplier,
            consumer_party: &consumer,
            enhancer_party: &enhancer,
            resources: &[r],
            needs: &[n],
            enhancements: &[],
            supplier_trust_score: 0.5,
            consumer_trust_score: 0.5,
            enhancer_trust_score: 0.5,
        };

        let breakdown = score_candidate(&inputs, MatchScoreWeights::default()).unwrap();
        assert!(breakdown.value_alignment > 0.5);
    }
}
