use domain::entities::{
    Enhancement, MatchScoreBreakdown, MatchScoreWeights, MatchSuggestion, Need, Party, Resource,
};
use domain::errors::DomainError;
use rust_decimal::prelude::{Decimal, ToPrimitive};
use std::collections::HashMap;
use uuid::Uuid;

/// Inputs needed to score a single candidate triplet.
#[derive(Debug, Clone)]
pub struct CandidateInputs<'a> {
    pub supplier_party: &'a Party,
    pub consumer_party: &'a Party,
    pub enhancer_party: &'a Party,
    pub resources: &'a [Resource],
    pub needs: &'a [Need],
    pub enhancements: &'a [Enhancement],
    pub supplier_trust_score: f64,
    pub consumer_trust_score: f64,
    pub enhancer_trust_score: f64,
}

/// Score a candidate triplet across the seven compatibility dimensions.
pub fn score_candidate(
    inputs: &CandidateInputs<'_>,
    weights: MatchScoreWeights,
) -> Result<MatchScoreBreakdown, DomainError> {
    weights.validate()?;

    let resource_need = score_resource_need_alignment(inputs);
    let geographic = score_geographic_fit(inputs);
    let temporal = score_temporal_availability(inputs);
    let trust = score_trust(inputs);
    let value = score_value_alignment(inputs);
    let historical = score_historical_success(inputs);
    let risk = score_risk_profile(inputs);

    Ok(MatchScoreBreakdown::new(
        [
            resource_need,
            value,
            trust,
            geographic,
            temporal,
            historical,
            risk,
        ],
        weights,
    ))
}

/// Generate candidate match suggestions from catalog items.
pub fn generate_candidates(
    suppliers: &[(Party, Vec<Resource>)],
    consumers: &[(Party, Vec<Need>)],
    enhancers: &[(Party, Vec<Enhancement>)],
    trust_scores: &HashMap<Uuid, f64>,
    weights: MatchScoreWeights,
    min_score: f64,
    max_suggestions: usize,
) -> Result<Vec<MatchSuggestion>, DomainError> {
    let mut suggestions = Vec::new();

    for (consumer_party, needs) in consumers {
        for need in needs.iter().filter(|n| n.is_active) {
            let matching_suppliers: Vec<_> = suppliers
                .iter()
                .filter(|(_, resources)| {
                    resources
                        .iter()
                        .any(|r| r.is_active && r.resource_type_id == need.need_category_id)
                })
                .collect();

            let matching_enhancers: Vec<_> = enhancers
                .iter()
                .filter(|(_, enhancements)| {
                    enhancements
                        .iter()
                        .any(|e| e.is_active && e.enhancement_type_id == need.need_category_id)
                })
                .collect();

            for (supplier_party, resources) in &matching_suppliers {
                for (enhancer_party, enhancements) in &matching_enhancers {
                    if supplier_party.id == consumer_party.id
                        || supplier_party.id == enhancer_party.id
                        || consumer_party.id == enhancer_party.id
                    {
                        continue;
                    }

                    let candidate_resources: Vec<_> = resources
                        .iter()
                        .filter(|r| r.resource_type_id == need.need_category_id)
                        .cloned()
                        .collect();
                    let candidate_enhancements: Vec<_> = enhancements
                        .iter()
                        .filter(|e| e.enhancement_type_id == need.need_category_id)
                        .cloned()
                        .collect();

                    let inputs = CandidateInputs {
                        supplier_party,
                        consumer_party,
                        enhancer_party,
                        resources: &candidate_resources,
                        needs: std::slice::from_ref(need),
                        enhancements: &candidate_enhancements,
                        supplier_trust_score: *trust_scores.get(&supplier_party.id).unwrap_or(&0.5),
                        consumer_trust_score: *trust_scores.get(&consumer_party.id).unwrap_or(&0.5),
                        enhancer_trust_score: *trust_scores.get(&enhancer_party.id).unwrap_or(&0.5),
                    };

                    let breakdown = score_candidate(&inputs, weights)?;
                    let total = breakdown.total();
                    if total >= min_score {
                        let reason = format!(
                            "Supplier {} offers resources matching need category {} for consumer {} with enhancement from {}",
                            supplier_party.display_name.as_str(),
                            need.need_category_id,
                            consumer_party.display_name.as_str(),
                            enhancer_party.display_name.as_str()
                        );
                        let mut suggestion = MatchSuggestion::new(
                            Uuid::now_v7(),
                            supplier_party.id,
                            consumer_party.id,
                            enhancer_party.id,
                            breakdown,
                            reason,
                        )?;
                        suggestion.set_categories(
                            Some(need.need_category_id),
                            Some(need.need_category_id),
                            Some(need.need_category_id),
                        );
                        if let Some(value) = estimate_deal_value(&inputs) {
                            suggestion.set_suggested_deal_value(value);
                        }
                        suggestions.push(suggestion);
                    }
                }
            }
        }
    }

    suggestions.sort_by(|a, b| b.match_score.partial_cmp(&a.match_score).unwrap());
    suggestions.truncate(max_suggestions);
    Ok(suggestions)
}

fn score_resource_need_alignment(inputs: &CandidateInputs<'_>) -> f64 {
    if inputs.needs.is_empty() || inputs.resources.is_empty() {
        return 0.0;
    }

    let need_category_ids: std::collections::HashSet<_> =
        inputs.needs.iter().map(|n| n.need_category_id).collect();
    let matching_resources = inputs
        .resources
        .iter()
        .filter(|r| need_category_ids.contains(&r.resource_type_id))
        .count();

    if matching_resources == 0 {
        return 0.0;
    }

    let total_resources = inputs.resources.len().max(1);
    let ratio = matching_resources as f64 / total_resources as f64;
    (0.5 + 0.5 * ratio).min(1.0)
}

