use anyhow::{anyhow, Context};
use serde::Deserialize;

use crate::{
    mapping::{
        to_manga_metadata, to_manga_preview, to_metadata, to_preview, MalAnime, MalManga,
        ANIME_FIELDS, MANGA_FIELDS,
    },
    recommendations::{recommend_anime, RecommendationOptions},
    types::{
        AnimeMetadata, AnimePreview, CatalogRequest, CatalogResponse, ContentType,
        RecommendationRequest, RecommendationResponse, SearchRequest,
    },
};

const API_BASE: &str = "https://api.myanimelist.net/v2";

#[derive(Clone)]
pub struct MyAnimeListCatalog {
    client_id: Option<String>,
    http: reqwest::Client,
}

impl MyAnimeListCatalog {
    pub fn new(client_id: Option<String>) -> Self {
        Self {
            client_id: client_id.filter(|value| !value.trim().is_empty()),
            http: reqwest::Client::new(),
        }
    }

    pub fn is_configured(&self) -> bool {
        self.client_id.is_some()
    }

    pub async fn catalog(&self, request: CatalogRequest) -> anyhow::Result<CatalogResponse> {
        let limit = clamp_limit(request.limit);
        if let Some(query) = request.query.filter(|query| !query.trim().is_empty()) {
            return self.search_query(&query, request.content_type, limit).await;
        }

        self.catalog_feed(&request.catalog_id, request.content_type, limit)
            .await
    }

    pub async fn search(&self, request: SearchRequest) -> anyhow::Result<CatalogResponse> {
        let limit = clamp_limit(request.limit);
        let query = request.query.trim();
        if query.is_empty() {
            let catalog_id = if request.content_type == Some(ContentType::Manga) {
                "manga-popular"
            } else {
                "popular"
            };
            return self
                .catalog_feed(catalog_id, request.content_type, limit)
                .await;
        }

        self.search_query(query, request.content_type, limit).await
    }

    async fn search_query(
        &self,
        query: &str,
        content_type: Option<ContentType>,
        limit: u32,
    ) -> anyhow::Result<CatalogResponse> {
        if content_type == Some(ContentType::Manga) {
            let response = self
                .get::<MalMangaListResponse>(&format!(
                    "/manga?q={}&limit={limit}&fields={MANGA_FIELDS}",
                    urlencoding::encode(query)
                ))
                .await?;
            return Ok(CatalogResponse {
                items: response
                    .data
                    .iter()
                    .map(|item| to_manga_preview(&item.node))
                    .collect(),
            });
        }

        let response = self
            .get::<MalAnimeListResponse>(&format!(
                "/anime?q={}&limit={limit}&fields={ANIME_FIELDS}",
                urlencoding::encode(query)
            ))
            .await?;
        Ok(CatalogResponse {
            items: response
                .data
                .iter()
                .map(|item| to_preview(&item.node))
                .collect(),
        })
    }

    async fn catalog_feed(
        &self,
        catalog_id: &str,
        content_type: Option<ContentType>,
        limit: u32,
    ) -> anyhow::Result<CatalogResponse> {
        if is_manga_request(content_type.as_ref(), catalog_id) {
            let ranking_type = if catalog_id == "manga-favorite" {
                "bypopularity"
            } else {
                "all"
            };
            let response = self
                .get::<MalMangaListResponse>(&format!(
                    "/manga/ranking?ranking_type={ranking_type}&limit={limit}&fields={MANGA_FIELDS}"
                ))
                .await?;
            return Ok(CatalogResponse {
                items: response
                    .data
                    .iter()
                    .map(|item| to_manga_preview(&item.node))
                    .collect(),
            });
        }

        let ranking_type = if catalog_id == "airing" {
            "airing"
        } else {
            "all"
        };
        let response = self
            .get::<MalAnimeListResponse>(&format!(
                "/anime/ranking?ranking_type={ranking_type}&limit={limit}&fields={ANIME_FIELDS}"
            ))
            .await?;
        Ok(CatalogResponse {
            items: response
                .data
                .iter()
                .map(|item| to_preview(&item.node))
                .collect(),
        })
    }

    pub async fn anime(&self, id: &str) -> anyhow::Result<AnimeMetadata> {
        let anime = self
            .get::<MalAnime>(&format!(
                "/anime/{}?fields={ANIME_FIELDS}",
                urlencoding::encode(id)
            ))
            .await?;
        Ok(to_metadata(&anime))
    }

    pub async fn manga(&self, id: &str) -> anyhow::Result<AnimeMetadata> {
        let manga = self
            .get::<MalManga>(&format!(
                "/manga/{}?fields={MANGA_FIELDS}",
                urlencoding::encode(id)
            ))
            .await?;
        Ok(to_manga_metadata(&manga))
    }

    pub async fn recommendations(
        &self,
        request: RecommendationRequest,
    ) -> anyhow::Result<RecommendationResponse> {
        let this = self.clone();
        recommend_anime(
            request,
            RecommendationOptions {
                fetch_anime: Box::new(move |id| {
                    let this = this.clone();
                    Box::pin(async move { this.anime(&id).await })
                }),
                fetch_candidates: Box::new({
                    let this = self.clone();
                    move |limit| {
                        let this = this.clone();
                        Box::pin(async move {
                            let anime_limit = (limit + 1) / 2;
                            let popular = this
                                .catalog(CatalogRequest {
                                    catalog_id: "popular".into(),
                                    limit: Some(anime_limit),
                                    ..CatalogRequest::default()
                                })
                                .await?;
                            let airing = this
                                .catalog(CatalogRequest {
                                    catalog_id: "airing".into(),
                                    limit: Some(anime_limit),
                                    ..CatalogRequest::default()
                                })
                                .await?;
                            Ok(unique_by_id(
                                popular.items.into_iter().chain(airing.items).collect(),
                            ))
                        })
                    }
                }),
            },
        )
        .await
    }

    async fn get<T: for<'de> Deserialize<'de>>(&self, path: &str) -> anyhow::Result<T> {
        let client_id = self
            .client_id
            .as_ref()
            .ok_or_else(|| anyhow!("MAL_CLIENT_ID is required"))?;
        let response = self
            .http
            .get(format!("{API_BASE}{path}"))
            .header("X-MAL-CLIENT-ID", client_id)
            .header("accept", "application/json")
            .send()
            .await
            .context("failed to call MAL")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "MAL returned {status}: {}",
                body.chars().take(300).collect::<String>()
            ));
        }

        response
            .json::<T>()
            .await
            .context("failed to parse MAL response")
    }
}

#[derive(Debug, Deserialize)]
struct MalAnimeListResponse {
    data: Vec<MalNode<MalAnime>>,
}

#[derive(Debug, Deserialize)]
struct MalMangaListResponse {
    data: Vec<MalNode<MalManga>>,
}

#[derive(Debug, Deserialize)]
struct MalNode<T> {
    node: T,
}

fn is_manga_request(content_type: Option<&ContentType>, catalog_id: &str) -> bool {
    matches!(content_type, Some(ContentType::Manga)) || catalog_id.starts_with("manga-")
}

fn clamp_limit(limit: Option<u32>) -> u32 {
    limit.unwrap_or(24).clamp(1, 100)
}

fn unique_by_id(items: Vec<AnimePreview>) -> Vec<AnimePreview> {
    let mut out = Vec::new();
    for item in items {
        if !out
            .iter()
            .any(|existing: &AnimePreview| existing.id == item.id)
        {
            out.push(item);
        }
    }
    out
}
