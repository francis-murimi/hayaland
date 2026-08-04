use crate::errors::ApiError;
use crate::AppState;
use actix_web::{web, HttpResponse};
use application::search::dto::SearchQuery;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SearchQueryParams {
    pub q: String,
    pub r#type: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl From<SearchQueryParams> for SearchQuery {
    fn from(params: SearchQueryParams) -> Self {
        Self {
            q: params.q,
            r#type: params.r#type,
            limit: params.limit,
            offset: params.offset,
        }
    }
}

pub async fn search(
    state: web::Data<AppState>,
    query: web::Query<SearchQueryParams>,
) -> Result<HttpResponse, ApiError> {
    let result = state.search.execute(query.into_inner().into()).await?;
    Ok(HttpResponse::Ok().json(result))
}
