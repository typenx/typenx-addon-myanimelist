import type {
  AnimeMetadata,
  AnimePreview,
  CatalogRequest,
  CatalogResponse,
  ContentType,
  RecommendationRequest,
  RecommendationResponse,
  SearchRequest,
} from '@typenx/addon-ts-sdk'
import { recommendAnime } from './recommendations.js'

const API_BASE = 'https://api.myanimelist.net/v2'
const FIELDS = [
  'id',
  'title',
  'main_picture',
  'pictures',
  'alternative_titles',
  'synopsis',
  'media_type',
  'start_date',
  'end_date',
  'start_season',
  'status',
  'genres',
  'num_episodes',
  'source',
  'average_episode_duration',
  'rating',
  'mean',
  'rank',
  'popularity',
  'num_list_users',
  'num_scoring_users',
  'studios',
  'broadcast',
  'updated_at',
].join(',')

const MANGA_FIELDS = [
  'id',
  'title',
  'main_picture',
  'pictures',
  'alternative_titles',
  'synopsis',
  'media_type',
  'start_date',
  'end_date',
  'status',
  'genres',
  'num_chapters',
  'num_volumes',
  'authors',
  'serialization',
  'mean',
  'rank',
  'popularity',
  'num_list_users',
  'num_scoring_users',
  'updated_at',
].join(',')

type MalPicture = {
  medium?: string
  large?: string
}

type MalGenre = {
  id: number
  name: string
}

type MalNamedResource = {
  id: number
  name: string
}

type MalAnime = {
  id: number
  title: string
  main_picture?: MalPicture
  pictures?: MalPicture[]
  alternative_titles?: {
    synonyms?: string[]
    en?: string
    ja?: string
  }
  synopsis?: string
  media_type?: string
  start_date?: string
  end_date?: string
  start_season?: {
    year?: number
    season?: string
  }
  status?: string
  genres?: MalGenre[]
  num_episodes?: number
  source?: string
  average_episode_duration?: number
  rating?: string
  mean?: number
  rank?: number
  popularity?: number
  num_list_users?: number
  num_scoring_users?: number
  studios?: MalNamedResource[]
  broadcast?: {
    day_of_the_week?: string
    start_time?: string
  }
  updated_at?: string
}

type MalManga = Omit<
  MalAnime,
  | 'start_season'
  | 'num_episodes'
  | 'source'
  | 'average_episode_duration'
  | 'rating'
  | 'studios'
  | 'broadcast'
> & {
  num_chapters?: number
  num_volumes?: number
  authors?: Array<{
    node?: MalNamedResource
    role?: string
  }>
  serialization?: MalNamedResource[]
}

type MalListResponse = {
  data: Array<{ node: MalAnime }>
}

type MalMangaListResponse = {
  data: Array<{ node: MalManga }>
}

export class MyAnimeListCatalog {
  constructor(private readonly clientId: string | null) {}

  isConfigured() {
    return !!this.clientId
  }

  async catalog(request: CatalogRequest): Promise<CatalogResponse> {
    const limit = clampLimit(request.limit)
    if (request.query) {
      return this.search({ query: request.query, content_type: request.content_type, limit })
    }

    if (isMangaRequest(request)) {
      const rankingType = request.catalog_id === 'manga-favorite' ? 'bypopularity' : 'all'
      const response = await this.get<MalMangaListResponse>(
        `/manga/ranking?ranking_type=${rankingType}&limit=${limit}&fields=${MANGA_FIELDS}`,
      )
      return { items: response.data.map(({ node }) => toMangaPreview(node)) }
    }

    const rankingType = request.catalog_id === 'airing' ? 'airing' : 'all'
    const response = await this.get<MalListResponse>(
      `/anime/ranking?ranking_type=${rankingType}&limit=${limit}&fields=${FIELDS}`,
    )
    return { items: response.data.map(({ node }) => toPreview(node)) }
  }

  async search(request: SearchRequest): Promise<CatalogResponse> {
    const limit = clampLimit(request.limit)
    const query = request.query.trim()
    if (!query) {
      return this.catalog({
        catalog_id: request.content_type === 'manga' ? 'manga-popular' : 'popular',
        content_type: request.content_type,
        limit,
      })
    }

    if (request.content_type === 'manga') {
      const response = await this.get<MalMangaListResponse>(
        `/manga?q=${encodeURIComponent(query)}&limit=${limit}&fields=${MANGA_FIELDS}`,
      )
      return { items: response.data.map(({ node }) => toMangaPreview(node)) }
    }

    const response = await this.get<MalListResponse>(
      `/anime?q=${encodeURIComponent(query)}&limit=${limit}&fields=${FIELDS}`,
    )
    return { items: response.data.map(({ node }) => toPreview(node)) }
  }

