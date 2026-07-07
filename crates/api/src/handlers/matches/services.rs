use application::matching::{AdminMatchControls, GenerateMatches, ListMatches, RespondToMatch};
use domain::repositories::{CatalogRepository, MatchRepository, PartyRepository};
use infrastructure::repositories::{
    PostgresCatalogRepository, PostgresMatchRepository, PostgresPartyRepository,
};
use sqlx::PgPool;
use std::sync::Arc;

pub fn generate_matches(pool: PgPool) -> GenerateMatches {
    let match_repo: Arc<dyn MatchRepository> = Arc::new(PostgresMatchRepository::new(pool.clone()));
    let party_repo: Arc<dyn PartyRepository> = Arc::new(PostgresPartyRepository::new(pool.clone()));
    let catalog_repo: Arc<dyn CatalogRepository> = Arc::new(PostgresCatalogRepository::new(pool));
    GenerateMatches::new(match_repo, party_repo, catalog_repo)
}

pub fn list_matches(pool: PgPool) -> ListMatches {
    let match_repo: Arc<dyn MatchRepository> = Arc::new(PostgresMatchRepository::new(pool.clone()));
    let party_repo: Arc<dyn PartyRepository> = Arc::new(PostgresPartyRepository::new(pool));
    ListMatches::new(match_repo, party_repo)
}

pub fn respond_to_match(pool: PgPool) -> RespondToMatch {
    let match_repo: Arc<dyn MatchRepository> = Arc::new(PostgresMatchRepository::new(pool.clone()));
    let party_repo: Arc<dyn PartyRepository> = Arc::new(PostgresPartyRepository::new(pool));
    RespondToMatch::new(match_repo, party_repo)
}

pub fn admin_match_controls(pool: PgPool) -> AdminMatchControls {
    let match_repo: Arc<dyn MatchRepository> = Arc::new(PostgresMatchRepository::new(pool));
    AdminMatchControls::new(match_repo)
}
