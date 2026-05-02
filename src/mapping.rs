use serde::Deserialize;

use crate::types::{
    AnimeMetadata, AnimePreview, ContentType, EpisodeMetadata, ExternalLink, StaffCredit,
};

pub const ANIME_FIELDS: &str = "id,title,main_picture,pictures,alternative_titles,synopsis,media_type,start_date,end_date,start_season,status,genres,num_episodes,source,average_episode_duration,rating,mean,rank,popularity,num_list_users,num_scoring_users,studios,broadcast,updated_at";
pub const MANGA_FIELDS: &str = "id,title,main_picture,pictures,alternative_titles,synopsis,media_type,start_date,end_date,status,genres,num_chapters,num_volumes,authors,serialization,mean,rank,popularity,num_list_users,num_scoring_users,updated_at";

#[derive(Debug, Clone, Deserialize)]
pub struct MalPicture {
    pub medium: Option<String>,
    pub large: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MalNamedResource {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MalAlternativeTitles {
    pub synonyms: Option<Vec<String>>,
    pub en: Option<String>,
    pub ja: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MalSeason {
    pub year: Option<u32>,
    pub season: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MalAnime {
    pub id: u32,
    pub title: String,
    pub main_picture: Option<MalPicture>,
    pub pictures: Option<Vec<MalPicture>>,
    pub alternative_titles: Option<MalAlternativeTitles>,
    pub synopsis: Option<String>,
    pub media_type: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub start_season: Option<MalSeason>,
    pub status: Option<String>,
    pub genres: Option<Vec<MalNamedResource>>,
    pub num_episodes: Option<u32>,
    pub source: Option<String>,
    pub average_episode_duration: Option<u32>,
    pub rating: Option<String>,
    pub mean: Option<f64>,
    pub rank: Option<u32>,
    pub popularity: Option<u32>,
    pub num_list_users: Option<u32>,
    pub num_scoring_users: Option<u32>,
    pub studios: Option<Vec<MalNamedResource>>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MalAuthor {
    pub node: Option<MalNamedResource>,
    pub role: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MalManga {
    pub id: u32,
    pub title: String,
    pub main_picture: Option<MalPicture>,
    pub pictures: Option<Vec<MalPicture>>,
    pub alternative_titles: Option<MalAlternativeTitles>,
    pub synopsis: Option<String>,
    pub media_type: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub status: Option<String>,
    pub genres: Option<Vec<MalNamedResource>>,
    pub num_chapters: Option<u32>,
    pub num_volumes: Option<u32>,
    pub authors: Option<Vec<MalAuthor>>,
    pub serialization: Option<Vec<MalNamedResource>>,
    pub mean: Option<f64>,
    pub rank: Option<u32>,
    pub popularity: Option<u32>,
    pub num_list_users: Option<u32>,
    pub updated_at: Option<String>,
}

pub fn to_preview(anime: &MalAnime) -> AnimePreview {
    AnimePreview {
        id: anime.id.to_string(),
        title: anime.title.clone(),
        poster: poster_of(&anime.main_picture),
        banner: banner_of(&anime.main_picture, anime.pictures.as_deref()),
        synopsis: anime.synopsis.clone(),
        score: anime.mean,
        year: year_of(anime),
        content_type: anime_content_type(anime.media_type.as_deref()),
        genres: Some(names(anime.genres.as_deref())),
        season_entries: None,
    }
}

pub fn to_manga_preview(manga: &MalManga) -> AnimePreview {
    AnimePreview {
        id: manga.id.to_string(),
        title: manga.title.clone(),
        poster: poster_of(&manga.main_picture),
        banner: banner_of(&manga.main_picture, manga.pictures.as_deref()),
        synopsis: manga.synopsis.clone(),
        score: manga.mean,
        year: parse_year(manga.start_date.as_deref()),
        content_type: manga_content_type(manga.media_type.as_deref()),
        genres: Some(names(manga.genres.as_deref())),
        season_entries: None,
    }
}

pub fn to_metadata(anime: &MalAnime) -> AnimeMetadata {
    let description = anime.synopsis.clone();
    let year = year_of(anime);
    AnimeMetadata {
        id: anime.id.to_string(),
        title: anime.title.clone(),
        original_title: anime
            .alternative_titles
            .as_ref()
            .and_then(|titles| titles.ja.clone()),
        alternative_titles: alternative_titles_of(anime.alternative_titles.as_ref(), &anime.title),
        synopsis: description.clone(),
        description,
        poster: poster_of(&anime.main_picture),
        banner: banner_of(&anime.main_picture, anime.pictures.as_deref()),
        year,
        season: anime
            .start_season
            .as_ref()
            .and_then(|season| season.season.clone()),
        season_year: anime
            .start_season
            .as_ref()
            .and_then(|season| season.year)
            .or(year),
        status: anime.status.clone(),
        content_type: anime_content_type(anime.media_type.as_deref()),
        source: anime.source.clone(),
        duration_minutes: anime
            .average_episode_duration
            .map(|seconds| (seconds + 30) / 60),
        episode_count: anime.num_episodes,
        score: anime.mean,
        rank: anime.rank,
        popularity: anime.popularity.or(anime.num_list_users),
        rating: anime.rating.clone(),
        genres: names(anime.genres.as_deref()),
        tags: vec![],
        authors: vec![],
        studios: names(anime.studios.as_deref()),
        staff: vec![],
        country_of_origin: Some("JP".to_string()),
        start_date: anime.start_date.clone(),
        end_date: anime.end_date.clone(),
        site_url: Some(format!("https://myanimelist.net/anime/{}", anime.id)),
        trailer_url: None,
        external_links: vec![ExternalLink {
            site: "MyAnimeList".to_string(),
            url: format!("https://myanimelist.net/anime/{}", anime.id),
        }],
        episodes: create_episodes(anime),
        updated_at: anime.updated_at.clone().or_else(now_iso),
    }
}

pub fn to_manga_metadata(manga: &MalManga) -> AnimeMetadata {
    let description = manga.synopsis.clone();
    let year = parse_year(manga.start_date.as_deref());
    AnimeMetadata {
        id: manga.id.to_string(),
        title: manga.title.clone(),
        original_title: manga
            .alternative_titles
            .as_ref()
            .and_then(|titles| titles.ja.clone()),
        alternative_titles: alternative_titles_of(manga.alternative_titles.as_ref(), &manga.title),
        synopsis: description.clone(),
        description,
        poster: poster_of(&manga.main_picture),
        banner: banner_of(&manga.main_picture, manga.pictures.as_deref()),
        year,
        season: None,
        season_year: year,
        status: manga.status.clone(),
        content_type: manga_content_type(manga.media_type.as_deref()),
        source: manga.media_type.clone(),
        duration_minutes: None,
        episode_count: positive(manga.num_chapters).or_else(|| positive(manga.num_volumes)),
        score: manga.mean,
        rank: manga.rank,
        popularity: manga.popularity.or(manga.num_list_users),
        rating: None,
        genres: names(manga.genres.as_deref()),
        tags: vec![],
        authors: unique_strings(
            manga
                .authors
                .as_deref()
                .unwrap_or_default()
                .iter()
                .filter_map(|author| author.node.as_ref().map(|node| node.name.clone())),
        ),
        studios: unique_strings(
            manga
                .serialization
                .as_deref()
                .unwrap_or_default()
                .iter()
                .map(|item| item.name.clone()),
        ),
        staff: manga
            .authors
            .as_deref()
            .unwrap_or_default()
            .iter()
            .filter_map(|author| {
                let name = author.node.as_ref()?.name.clone();
                (!name.is_empty()).then_some(StaffCredit {
                    name,
                    role: author.role.clone(),
                })
            })
            .collect(),
        country_of_origin: Some("JP".to_string()),
        start_date: manga.start_date.clone(),
        end_date: manga.end_date.clone(),
        site_url: Some(format!("https://myanimelist.net/manga/{}", manga.id)),
        trailer_url: None,
        external_links: vec![ExternalLink {
            site: "MyAnimeList".to_string(),
            url: format!("https://myanimelist.net/manga/{}", manga.id),
        }],
        episodes: vec![],
        updated_at: manga.updated_at.clone().or_else(now_iso),
    }
}

fn poster_of(picture: &Option<MalPicture>) -> Option<String> {
    picture
        .as_ref()
        .and_then(|picture| picture.large.clone().or_else(|| picture.medium.clone()))
}

fn banner_of(main: &Option<MalPicture>, pictures: Option<&[MalPicture]>) -> Option<String> {
    let poster = poster_of(main);
    let pictures = pictures.unwrap_or_default();
    pictures
        .iter()
        .find_map(|picture| {
            let large = picture.large.clone()?;
            (Some(large.clone()) != poster).then_some(large)
        })
        .or_else(|| pictures.first().and_then(|picture| picture.large.clone()))
        .or(poster)
}

fn year_of(anime: &MalAnime) -> Option<u32> {
    anime
        .start_season
        .as_ref()
        .and_then(|season| season.year)
        .or_else(|| parse_year(anime.start_date.as_deref()))
}

pub fn parse_year(date: Option<&str>) -> Option<u32> {
    let year = date?.get(0..4)?;
    (year.len() == 4 && year.chars().all(|ch| ch.is_ascii_digit()))
        .then(|| year.parse().ok())
        .flatten()
}

fn anime_content_type(media_type: Option<&str>) -> ContentType {
    match media_type {
        Some("movie") => ContentType::Movie,
        Some("ova") => ContentType::Ova,
        Some("ona") => ContentType::Ona,
        Some("special") => ContentType::Special,
        _ => ContentType::Anime,
    }
}

fn manga_content_type(media_type: Option<&str>) -> ContentType {
    match media_type {
        Some("manhwa") => ContentType::Manhwa,
        Some("manhua") => ContentType::Manhua,
        Some("light_novel" | "novel") => ContentType::LightNovel,
        _ => ContentType::Manga,
    }
}

fn alternative_titles_of(titles: Option<&MalAlternativeTitles>, main_title: &str) -> Vec<String> {
    let Some(titles) = titles else {
        return vec![];
    };
    unique_strings(
        titles
            .en
            .clone()
            .into_iter()
            .chain(titles.ja.clone())
            .chain(titles.synonyms.clone().unwrap_or_default())
            .filter(|title| title != main_title),
    )
}

fn names(resources: Option<&[MalNamedResource]>) -> Vec<String> {
    resources
        .unwrap_or_default()
        .iter()
        .map(|resource| resource.name.clone())
        .collect()
}

fn create_episodes(anime: &MalAnime) -> Vec<EpisodeMetadata> {
    let duration = anime
        .average_episode_duration
        .map(|seconds| (seconds + 30) / 60);
    (1..=anime.num_episodes.unwrap_or(0))
        .map(|number| EpisodeMetadata {
            id: format!("{}:{}", anime.id, number),
            anime_id: anime.id.to_string(),
            season_number: None,
            number,
            title: Some(format!("Episode {}", number)),
            synopsis: None,
            thumbnail: poster_of(&anime.main_picture),
            duration_minutes: duration,
            source: None,
            aired_at: None,
        })
        .collect()
}

fn unique_strings(values: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut out = Vec::new();
    for value in values {
        let value = value.trim().to_string();
        if !value.is_empty() && !out.contains(&value) {
            out.push(value);
        }
    }
    out
}

fn positive(value: Option<u32>) -> Option<u32> {
    value.filter(|value| *value > 0)
}

fn now_iso() -> Option<String> {
    Some(chrono::Utc::now().to_rfc3339())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_anime_metadata_with_titles_episodes_and_type() {
        let anime = MalAnime {
            id: 42,
            title: "Example".to_string(),
            main_picture: Some(MalPicture {
                medium: Some("m".into()),
                large: Some("poster".into()),
            }),
            pictures: Some(vec![
                MalPicture {
                    medium: None,
                    large: Some("poster".into()),
                },
                MalPicture {
                    medium: None,
                    large: Some("banner".into()),
                },
            ]),
            alternative_titles: Some(MalAlternativeTitles {
                synonyms: Some(vec!["Example".into(), "Alt".into()]),
                en: Some("English".into()),
                ja: Some("Japanese".into()),
            }),
            synopsis: Some("About it".into()),
            media_type: Some("ova".into()),
            start_date: Some("2020-01-01".into()),
            end_date: None,
            start_season: Some(MalSeason {
                year: Some(2021),
                season: Some("winter".into()),
            }),
            status: Some("finished_airing".into()),
            genres: Some(vec![MalNamedResource {
                id: 1,
                name: "Action".into(),
            }]),
            num_episodes: Some(2),
            source: Some("manga".into()),
            average_episode_duration: Some(1500),
            rating: Some("pg_13".into()),
            mean: Some(8.2),
            rank: Some(10),
            popularity: None,
            num_list_users: Some(99),
            num_scoring_users: None,
            studios: Some(vec![MalNamedResource {
                id: 2,
                name: "Bones".into(),
            }]),
            updated_at: Some("updated".into()),
        };

        let metadata = to_metadata(&anime);

        assert_eq!(metadata.content_type, ContentType::Ova);
        assert_eq!(metadata.banner.as_deref(), Some("banner"));
        assert_eq!(
            metadata.alternative_titles,
            vec!["English", "Japanese", "Alt"]
        );
        assert_eq!(metadata.duration_minutes, Some(25));
        assert_eq!(metadata.episode_count, Some(2));
        assert_eq!(metadata.episodes[1].id, "42:2");
        assert_eq!(metadata.season_year, Some(2021));
    }

    #[test]
    fn maps_manga_authors_serialization_and_positive_counts() {
        let manga = MalManga {
            id: 7,
            title: "Book".into(),
            main_picture: None,
            pictures: None,
            alternative_titles: None,
            synopsis: None,
            media_type: Some("novel".into()),
            start_date: Some("1999".into()),
            end_date: None,
            status: None,
            genres: None,
            num_chapters: Some(0),
            num_volumes: Some(3),
            authors: Some(vec![
                MalAuthor {
                    node: Some(MalNamedResource {
                        id: 1,
                        name: "Writer".into(),
                    }),
                    role: Some("Story".into()),
                },
                MalAuthor {
                    node: Some(MalNamedResource {
                        id: 1,
                        name: "Writer".into(),
                    }),
                    role: Some("Art".into()),
                },
            ]),
            serialization: Some(vec![MalNamedResource {
                id: 2,
                name: "Jump".into(),
            }]),
            mean: None,
            rank: None,
            popularity: None,
            num_list_users: None,
            updated_at: Some("updated".into()),
        };

        let metadata = to_manga_metadata(&manga);

        assert_eq!(metadata.content_type, ContentType::LightNovel);
        assert_eq!(metadata.episode_count, Some(3));
        assert_eq!(metadata.authors, vec!["Writer"]);
        assert_eq!(metadata.studios, vec!["Jump"]);
        assert_eq!(metadata.staff.len(), 2);
    }
}