fn score_geographic_fit(inputs: &CandidateInputs<'_>) -> f64 {
    let locations: Vec<_> = [
        inputs.supplier_party.location,
        inputs.consumer_party.location,
        inputs.enhancer_party.location,
    ]
    .into_iter()
    .flatten()
    .collect();

    if locations.len() < 2 {
        return 0.5;
    }

    let mut distances = Vec::new();
    for i in 0..locations.len() {
        for j in (i + 1)..locations.len() {
            distances.push(haversine_km(locations[i], locations[j]));
        }
    }

    let avg_distance = distances.iter().sum::<f64>() / distances.len() as f64;
    let radius = inputs.supplier_party.service_radius_km.unwrap_or(50.0);
    if avg_distance <= radius {
        1.0 - (avg_distance / (radius * 2.0)).min(1.0)
    } else {
        (1.0 - (avg_distance - radius) / 200.0).max(0.0)
    }
}

fn score_temporal_availability(inputs: &CandidateInputs<'_>) -> f64 {
    let need_dates: Vec<_> = inputs
        .needs
        .iter()
        .filter_map(|n| n.required_by_date)
        .collect();
    let resource_ranges: Vec<_> = inputs
        .resources
        .iter()
        .filter_map(|r| r.availability_start.zip(r.availability_end))
        .collect();

    if need_dates.is_empty() && resource_ranges.is_empty() {
        return 0.5;
    }

    if need_dates.is_empty() || resource_ranges.is_empty() {
        return 0.3;
    }

    let mut matches = 0;
    for need_date in &need_dates {
        if resource_ranges
            .iter()
            .any(|(start, end)| need_date >= start && need_date <= end)
        {
            matches += 1;
        }
    }

    matches as f64 / need_dates.len() as f64
}

fn score_trust(inputs: &CandidateInputs<'_>) -> f64 {
    let scores = [
        inputs.supplier_trust_score,
        inputs.consumer_trust_score,
        inputs.enhancer_trust_score,
    ];
    scores.iter().sum::<f64>() / scores.len() as f64
}

fn score_value_alignment(inputs: &CandidateInputs<'_>) -> f64 {
    let max_budgets: Vec<_> = inputs
        .needs
        .iter()
        .filter_map(|n| n.max_budget)
        .filter(|b| *b > Decimal::ZERO)
        .collect();
    let costs: Vec<_> = inputs
        .resources
        .iter()
        .filter_map(|r| r.opportunity_cost)
        .filter(|c| *c > Decimal::ZERO)
        .collect();

    if max_budgets.is_empty() || costs.is_empty() {
        return 0.5;
    }

    let avg_budget = max_budgets.iter().sum::<Decimal>() / Decimal::from(max_budgets.len() as i64);
    let avg_cost = costs.iter().sum::<Decimal>() / Decimal::from(costs.len() as i64);
    let budget_f64 = avg_budget.to_f64().unwrap_or(1.0);

    if avg_cost <= avg_budget {
        let diff_f64 = (avg_budget - avg_cost)
            .max(Decimal::ZERO)
            .min(avg_budget)
            .to_f64()
            .unwrap_or(0.0);
        0.7 + 0.3 * diff_f64 / budget_f64.max(1e-9)
    } else {
        let overshoot = avg_cost - avg_budget;
        let overshoot_f64 = overshoot.to_f64().unwrap_or(0.0);
        (1.0 - overshoot_f64 / budget_f64.max(1e-9)).max(0.0)
    }
}

fn score_historical_success(inputs: &CandidateInputs<'_>) -> f64 {
    let parties = [
        inputs.supplier_party,
        inputs.consumer_party,
        inputs.enhancer_party,
    ];
    let mut scores = Vec::new();
    for party in &parties {
        let completed = party.total_deals_completed.max(0) as f64;
        let initiated = (party.total_deals_initiated.max(1)) as f64;
        scores.push((completed / initiated).min(1.0));
    }
    scores.iter().sum::<f64>() / scores.len() as f64
}

fn score_risk_profile(inputs: &CandidateInputs<'_>) -> f64 {
    let trust = score_trust(inputs);
    let historical = score_historical_success(inputs);
    (trust + historical) / 2.0
}

fn estimate_deal_value(inputs: &CandidateInputs<'_>) -> Option<Decimal> {
    let total_budget: Decimal = inputs.needs.iter().filter_map(|n| n.max_budget).sum();
    let total_cost: Decimal = inputs
        .resources
        .iter()
        .filter_map(|r| r.opportunity_cost)
        .sum();

    if total_budget > Decimal::ZERO {
        Some(total_budget.max(total_cost))
    } else if total_cost > Decimal::ZERO {
        Some(total_cost)
    } else {
        None
    }
}

fn haversine_km(a: domain::entities::GeoPoint, b: domain::entities::GeoPoint) -> f64 {
    let r = 6371.0;
    let d_lat = (b.latitude - a.latitude).to_radians();
    let d_lon = (b.longitude - a.longitude).to_radians();
    let lat1 = a.latitude.to_radians();
    let lat2 = b.latitude.to_radians();

    let sin_d_lat = (d_lat / 2.0).sin();
    let sin_d_lon = (d_lon / 2.0).sin();
    let h = sin_d_lat * sin_d_lat + lat1.cos() * lat2.cos() * sin_d_lon * sin_d_lon;
    2.0 * r * h.sqrt().atan2((1.0 - h).sqrt())
}

#[cfg(test)]
#[path = "discovery_engine_tests.rs"]
mod tests;
