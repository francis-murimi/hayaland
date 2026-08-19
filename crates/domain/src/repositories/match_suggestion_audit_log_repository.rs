use crate::entities::MatchSuggestionAuditLogEntry;
use crate::errors::DomainError;
use async_trait::async_trait;
use uuid::Uuid;

/// Outbound port for persisting and retrieving match-suggestion audit log entries.
#[async_trait]
pub trait MatchSuggestionAuditLogRepository: Send + Sync {
    /// Persist a new audit-log entry.
    async fn create(&self, entry: &MatchSuggestionAuditLogEntry) -> Result<(), DomainError>;

    /// List entries for a specific match suggestion, newest first.
    async fn list_by_match_suggestion(
        &self,
        match_suggestion_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<MatchSuggestionAuditLogEntry>, DomainError>;

    /// List entries authored by a specific admin user, newest first.
    async fn list_by_admin(
        &self,
        admin_user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<MatchSuggestionAuditLogEntry>, DomainError>;

    /// List entries that reference a specific party, newest first.
    async fn list_by_party(
        &self,
        party_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<MatchSuggestionAuditLogEntry>, DomainError>;
}
