use crate::agreements::dto::{AgreementResult, GenerateAgreementCommand};
use crate::errors::ApplicationError;
use domain::entities::{
    Agreement, AgreementStatus, Deal, DealParticipation, DealStatus, Term, TermStatus,
    ValueDistribution,
};
use domain::repositories::{AgreementRepository, DealRepository, PartyRepository};
use std::collections::BTreeMap;
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

use super::dto::SignatureResult;

/// Generate or regenerate the legal agreement for a deal that has reached `TERMS_LOCKED`.
#[derive(Clone)]
pub struct GenerateAgreement {
    deal_repo: Arc<dyn DealRepository>,
    party_repo: Arc<dyn PartyRepository>,
    agreement_repo: Arc<dyn AgreementRepository>,
}

impl GenerateAgreement {
    pub fn new(
        deal_repo: Arc<dyn DealRepository>,
        party_repo: Arc<dyn PartyRepository>,
        agreement_repo: Arc<dyn AgreementRepository>,
    ) -> Self {
        Self {
            deal_repo,
            party_repo,
            agreement_repo,
        }
    }

    #[instrument(skip(self, cmd), fields(deal_id = %cmd.deal_id))]
    pub async fn execute(
        &self,
        cmd: GenerateAgreementCommand,
    ) -> Result<AgreementResult, ApplicationError> {
        let aggregate = self
            .deal_repo
            .find_aggregate_by_id(cmd.deal_id)
            .await?
            .ok_or(ApplicationError::DealNotFound)?;

        if aggregate.deal.deal_status != DealStatus::TermsLocked {
            return Err(ApplicationError::Validation(vec![format!(
                "agreement can only be generated when deal status is TERMS_LOCKED, got {}",
                aggregate.deal.deal_status.as_str()
            )]));
        }

        if !self
            .party_repo
            .is_user_member_of_party(cmd.actor_user_id, cmd.actor_party_id)
            .await?
        {
            return Err(ApplicationError::Forbidden);
        }

        if !self
            .deal_repo
            .is_party_participant(cmd.deal_id, cmd.actor_party_id)
            .await?
        {
            return Err(ApplicationError::Forbidden);
        }

        let terms = self.deal_repo.find_terms_by_deal(cmd.deal_id).await?;
        let mandatory_unaccepted = terms
            .iter()
            .any(|t| t.is_mandatory && t.negotiation_status != TermStatus::Accepted);
        if mandatory_unaccepted {
            return Err(ApplicationError::Validation(vec![
                "all mandatory terms must be accepted before generating the agreement".to_string(),
            ]));
        }

        let value_distribution = self
            .deal_repo
            .find_value_distribution_by_deal(cmd.deal_id)
            .await?
            .ok_or_else(|| {
                ApplicationError::Validation(vec![
                    "value distribution is required before generating the agreement".to_string(),
                ])
            })?;

        let parties = self.load_parties(&aggregate.participations).await?;
        let agreement_text = render_agreement_text(
            &aggregate.deal,
            &aggregate.participations,
            &parties,
            &terms,
            &value_distribution,
        );

        let existing = self.agreement_repo.find_by_deal_id(cmd.deal_id).await?;

        let agreement = match existing {
            Some(mut existing) => {
                if existing.agreement_status == AgreementStatus::Executed {
                    return Err(ApplicationError::Validation(vec![
                        "executed agreements cannot be regenerated".to_string(),
                    ]));
                }

                self.deal_repo
                    .record_history(
                        cmd.deal_id,
                        "AGREEMENT_REGENERATED",
                        Some(cmd.actor_party_id),
                        Some(serde_json::json!({
                            "previous_version": existing.version,
                            "previous_text": existing.agreement_text,
                            "new_version": existing.version + 1,
                        })),
                    )
                    .await?;

                existing.version += 1;
                existing.agreement_status = AgreementStatus::PendingSignatures;
                existing.agreement_text = agreement_text;
                existing.executed_at = None;
                existing
            }
            None => Agreement::new(
                Uuid::now_v7(),
                cmd.deal_id,
                agreement_text,
                None,
                None,
                None,
                1,
            ),
        };

        let is_new = self
            .agreement_repo
            .find_by_deal_id(cmd.deal_id)
            .await?
            .is_none();

        if is_new {
            self.agreement_repo.create(&agreement).await?;
        } else {
            self.agreement_repo.update(&agreement).await?;
        }

        self.deal_repo
            .record_history(
                cmd.deal_id,
                "AGREEMENT_GENERATED",
                Some(cmd.actor_party_id),
                Some(serde_json::json!({
                    "agreement_id": agreement.id,
                    "version": agreement.version,
                })),
            )
            .await?;

        info!(
            %agreement.id,
            deal_id = %cmd.deal_id,
            version = agreement.version,
            "generated agreement"
        );

        Ok(map_agreement_to_result(agreement, vec![]))
    }

    async fn load_parties(
        &self,
        participations: &[DealParticipation],
    ) -> Result<BTreeMap<Uuid, domain::entities::Party>, ApplicationError> {
        let mut parties = BTreeMap::new();
        for p in participations {
            if let Some(party) = self.party_repo.find_by_id(p.party_id).await? {
                parties.insert(p.party_id, party);
            }
        }
        Ok(parties)
    }
}

