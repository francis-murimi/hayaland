use crate::errors::ApplicationError;
use crate::users::dto::{ListUsersQuery, ListUsersResult, UserDto};
use domain::repositories::UserRepository;
use std::sync::Arc;

const DEFAULT_PAGE: i64 = 1;
const DEFAULT_PER_PAGE: i64 = 20;
const MAX_PER_PAGE: i64 = 100;

pub struct ListUsers {
    repo: Arc<dyn UserRepository>,
}

impl ListUsers {
    pub fn new(repo: Arc<dyn UserRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        query: ListUsersQuery,
    ) -> Result<ListUsersResult, ApplicationError> {
        let page = query.page.unwrap_or(DEFAULT_PAGE).max(1);
        let per_page = query
            .per_page
            .unwrap_or(DEFAULT_PER_PAGE)
            .clamp(1, MAX_PER_PAGE);
        let offset = (page - 1) * per_page;

        let users = self
            .repo
            .list(per_page, offset, query.active_only)
            .await?
            .into_iter()
            .map(UserDto::from)
            .collect::<Vec<_>>();

        let total = users.len();

        Ok(ListUsersResult {
            users,
            total,
            page,
            per_page,
        })
    }
}
