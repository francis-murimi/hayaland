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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{test_repo_with, test_user, FakeRepo};
    use std::sync::Arc;

    #[tokio::test]
    async fn lists_users_paginated() {
        let repo = Arc::new(FakeRepo {
            users: Default::default(),
        });
        repo.create(&test_user("alice@example.com", "alice", "password123"))
            .await
            .unwrap();
        repo.create(&test_user("bob@example.com", "bob", "password123"))
            .await
            .unwrap();

        let result = ListUsers::new(repo)
            .execute(ListUsersQuery {
                page: Some(1),
                per_page: Some(1),
                active_only: None,
            })
            .await
            .unwrap();

        assert_eq!(result.users.len(), 1);
        assert_eq!(result.page, 1);
        assert_eq!(result.per_page, 1);
    }

    #[tokio::test]
    async fn filters_active_only() {
        let mut inactive = test_user("inactive@example.com", "inactive", "password123");
        inactive.is_active = false;
        let repo = test_repo_with(inactive);
        repo.create(&test_user("active@example.com", "active", "password123"))
            .await
            .unwrap();

        let result = ListUsers::new(repo)
            .execute(ListUsersQuery {
                page: None,
                per_page: None,
                active_only: Some(true),
            })
            .await
            .unwrap();

        assert_eq!(result.users.len(), 1);
        assert!(result.users[0].is_active);
    }

    #[tokio::test]
    async fn clamps_out_of_range_pagination() {
        let repo = test_repo_with(test_user("only@example.com", "only", "password123"));

        let result = ListUsers::new(repo)
            .execute(ListUsersQuery {
                page: Some(0),
                per_page: Some(500),
                active_only: None,
            })
            .await
            .unwrap();

        assert_eq!(result.page, 1);
        assert_eq!(result.per_page, 100);
    }
}
