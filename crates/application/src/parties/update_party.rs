use crate::errors::ApplicationError;
use crate::parties::create_party::map_party_to_result;
use crate::parties::dto::{PartyResult, UpdatePartyCommand};
use domain::entities::{Email, GeoPoint, Phone};
use domain::repositories::PartyRepository;
use std::sync::Arc;
use time::OffsetDateTime;
use tracing::instrument;
use uuid::Uuid;

/// Update an existing party.
#[derive(Clone)]
pub struct UpdateParty {
    repo: Arc<dyn PartyRepository>,
}

impl UpdateParty {
    pub fn new(repo: Arc<dyn PartyRepository>) -> Self {
        Self { repo }
    }

    #[instrument(skip(self, cmd), fields(party_id = %party_id))]
    pub async fn execute(
        &self,
        party_id: Uuid,
        cmd: UpdatePartyCommand,
    ) -> Result<PartyResult, ApplicationError> {
        if !cmd.is_admin {
            let membership = self
                .repo
                .find_membership(cmd.actor_user_id, party_id)
                .await?;
            match membership {
                Some(m) if m.is_active && can_modify(m.member_role) => {}
                _ => return Err(ApplicationError::Forbidden),
            }
        }

        let mut party = self
            .repo
            .find_by_id(party_id)
            .await?
            .ok_or(ApplicationError::PartyNotFound)?;

        if let Some(display_name) = cmd.display_name {
            party.display_name = domain::entities::DisplayName::new(&display_name)
                .map_err(ApplicationError::from)?;
        }

        if let Some(email) = cmd.email {
            let new_email = Email::new(&email).map_err(ApplicationError::from)?;
            if new_email.as_str() != party.email.as_str()
                && self.repo.find_by_email(&new_email).await?.is_some()
            {
                return Err(ApplicationError::DuplicatePartyEmail);
            }
            party.email = new_email;
        }

        if let Some(phone) = cmd.phone {
            party.phone = Some(Phone::new(&phone).map_err(ApplicationError::from)?);
        }

        if cmd.tax_id.is_some() {
            party.tax_id = cmd.tax_id;
        }

        if cmd.primary_domain_id.is_some() {
            party.primary_domain_id = cmd.primary_domain_id;
        }

        if let (Some(lat), Some(lng)) = (cmd.latitude, cmd.longitude) {
            party.location = Some(GeoPoint::new(lat, lng).map_err(ApplicationError::from)?);
        } else if cmd.latitude.is_none() && cmd.longitude.is_none() {
            // no-op; preserve existing location
        } else {
            return Err(ApplicationError::Validation(vec![
                "latitude and longitude must be provided together".to_string(),
            ]));
        }

        if cmd.service_radius_km.is_some() {
            party.service_radius_km = cmd.service_radius_km;
        }

        if cmd.is_admin {
            if let Some(status) = cmd.verification_status {
                party.verification_status = status;
            }
            if let Some(is_active) = cmd.is_active {
                party.is_active = is_active;
            }
        }

        party.updated_at = OffsetDateTime::now_utc();
        self.repo.update(&party).await?;

        Ok(map_party_to_result(party))
    }
}

fn can_modify(role: domain::entities::PartyMembershipRole) -> bool {
    matches!(
        role,
        domain::entities::PartyMembershipRole::Owner | domain::entities::PartyMembershipRole::Admin
    )
}