pub(crate) fn map_agreement_to_result(
    agreement: Agreement,
    signatures: Vec<domain::entities::Signature>,
) -> AgreementResult {
    AgreementResult {
        id: agreement.id,
        deal_id: agreement.deal_id,
        status: agreement.agreement_status,
        agreement_text: agreement.agreement_text,
        governing_law: agreement.governing_law,
        dispute_resolution: agreement.dispute_resolution,
        effective_date: agreement.effective_date,
        termination_date: agreement.termination_date,
        auto_renew: agreement.auto_renew,
        version: agreement.version,
        digital_signature_url: agreement.digital_signature_url,
        created_at: agreement.created_at,
        executed_at: agreement.executed_at,
        signatures: signatures
            .into_iter()
            .map(map_signature_to_result)
            .collect(),
    }
}

fn map_signature_to_result(signature: domain::entities::Signature) -> SignatureResult {
    SignatureResult {
        id: signature.id,
        agreement_id: signature.agreement_id,
        party_id: signature.party_id,
        signed_by_user_id: signature.signed_by_user_id,
        signature_type: signature.signature_type,
        signature_data: signature.signature_data,
        ip_address: signature.ip_address,
        signed_at: signature.signed_at,
        version: signature.version,
    }
}

fn render_agreement_text(
    deal: &Deal,
    participations: &[DealParticipation],
    parties: &BTreeMap<Uuid, domain::entities::Party>,
    terms: &[Term],
    value_distribution: &ValueDistribution,
) -> String {
    let mut text = String::new();
    text.push_str(&format!("# Agreement for {}\n\n", deal.deal_title.as_str()));
    text.push_str(&format!(
        "**Deal Reference:** {}  \n**Status:** {}  \n**Version:** Generated from locked terms.\n\n",
        deal.deal_reference,
        deal.deal_status.as_str()
    ));

    text.push_str("## Parties\n\n");
    for p in participations {
        let display = parties
            .get(&p.party_id)
            .map(|party| party.display_name.as_str())
            .unwrap_or("Unknown party");
        text.push_str(&format!(
            "- **{}** ({}) – {}\n",
            display,
            p.role.as_str(),
            p.party_id
        ));
    }
    text.push('\n');

    text.push_str("## Terms\n\n");
    let accepted_terms: Vec<&Term> = terms
        .iter()
        .filter(|t| t.negotiation_status == TermStatus::Accepted)
        .collect();
    if accepted_terms.is_empty() {
        text.push_str("_No accepted terms._\n");
    } else {
        for t in accepted_terms {
            text.push_str(&format!(
                "- **{}** ({}): {}\n",
                t.term_name,
                t.term_type.as_str(),
                t.description
            ));
        }
    }
    text.push('\n');

    text.push_str("## Value Distribution\n\n");
    text.push_str(&format!(
        "- Total value: {} {}\n",
        value_distribution.total_value, value_distribution.currency
    ));
    text.push_str(&format!(
        "- Supplier share: {}% ({} {})\n",
        value_distribution.supplier_share_percentage,
        value_distribution.supplier_share_amount,
        value_distribution.currency
    ));
    text.push_str(&format!(
        "- Enhancer share: {}% ({} {})\n",
        value_distribution.enhancer_share_percentage,
        value_distribution.enhancer_share_amount,
        value_distribution.currency
    ));
    text.push_str(&format!(
        "- Consumer cost: {}% ({} {})\n",
        value_distribution.consumer_cost_percentage,
        value_distribution.consumer_cost_amount,
        value_distribution.currency
    ));
    text.push_str(&format!(
        "- Platform fee: {}% ({} {})\n",
        value_distribution.platform_fee_percentage,
        value_distribution.platform_fee_amount,
        value_distribution.currency
    ));
    text.push('\n');

    text.push_str("## Payment Schedule\n\n");
    if value_distribution.payment_schedule.is_empty() {
        text.push_str("_No scheduled payments._\n");
    } else {
        for entry in &value_distribution.payment_schedule {
            text.push_str(&format!(
                "- {}. {} – {} {} to {}\n",
                entry.sequence,
                entry.trigger.as_str(),
                entry.amount,
                value_distribution.currency,
                entry.recipient_role.as_str()
            ));
        }
    }
    text.push('\n');

    text.push_str("## Roles and Responsibilities\n\n");
    for p in participations {
        let display = parties
            .get(&p.party_id)
            .map(|party| party.display_name.as_str())
            .unwrap_or("Unknown party");
        text.push_str(&format!(
            "- {} ({}) agrees to perform the obligations described for the {} role.\n",
            display,
            p.party_id,
            p.role.as_str()
        ));
    }
    text.push('\n');

    text.push_str("## Platform Terms\n\n");
    text.push_str("This agreement is governed by the Hayaland platform terms of service and the dispute resolution mechanism selected by the platform administrator.\n");

    if let Some(timeline) = &deal.timeline {
        text.push_str("\n## Timeline\n\n");
        text.push_str(&format!("{:#}\n", timeline));
    }

    text
}
