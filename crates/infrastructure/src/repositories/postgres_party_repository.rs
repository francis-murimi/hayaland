use async_trait::async_trait;
use domain::entities::{
    DealRole, DisplayName, Email, GeoPoint, Party, PartyMembershipRole, PartyType, RoleProfile,
    UserPartyMembership, VerificationStatus,
};
use domain::errors::DomainError;
use domain::repositories::{PartyRepository, PartySearchCriteria};
use sqlx::{Error as SqlxError, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

pub struct PostgresPartyRepository {
    pool: PgPool,
}

impl PostgresPartyRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PartyRepository for PostgresPartyRepository {
    async fn create(&self, party: &Party) -> Result<(), DomainError> {
        let (lat, lng) = party
            .location
            .map(|l| (Some(l.latitude), Some(l.longitude)))
            .unwrap_or((None, None));

        sqlx::query!(
            r#"
            INSERT INTO parties (
                id, party_type, display_name, email, phone, tax_id, verification_status,
                primary_domain_id, latitude, longitude, location_geo, service_radius_km, trust_score,
                total_deals_completed, total_deals_initiated, is_active, created_at, updated_at
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                CASE
                    WHEN $9::float8 IS NOT NULL AND $10::float8 IS NOT NULL
                    THEN ST_SetSRID(ST_MakePoint($10, $9), 4326)::geography
                    ELSE NULL
                END,
                $11, $12, $13, $14, $15, $16, $17
            )
            "#,
            party.id,
            party.party_type.as_str(),
            party.display_name.as_str(),
            party.email.as_str(),
            party.phone.as_ref().map(|p| p.as_str()),
            party.tax_id.as_deref(),
            party.verification_status.as_str(),
            party.primary_domain_id,
            lat,
            lng,
            party.service_radius_km,
            party.trust_score,
            party.total_deals_completed,
            party.total_deals_initiated,
            party.is_active,
            party.created_at,
            party.updated_at
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Party>, DomainError> {
        let row = sqlx::query_as!(
            PartyRow,
            r#"
            SELECT id, party_type, display_name, email, phone, tax_id, verification_status,
                primary_domain_id, latitude, longitude, service_radius_km, trust_score,
                total_deals_completed, total_deals_initiated, is_active, created_at, updated_at
            FROM parties
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.map(build_party_from_row))
    }

    async fn find_by_email(&self, email: &Email) -> Result<Option<Party>, DomainError> {
        let row = sqlx::query_as!(
            PartyRow,
            r#"
            SELECT id, party_type, display_name, email, phone, tax_id, verification_status,
                primary_domain_id, latitude, longitude, service_radius_km, trust_score,
                total_deals_completed, total_deals_initiated, is_active, created_at, updated_at
            FROM parties
            WHERE email = $1
            "#,
            email.as_str()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.map(build_party_from_row))
    }

    async fn update(&self, party: &Party) -> Result<(), DomainError> {
        let (lat, lng) = party
            .location
            .map(|l| (Some(l.latitude), Some(l.longitude)))
            .unwrap_or((None, None));

        sqlx::query!(
            r#"
            UPDATE parties
            SET party_type = $1,
                display_name = $2,
                email = $3,
                phone = $4,
                tax_id = $5,
                verification_status = $6,
                primary_domain_id = $7,
                latitude = $8,
                longitude = $9,
                location_geo = CASE
                    WHEN $8::float8 IS NOT NULL AND $9::float8 IS NOT NULL
                    THEN ST_SetSRID(ST_MakePoint($9, $8), 4326)::geography
                    ELSE NULL
                END,
                service_radius_km = $10,
                trust_score = $11,
                total_deals_completed = $12,
                total_deals_initiated = $13,
                is_active = $14,
                created_at = $15,
                updated_at = $16
            WHERE id = $17
            "#,
            party.party_type.as_str(),
            party.display_name.as_str(),
            party.email.as_str(),
            party.phone.as_ref().map(|p| p.as_str()),
            party.tax_id.as_deref(),
            party.verification_status.as_str(),
            party.primary_domain_id,
            lat,
            lng,
            party.service_radius_km,
            party.trust_score,
            party.total_deals_completed,
            party.total_deals_initiated,
            party.is_active,
            party.created_at,
            party.updated_at,
            party.id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn soft_delete(&self, id: Uuid) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            UPDATE parties
            SET is_active = false, updated_at = now()
            WHERE id = $1
            "#,
            id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn list(&self, criteria: &PartySearchCriteria) -> Result<Vec<Party>, DomainError> {
        let query_pattern = criteria.query.as_ref().map(|q| format!("%{q}%"));
        let party_types: Option<Vec<String>> = if criteria.party_types.is_empty() {
            None
        } else {
            Some(
                criteria
                    .party_types
                    .iter()
                    .map(|t| t.as_str().to_string())
                    .collect(),
            )
        };
        let verification_statuses: Option<Vec<String>> =
            if criteria.verification_statuses.is_empty() {
                None
            } else {
                Some(
                    criteria
                        .verification_statuses
                        .iter()
                        .map(|s| s.as_str().to_string())
                        .collect(),
                )
            };
        let roles: Option<Vec<String>> = if criteria.roles.is_empty() {
            None
        } else {
            Some(
                criteria
                    .roles
                    .iter()
                    .map(|r| r.as_str().to_string())
                    .collect(),
            )
        };

        let rows = sqlx::query_as!(
            PartyRow,
            r#"
            SELECT id, party_type, display_name, email, phone, tax_id, verification_status,
                primary_domain_id, latitude, longitude, service_radius_km, trust_score,
                total_deals_completed, total_deals_initiated, is_active, created_at, updated_at
            FROM parties
            WHERE ($1::text IS NULL OR display_name ILIKE $1 OR email ILIKE $1)
              AND ($2::text[] IS NULL OR party_type = ANY($2))
              AND ($3::text[] IS NULL OR verification_status = ANY($3))
              AND ($4::bool IS NULL OR is_active = $4)
              AND ($5::float8 IS NULL OR trust_score >= $5)
              AND ($6::float8 IS NULL OR trust_score <= $6)
              AND ($7::uuid IS NULL OR primary_domain_id = $7)
              AND ($8::text[] IS NULL OR id IN (
                  SELECT party_id FROM party_roles WHERE role_type = ANY($8) AND is_active = true
              ))
              AND ($11::float8 IS NULL OR $12::float8 IS NULL OR $13::float8 IS NULL
                  OR ST_DWithin(
                      location_geo,
                      ST_SetSRID(ST_MakePoint($12, $11), 4326)::geography,
                      $13 * 1000
                  ))
            ORDER BY
                CASE
                    WHEN $11::float8 IS NOT NULL AND $12::float8 IS NOT NULL AND $13::float8 IS NOT NULL
                    THEN ST_Distance(
                        location_geo,
                        ST_SetSRID(ST_MakePoint($12, $11), 4326)::geography
                    )
                END ASC NULLS LAST,
                created_at DESC
            LIMIT $9 OFFSET $10
            "#,
            query_pattern.as_deref(),
            party_types.as_deref(),
            verification_statuses.as_deref(),
            criteria.active_only,
            criteria.min_trust_score,
            criteria.max_trust_score,
            criteria.primary_domain_id,
            roles.as_deref(),
            criteria.limit,
            criteria.offset,
            criteria.latitude,
            criteria.longitude,
            criteria.radius_km
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(rows.into_iter().map(build_party_from_row).collect())
    }

    async fn count(&self, criteria: &PartySearchCriteria) -> Result<i64, DomainError> {
        let query_pattern = criteria.query.as_ref().map(|q| format!("%{q}%"));
        let party_types: Option<Vec<String>> = if criteria.party_types.is_empty() {
            None
        } else {
            Some(
                criteria
                    .party_types
                    .iter()
                    .map(|t| t.as_str().to_string())
                    .collect(),
            )
        };
        let verification_statuses: Option<Vec<String>> =
            if criteria.verification_statuses.is_empty() {
                None
            } else {
                Some(
                    criteria
                        .verification_statuses
                        .iter()
                        .map(|s| s.as_str().to_string())
                        .collect(),
                )
            };
        let roles: Option<Vec<String>> = if criteria.roles.is_empty() {
            None
        } else {
            Some(
                criteria
                    .roles
                    .iter()
                    .map(|r| r.as_str().to_string())
                    .collect(),
            )
        };

        let row = sqlx::query!(
            r#"
            SELECT COUNT(*) as count
            FROM parties
            WHERE ($1::text IS NULL OR display_name ILIKE $1 OR email ILIKE $1)
              AND ($2::text[] IS NULL OR party_type = ANY($2))
              AND ($3::text[] IS NULL OR verification_status = ANY($3))
              AND ($4::bool IS NULL OR is_active = $4)
              AND ($5::float8 IS NULL OR trust_score >= $5)
              AND ($6::float8 IS NULL OR trust_score <= $6)
              AND ($7::uuid IS NULL OR primary_domain_id = $7)
              AND ($8::text[] IS NULL OR id IN (
                  SELECT party_id FROM party_roles WHERE role_type = ANY($8) AND is_active = true
              ))
              AND ($9::float8 IS NULL OR $10::float8 IS NULL OR $11::float8 IS NULL
                  OR ST_DWithin(
                      location_geo,
                      ST_SetSRID(ST_MakePoint($10, $9), 4326)::geography,
                      $11 * 1000
                  ))
            "#,
            query_pattern.as_deref(),
            party_types.as_deref(),
            verification_statuses.as_deref(),
            criteria.active_only,
            criteria.min_trust_score,
            criteria.max_trust_score,
            criteria.primary_domain_id,
            roles.as_deref(),
            criteria.latitude,
            criteria.longitude,
            criteria.radius_km
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.count.unwrap_or(0))
    }

    async fn add_role(
        &self,
        party_id: Uuid,
        role: DealRole,
        profile: RoleProfile,
    ) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO party_roles (id, party_id, role_type, profile, is_active, assigned_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (party_id, role_type) DO UPDATE SET
                profile = EXCLUDED.profile,
                is_active = true,
                assigned_at = EXCLUDED.assigned_at
            "#,
            Uuid::now_v7(),
            party_id,
            role.as_str(),
            serde_json::to_value(&profile)
                .map_err(|e| DomainError::RepositoryError(e.to_string()))?,
            true,
            OffsetDateTime::now_utc()
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn remove_role(&self, party_id: Uuid, role: DealRole) -> Result<(), DomainError> {
        let active_deals = self.count_active_deals_for_role(party_id, role).await?;
        if active_deals > 0 {
            return Err(DomainError::PartyRoleHasActiveDeals);
        }

        sqlx::query!(
            r#"
            UPDATE party_roles
            SET is_active = false
            WHERE party_id = $1 AND role_type = $2
            "#,
            party_id,
            role.as_str()
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn list_roles(
        &self,
        party_id: Uuid,
    ) -> Result<Vec<(DealRole, RoleProfile)>, DomainError> {
        let rows = sqlx::query!(
            r#"
            SELECT role_type, profile
            FROM party_roles
            WHERE party_id = $1 AND is_active = true
            ORDER BY assigned_at
            "#,
            party_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        rows.into_iter()
            .map(|r| {
                let role = DealRole::try_from(r.role_type.as_str())?;
                let profile: RoleProfile = serde_json::from_value(r.profile)
                    .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
                Ok((role, profile))
            })
            .collect()
    }

    async fn has_role(&self, party_id: Uuid, role: DealRole) -> Result<bool, DomainError> {
        let row = sqlx::query!(
            r#"
            SELECT EXISTS (
                SELECT 1 FROM party_roles
                WHERE party_id = $1 AND role_type = $2 AND is_active = true
            ) as exists
            "#,
            party_id,
            role.as_str()
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.exists.unwrap_or(false))
    }

    async fn count_active_deals_for_role(
        &self,
        _party_id: Uuid,
        _role: DealRole,
    ) -> Result<i64, DomainError> {
        // Placeholder until deal tables exist.
        Ok(0)
    }

    async fn count_active_deals(&self, _party_id: Uuid) -> Result<i64, DomainError> {
        // Placeholder until deal tables exist.
        Ok(0)
    }

    async fn add_membership(&self, membership: &UserPartyMembership) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO user_party_memberships (id, user_id, party_id, member_role, is_active, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (user_id, party_id) DO UPDATE SET
                member_role = EXCLUDED.member_role,
                is_active = true
            "#,
            membership.id,
            membership.user_id,
            membership.party_id,
            membership.member_role.as_str(),
            membership.is_active,
            membership.created_at
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn list_memberships_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<(UserPartyMembership, Party)>, DomainError> {
        let rows = sqlx::query!(
            r#"
            SELECT
                m.id as membership_id, m.user_id, m.party_id, m.member_role, m.is_active as membership_active, m.created_at as membership_created_at,
                p.id as "party_id_2!", p.party_type, p.display_name, p.email, p.phone, p.tax_id, p.verification_status,
                p.primary_domain_id, p.latitude, p.longitude, p.service_radius_km, p.trust_score,
                p.total_deals_completed, p.total_deals_initiated, p.is_active as party_active, p.created_at as party_created_at, p.updated_at as party_updated_at
            FROM user_party_memberships m
            JOIN parties p ON p.id = m.party_id
            WHERE m.user_id = $1 AND m.is_active = true
            ORDER BY p.created_at DESC
            "#,
            user_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        rows.into_iter()
            .map(|r| {
                let membership = UserPartyMembership {
                    id: r.membership_id,
                    user_id: r.user_id,
                    party_id: r.party_id,
                    member_role: PartyMembershipRole::try_from(r.member_role.as_str())?,
                    is_active: r.membership_active,
                    created_at: r.membership_created_at,
                };
                let party = build_party_from_row(PartyRow {
                    id: r.party_id_2,
                    party_type: r.party_type,
                    display_name: r.display_name,
                    email: r.email,
                    phone: r.phone,
                    tax_id: r.tax_id,
                    verification_status: r.verification_status,
                    primary_domain_id: r.primary_domain_id,
                    latitude: r.latitude,
                    longitude: r.longitude,
                    service_radius_km: r.service_radius_km,
                    trust_score: r.trust_score,
                    total_deals_completed: r.total_deals_completed,
                    total_deals_initiated: r.total_deals_initiated,
                    is_active: r.party_active,
                    created_at: r.party_created_at,
                    updated_at: r.party_updated_at,
                });
                Ok((membership, party))
            })
            .collect()
    }

    async fn find_membership(
        &self,
        user_id: Uuid,
        party_id: Uuid,
    ) -> Result<Option<UserPartyMembership>, DomainError> {
        let row = sqlx::query!(
            r#"
            SELECT id, user_id, party_id, member_role, is_active, created_at
            FROM user_party_memberships
            WHERE user_id = $1 AND party_id = $2
            "#,
            user_id,
            party_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.map(|r| UserPartyMembership {
            id: r.id,
            user_id: r.user_id,
            party_id: r.party_id,
            member_role: PartyMembershipRole::try_from(r.member_role.as_str())
                .unwrap_or(PartyMembershipRole::Member),
            is_active: r.is_active,
            created_at: r.created_at,
        }))
    }

    async fn touch(&self, id: Uuid, updated_at: OffsetDateTime) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            UPDATE parties
            SET updated_at = $1
            WHERE id = $2
            "#,
            updated_at,
            id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct PartyRow {
    id: Uuid,
    party_type: String,
    display_name: String,
    email: String,
    phone: Option<String>,
    tax_id: Option<String>,
    verification_status: String,
    primary_domain_id: Option<Uuid>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    service_radius_km: Option<f64>,
    trust_score: f64,
    total_deals_completed: i32,
    total_deals_initiated: i32,
    is_active: bool,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

fn build_party_from_row(row: PartyRow) -> Party {
    let location = match (row.latitude, row.longitude) {
        (Some(lat), Some(lng)) => GeoPoint::new(lat, lng).ok(),
        _ => None,
    };

    let mut party = Party::new(
        row.id,
        PartyType::try_from(row.party_type.as_str()).expect("stored party type is valid"),
        DisplayName::new(&row.display_name).expect("stored display name is valid"),
        Email::new(&row.email).expect("stored email is valid"),
    );

    party.phone = row
        .phone
        .map(|p| domain::entities::Phone::new(&p).expect("stored phone is valid"));
    party.tax_id = row.tax_id;
    party.verification_status = VerificationStatus::try_from(row.verification_status.as_str())
        .expect("stored verification status is valid");
    party.primary_domain_id = row.primary_domain_id;
    party.location = location;
    party.service_radius_km = row.service_radius_km;
    party.trust_score = row.trust_score;
    party.total_deals_completed = row.total_deals_completed;
    party.total_deals_initiated = row.total_deals_initiated;
    party.is_active = row.is_active;
    party.created_at = row.created_at;
    party.updated_at = row.updated_at;

    party
}

fn map_err(err: SqlxError) -> DomainError {
    if let SqlxError::Database(db_err) = &err {
        match db_err.constraint() {
            Some("parties_email_key") => return DomainError::DuplicatePartyEmail,
            Some("party_roles_party_id_role_type_key") => return DomainError::DuplicatePartyRole,
            _ => {}
        }
    }
    DomainError::RepositoryError(err.to_string())
}
