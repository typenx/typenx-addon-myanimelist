use std::{net::SocketAddr, sync::Arc};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use tower_http::trace::TraceLayer;

use crate::{
    api::MyAnimeListCatalog,
    types::{
        AddonHealth, AddonManifest, CatalogDefinition, CatalogRequest, ContentType,
        RecommendationRequest, SearchRequest,
    },
};

#[derive(Clone)]
pub struct AppState {
    catalog: Arc<MyAnimeListCatalog>,
}

pub fn manifest() -> AddonManifest {
    AddonManifest {
        id: "typenx-addon-myanimelist".to_string(),
        name: "MyAnimeList".to_string(),
        version: "0.1.0".to_string(),
        description: Some("Official Typenx metadata addon backed by MyAnimeList.".to_string()),
        icon: Some("https://cdn.myanimelist.net/img/sp/icon/apple-touch-icon-256.png".to_string()),
        resources: vec![
            "catalog".to_string(),
            "search".to_string(),
            "anime_meta".to_string(),
            "recommendations".to_string(),
        ],
        catalogs: vec![
            CatalogDefinition {
                id: "popular".to_string(),
                name: "Popular Anime".to_string(),
                content_type: ContentType::Anime,
                filters: vec![],
            },
            CatalogDefinition {
                id: "airing".to_string(),
                name: "Airing Anime".to_string(),
                content_type: ContentType::Anime,
                filters: vec![],
            },
            CatalogDefinition {
                id: "manga-popular".to_string(),
                name: "Popular Manga".to_string(),
                content_type: ContentType::Manga,
                filters: vec![],
            },
            CatalogDefinition {
                id: "manga-favorite".to_string(),
                name: "Favorite Manga".to_string(),
                content_type: ContentType::Manga,
                filters: vec![],
            },
        ],
    }
}

pub fn app(catalog_client: MyAnimeListCatalog) -> Router {
    let state = AppState {
        catalog: Arc::new(catalog_client),
    };

    Router::new()
        .route("/health", get(health))
        .route("/manifest", get(|| async { Json(manifest()) }))
        .route("/catalog", post(catalog_handler))
        .route("/search", post(search_handler))
        .route("/anime/:id", get(anime_handler))
        .route("/manga/:id", get(manga_handler))
        .route("/recommendations", post(recommendations_handler))
        .fallback(not_found)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

pub async fn serve(addr: SocketAddr, catalog: MyAnimeListCatalog) -> anyhow::Result<()> {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("Typenx addon listening on http://{addr}");
    axum::serve(listener, app(catalog)).await?;
    Ok(())
}

async fn health(State(state): State<AppState>) -> Json<AddonHealth> {
    let ok = state.catalog.is_configured();
    Json(AddonHealth {
        ok,
        message: (!ok).then(|| "MAL_CLIENT_ID is missing".to_string()),
    })
}

async fn catalog_handler(
    State(state): State<AppState>,
    Json(request): Json<CatalogRequest>,
) -> Result<Json<impl Serialize>, ApiError> {
    Ok(Json(state.catalog.catalog(request).await?))
}

async fn search_handler(
    State(state): State<AppState>,
    Json(request): Json<SearchRequest>,
) -> Result<Json<impl Serialize>, ApiError> {
    Ok(Json(state.catalog.search(request).await?))
}

async fn anime_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<impl Serialize>, ApiError> {
    Ok(Json(state.catalog.anime(&id).await?))
}

async fn manga_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<impl Serialize>, ApiError> {
    Ok(Json(state.catalog.manga(&id).await?))
}

async fn recommendations_handler(
    State(state): State<AppState>,
    Json(request): Json<RecommendationRequest>,
) -> Result<Json<impl Serialize>, ApiError> {
    Ok(Json(state.catalog.recommendations(request).await?))
}

async fn not_found() -> (StatusCode, Json<ErrorBody>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorBody {
            message: "Not found".to_string(),
        }),
    )
}

#[derive(Debug)]
struct ApiError(anyhow::Error);

impl From<anyhow::Error> for ApiError {
    fn from(value: anyhow::Error) -> Self {
        Self(value)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorBody {
                message: self.0.to_string(),
            }),
        )
            .into_response()
    }
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    message: String,
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    use super::*;

    #[tokio::test]
    async fn manifest_route_matches_addon_contract() {
        let response = app(MyAnimeListCatalog::new(None))
            .oneshot(
                Request::builder()
                    .uri("/manifest")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let manifest: AddonManifest = serde_json::from_slice(&body).unwrap();
        assert_eq!(manifest.id, "typenx-addon-myanimelist");
        assert_eq!(
            manifest.resources,
            vec!["catalog", "search", "anime_meta", "recommendations"]
        );
        assert_eq!(manifest.catalogs.len(), 4);
    }

    #[tokio::test]
    async fn health_reports_missing_client_id() {
        let response = app(MyAnimeListCatalog::new(None))
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let health: AddonHealth = serde_json::from_slice(&body).unwrap();

        assert!(!health.ok);
        assert_eq!(health.message.as_deref(), Some("MAL_CLIENT_ID is missing"));
    }
}
