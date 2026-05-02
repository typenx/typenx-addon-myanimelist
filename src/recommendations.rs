use std::{collections::HashMap, future::Future, pin::Pin};

use crate::types::{
    AnimeMetadata, AnimePreview, RecommendationPreview, RecommendationRequest,
    RecommendationResponse, RecommendationSeed,
};

type BoxFuture<T> = Pin<Box<dyn Future<Output = anyhow::Result<T>> + Send>>;

pub struct RecommendationOptions {
    pub fetch_anime: Box<dyn Fn(String) -> BoxFuture<AnimeMetadata> + Send + Sync>,
    pub fetch_candidates: Box<dyn Fn(u32) -> BoxFuture<Vec<AnimePreview>> + Send + Sync>,
}

#[derive(Debug, Clone)]
struct ScoredSeed {
    anime: AnimeMetadata,
    weight: f64,
}

#[derive(Debug, Clone)]
struct Profile {
    weights: HashMap<String, f64>,
    preferred_year: Option<f64>,
}

pub async fn recommend_anime(
    request: RecommendationRequest,
    options: RecommendationOptions,
) -> anyhow::Result<RecommendationResponse> {
    let limit = clamp(request.limit.unwrap_or(24) as f64, 1.0, 50.0) as usize;
    let candidate_limit = clamp(
        request.candidate_limit.unwrap_or((limit * 5) as u32) as f64,
        limit as f64,
        200.0,
    ) as u32;
    let liked = hydrate_seeds(&request.liked, &options.fetch_anime, 1.0).await?;
    let disliked = hydrate_seeds(
        request.disliked.as_deref().unwrap_or_default(),
        &options.fetch_anime,
        -1.0,
    )
    .await?;

    if liked.is_empty() {
        return Ok(RecommendationResponse { items: vec![] });
    }

    let profile = build_profile(&liked, &disliked);
    let seen_ids = liked
        .iter()
        .chain(disliked.iter())
        .map(|seed| seed.anime.id.clone())
        .collect::<Vec<_>>();
    let mut scored = (options.fetch_candidates)(candidate_limit)
        .await?
        .into_iter()
        .filter(|candidate| !seen_ids.contains(&candidate.id))
        .map(|candidate| {
            score_candidate(candidate, &profile, request.include_reasons.unwrap_or(true))
        })
        .collect::<Vec<_>>();
    scored.sort_by(|a, b| {
        b.recommendation_score
            .partial_cmp(&a.recommendation_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(RecommendationResponse {
        items: diversify(scored, limit),
    })
}

async fn hydrate_seeds(
    seeds: &[RecommendationSeed],
    fetch_anime: &(dyn Fn(String) -> BoxFuture<AnimeMetadata> + Send + Sync),
    polarity: f64,
) -> anyhow::Result<Vec<ScoredSeed>> {
    let mut hydrated = Vec::new();
    for seed in seeds.iter().filter(|seed| !seed.anime_id.is_empty()) {
        hydrated.push(ScoredSeed {
            anime: fetch_anime(seed.anime_id.clone()).await?,
            weight: seed_weight(seed) * polarity,
        });
    }
    Ok(hydrated)
}

fn seed_weight(seed: &RecommendationSeed) -> f64 {
    if let Some(weight) = seed.weight {
        return clamp(weight, 0.1, 3.0);
    }
    if let Some(score) = seed.score {
        return clamp((score - 5.0).abs() / 2.5, 0.2, 2.0);
    }
    1.0
}

fn build_profile(liked: &[ScoredSeed], disliked: &[ScoredSeed]) -> Profile {
    let mut weights = HashMap::new();
    let mut years = Vec::new();

    for seed in liked.iter().chain(disliked.iter()) {
        for feature in metadata_features(&seed.anime) {
            *weights.entry(feature).or_insert(0.0) += seed.weight;
        }
        if seed.weight > 0.0 {
            if let Some(year) = seed.anime.year {
                years.push(year as f64);
            }
        }
    }

    Profile {
        weights,
        preferred_year: mean(&years),
    }
}

fn score_candidate(
    candidate: AnimePreview,
    profile: &Profile,
    include_reasons: bool,
) -> RecommendationPreview {
    let features = preview_features(&candidate);
    let affinity = features
        .iter()
        .map(|feature| profile.weights.get(feature).copied().unwrap_or(0.0))
        .sum::<f64>();
    let normalized_affinity = if features.is_empty() {
        0.0
    } else {
        affinity / (features.len() as f64).sqrt()
    };
    let quality = candidate.score.unwrap_or(0.0) / 10.0;
    let recency = match (profile.preferred_year, candidate.year) {
        (Some(preferred), Some(year)) => (1.0 - ((year as f64 - preferred).abs() / 20.0)).max(0.0),
        _ => 0.25,
    };
    let score = normalized_affinity * 0.72 + quality * 0.18 + recency * 0.1;
    let reasons = include_reasons.then(|| reasons_for(&candidate, &features, profile));

    RecommendationPreview {
        preview: candidate,
        recommendation_score: (score * 10_000.0).round() / 10_000.0,
        reasons,
    }
}

fn diversify(items: Vec<RecommendationPreview>, limit: usize) -> Vec<RecommendationPreview> {
    let mut selected: Vec<RecommendationPreview> = Vec::new();
    for item in items {
        let item_genres = item.preview.genres.clone().unwrap_or_default();
        let overlaps = selected
            .iter()
            .filter(|selected_item| {
                selected_item
                    .preview
                    .genres
                    .as_deref()
                    .unwrap_or_default()
                    .iter()
                    .any(|genre| item_genres.contains(genre))
            })
            .count();
        if overlaps < 4 || selected.len() < limit.div_ceil(3) {
            selected.push(item);
        }
        if selected.len() >= limit {
            break;
        }
    }
    selected
}

fn metadata_features(anime: &AnimeMetadata) -> Vec<String> {
    anime
        .genres
        .iter()
        .map(|genre| format!("genre:{}", normalize(genre)))
        .chain(
            anime
                .tags
                .iter()
                .map(|tag| format!("tag:{}", normalize(tag))),
        )
        .chain(std::iter::once(format!(
            "type:{}",
            content_type_feature(&anime.content_type)
        )))
        .chain(anime.year.map(|year| format!("era:{}", (year / 5) * 5)))
        .collect()
}

fn preview_features(anime: &AnimePreview) -> Vec<String> {
    anime
        .genres
        .as_deref()
        .unwrap_or_default()
        .iter()
        .map(|genre| format!("genre:{}", normalize(genre)))
        .chain(std::iter::once(format!(
            "type:{}",
            content_type_feature(&anime.content_type)
        )))
        .chain(anime.year.map(|year| format!("era:{}", (year / 5) * 5)))
        .collect()
}

fn reasons_for(candidate: &AnimePreview, features: &[String], profile: &Profile) -> Vec<String> {
    let mut matches = features
        .iter()
        .filter(|feature| profile.weights.get(*feature).copied().unwrap_or(0.0) > 0.0)
        .take(3)
        .map(|feature| {
            feature
                .trim_start_matches("genre:")
                .trim_start_matches("tag:")
                .trim_start_matches("type:")
                .trim_start_matches("era:")
                .replace('-', " ")
        })
        .collect::<Vec<_>>();
    if candidate.score.is_some_and(|score| score >= 8.0) {
        matches.push("strong community score".to_string());
    }
    matches.truncate(4);
    matches
}

fn normalize(value: &str) -> String {
    let mut out = String::new();
    let mut previous_dash = false;
    for ch in value.trim().to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            previous_dash = false;
        } else if !previous_dash {
            out.push('-');
            previous_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

fn content_type_feature(content_type: &crate::types::ContentType) -> &'static str {
    match content_type {
        crate::types::ContentType::Anime => "anime",
        crate::types::ContentType::Manga => "manga",
        crate::types::ContentType::Manhwa => "manhwa",
        crate::types::ContentType::Manhua => "manhua",
        crate::types::ContentType::LightNovel => "light_novel",
        crate::types::ContentType::Movie => "movie",
        crate::types::ContentType::Ova => "ova",
        crate::types::ContentType::Ona => "ona",
        crate::types::ContentType::Special => "special",
    }
}

fn mean(values: &[f64]) -> Option<f64> {
    (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
}

fn clamp(value: f64, min: f64, max: f64) -> f64 {
    value.max(min).min(max)
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc};

    use super::*;
    use crate::types::{ContentType, ExternalLink};

    fn metadata(id: &str, genres: Vec<&str>, year: u32) -> AnimeMetadata {
        AnimeMetadata {
            id: id.into(),
            title: id.into(),
            original_title: None,
            alternative_titles: vec![],
            synopsis: None,
            description: None,
            poster: None,
            banner: None,
            year: Some(year),
            season: None,
            season_year: Some(year),
            status: None,
            content_type: ContentType::Anime,
            source: None,
            duration_minutes: Some(24),
            episode_count: None,
            score: None,
            rank: None,
            popularity: None,
            rating: None,
            genres: genres.into_iter().map(str::to_string).collect(),
            tags: vec![],
            authors: vec![],
            studios: vec![],
            staff: vec![],
            country_of_origin: None,
            start_date: None,
            end_date: None,
            site_url: None,
            trailer_url: None,
            external_links: vec![ExternalLink {
                site: "x".into(),
                url: "u".into(),
            }],
            episodes: vec![],
            updated_at: None,
        }
    }

    fn preview(id: &str, genres: Vec<&str>, score: f64, year: u32) -> AnimePreview {
        AnimePreview {
            id: id.into(),
            title: id.into(),
            poster: None,
            banner: None,
            synopsis: None,
            score: Some(score),
            year: Some(year),
            content_type: ContentType::Anime,
            genres: Some(genres.into_iter().map(str::to_string).collect()),
            season_entries: None,
        }
    }

    #[tokio::test]
    async fn recommends_affine_candidates_and_excludes_seen() {
        let seeds = Arc::new(HashMap::from([
            (
                "liked".to_string(),
                metadata("liked", vec!["Action", "Drama"], 2020),
            ),
            (
                "disliked".to_string(),
                metadata("disliked", vec!["Comedy"], 2000),
            ),
        ]));
        let candidates = vec![
            preview("liked", vec!["Action"], 9.0, 2020),
            preview("good", vec!["Action"], 8.0, 2021),
            preview("bad", vec!["Comedy"], 9.8, 2000),
        ];

        let response = recommend_anime(
            RecommendationRequest {
                liked: vec![RecommendationSeed {
                    anime_id: "liked".into(),
                    score: None,
                    weight: None,
                }],
                disliked: Some(vec![RecommendationSeed {
                    anime_id: "disliked".into(),
                    score: None,
                    weight: None,
                }]),
                limit: Some(2),
                include_reasons: Some(true),
                ..RecommendationRequest::default()
            },
            RecommendationOptions {
                fetch_anime: Box::new({
                    let seeds = Arc::clone(&seeds);
                    move |id| {
                        let seeds = Arc::clone(&seeds);
                        Box::pin(async move { Ok(seeds.get(&id).unwrap().clone()) })
                    }
                }),
                fetch_candidates: Box::new(move |_| {
                    let candidates = candidates.clone();
                    Box::pin(async move { Ok(candidates) })
                }),
            },
        )
        .await
        .unwrap();

        assert_eq!(response.items[0].preview.id, "good");
        assert!(response.items.iter().all(|item| item.preview.id != "liked"));
        assert_eq!(
            response.items[0].reasons.as_ref().unwrap(),
            &vec![
                "action".to_string(),
                "2020".to_string(),
                "strong community score".to_string()
            ]
        );
    }

    #[tokio::test]
    async fn returns_empty_without_liked_seeds() {
        let response = recommend_anime(
            RecommendationRequest::default(),
            RecommendationOptions {
                fetch_anime: Box::new(|_| Box::pin(async { unreachable!() })),
                fetch_candidates: Box::new(|_| Box::pin(async { unreachable!() })),
            },
        )
        .await
        .unwrap();

        assert!(response.items.is_empty());
    }
}