  async anime(id: string): Promise<AnimeMetadata> {
    const anime = await this.get<MalAnime>(
      `/anime/${encodeURIComponent(id)}?fields=${FIELDS}`,
    )
    return toMetadata(anime)
  }

  async manga(id: string): Promise<AnimeMetadata> {
    const manga = await this.get<MalManga>(
      `/manga/${encodeURIComponent(id)}?fields=${MANGA_FIELDS}`,
    )
    return toMangaMetadata(manga)
  }

  async recommendations(request: RecommendationRequest): Promise<RecommendationResponse> {
    return recommendAnime(request, {
      fetchAnime: (id) => this.anime(id),
      fetchCandidates: async (limit) => {
        const [popular, airing] = await Promise.all([
          this.catalog({ catalog_id: 'popular', limit: Math.ceil(limit / 2) }),
          this.catalog({ catalog_id: 'airing', limit: Math.ceil(limit / 2) }),
        ])
        return uniqueById([...popular.items, ...airing.items])
      },
    })
  }

  private async get<T>(path: string): Promise<T> {
    if (!this.clientId) {
      throw new Error('MAL_CLIENT_ID is required')
    }

    const response = await fetch(`${API_BASE}${path}`, {
      headers: {
        'X-MAL-CLIENT-ID': this.clientId,
        accept: 'application/json',
      },
    })

    if (!response.ok) {
      const body = await response.text()
      throw new Error(`MAL returned ${response.status}: ${body.slice(0, 300)}`)
    }

    return (await response.json()) as T
  }
}

function toPreview(anime: MalAnime): AnimePreview {
  return {
    id: String(anime.id),
    title: anime.title,
    poster: posterOf(anime),
    banner: bannerOf(anime),
    synopsis: anime.synopsis ?? null,
    score: anime.mean ?? null,
    year: yearOf(anime),
    content_type: contentTypeOf(anime.media_type),
    genres: anime.genres?.map((genre) => genre.name) ?? [],
  }
}

function toMangaPreview(manga: MalManga): AnimePreview {
  return {
    id: String(manga.id),
    title: manga.title,
    poster: posterOf(manga),
    banner: bannerOf(manga),
    synopsis: manga.synopsis ?? null,
    score: manga.mean ?? null,
    year: parseYear(manga.start_date),
    content_type: mangaContentTypeOf(manga.media_type),
    genres: manga.genres?.map((genre) => genre.name) ?? [],
  }
}

function toMetadata(anime: MalAnime): AnimeMetadata {
  const description = anime.synopsis ?? null
  const studios = anime.studios?.map((studio) => studio.name) ?? []
  return {
    id: String(anime.id),
    title: anime.title,
    original_title: anime.alternative_titles?.ja ?? null,
    alternative_titles: alternativeTitlesOf(anime),
    synopsis: description,
    description,
    poster: posterOf(anime),
    banner: bannerOf(anime),
    year: yearOf(anime),
    season: anime.start_season?.season ?? null,
    season_year: anime.start_season?.year ?? yearOf(anime),
    status: anime.status ?? null,
    content_type: contentTypeOf(anime.media_type),
    source: anime.source ?? null,
    duration_minutes: anime.average_episode_duration
      ? Math.round(anime.average_episode_duration / 60)
      : null,
    episode_count: anime.num_episodes ?? null,
    score: anime.mean ?? null,
    rank: anime.rank ?? null,
    popularity: anime.popularity ?? anime.num_list_users ?? null,
    rating: anime.rating ?? null,
    genres: anime.genres?.map((genre) => genre.name) ?? [],
    tags: [],
    authors: [],
    studios,
    staff: [],
    country_of_origin: 'JP',
    start_date: anime.start_date ?? null,
    end_date: anime.end_date ?? null,
    site_url: `https://myanimelist.net/anime/${anime.id}`,
    trailer_url: null,
    external_links: [
      {
        site: 'MyAnimeList',
        url: `https://myanimelist.net/anime/${anime.id}`,
      },
    ],
    episodes: createEpisodes(anime),
    updated_at: anime.updated_at ?? new Date().toISOString(),
  }
}

function toMangaMetadata(manga: MalManga): AnimeMetadata {
  const description = manga.synopsis ?? null
  const authors = uniqueStrings(
    manga.authors?.map((author) => author.node?.name) ?? [],
  )
  return {
    id: String(manga.id),
    title: manga.title,
    original_title: manga.alternative_titles?.ja ?? null,
    alternative_titles: alternativeTitlesOf(manga),
    synopsis: description,
    description,
    poster: posterOf(manga),
    banner: bannerOf(manga),
    year: parseYear(manga.start_date),
    season: null,
    season_year: parseYear(manga.start_date),
    status: manga.status ?? null,
    content_type: mangaContentTypeOf(manga.media_type),
    source: manga.media_type ?? null,
    duration_minutes: null,
    episode_count: manga.num_chapters ?? manga.num_volumes ?? null,
    score: manga.mean ?? null,
    rank: manga.rank ?? null,
    popularity: manga.popularity ?? manga.num_list_users ?? null,
    rating: null,
    genres: manga.genres?.map((genre) => genre.name) ?? [],
    tags: [],
    authors,
    studios: uniqueStrings(
      manga.serialization?.map((serialization) => serialization.name) ?? [],
    ),
    staff:
      manga.authors
        ?.map((author) => ({
          name: author.node?.name ?? '',
          role: author.role ?? null,
        }))
        .filter((credit) => credit.name) ?? [],
    country_of_origin: 'JP',
    start_date: manga.start_date ?? null,
    end_date: manga.end_date ?? null,
    site_url: `https://myanimelist.net/manga/${manga.id}`,
    trailer_url: null,
    external_links: [
      {
        site: 'MyAnimeList',
        url: `https://myanimelist.net/manga/${manga.id}`,
      },
    ],
    episodes: [],
    updated_at: manga.updated_at ?? new Date().toISOString(),
  }
}

function alternativeTitlesOf(anime: MalAnime) {
  return uniqueStrings([
    anime.alternative_titles?.en,
    anime.alternative_titles?.ja,
    ...(anime.alternative_titles?.synonyms ?? []),
  ]).filter((title) => title !== anime.title)
}

function posterOf(anime: MalAnime) {
  return anime.main_picture?.large ?? anime.main_picture?.medium ?? null
}

function bannerOf(anime: MalAnime) {
  const pictures = anime.pictures ?? []
  return (
    pictures.find((picture) => picture.large && picture.large !== posterOf(anime))
      ?.large ??
    pictures[0]?.large ??
    posterOf(anime)
  )
}

function yearOf(anime: MalAnime) {
  return anime.start_season?.year ?? parseYear(anime.start_date)
}

function parseYear(date: string | undefined) {
  const year = date?.slice(0, 4)
  return year && /^\d{4}$/.test(year) ? Number(year) : null
}

function contentTypeOf(mediaType: string | null | undefined): ContentType {
  if (mediaType === 'movie') return 'movie'
  if (mediaType === 'ova') return 'ova'
  if (mediaType === 'ona') return 'ona'
  if (mediaType === 'special') return 'special'
  return 'anime'
}

function mangaContentTypeOf(mediaType: string | null | undefined): ContentType {
  if (mediaType === 'manhwa') return 'manhwa'
  if (mediaType === 'manhua') return 'manhua'
  if (mediaType === 'light_novel' || mediaType === 'novel') return 'light_novel'
  return 'manga'
}

function isMangaRequest(request: CatalogRequest | SearchRequest) {
  const catalogId = 'catalog_id' in request ? request.catalog_id : ''
  return request.content_type === 'manga' || catalogId.startsWith('manga-')
}

function createEpisodes(anime: MalAnime) {
  const count = anime.num_episodes ?? 0
  const duration = anime.average_episode_duration
    ? Math.round(anime.average_episode_duration / 60)
    : null
  return Array.from({ length: count }, (_, index) => {
    const number = index + 1
    return {
      id: `${anime.id}:${number}`,
      anime_id: String(anime.id),
      season_number: null,
      number,
      title: `Episode ${number}`,
      synopsis: null,
      thumbnail: posterOf(anime),
      duration_minutes: duration,
      source: null,
      aired_at: null,
    }
  })
}

function uniqueStrings(values: Array<string | null | undefined>) {
  return Array.from(
    new Set(values.map((value) => value?.trim()).filter((value): value is string => !!value)),
  )
}

function uniqueById(items: AnimePreview[]) {
  return Array.from(new Map(items.map((item) => [item.id, item])).values())
}

function clampLimit(limit: number | undefined) {
  return Math.min(Math.max(limit ?? 24, 1), 100)
}
