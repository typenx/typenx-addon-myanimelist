import type {
  AnimeMetadata,
  AnimePreview,
  CatalogRequest,
  CatalogResponse,
  SearchRequest,
} from '@typenx/addon-sdk'

const API_BASE = 'https://api.myanimelist.net/v2'
const FIELDS = [
  'id',
  'title',
  'main_picture',
  'alternative_titles',
  'synopsis',
  'start_season',
  'status',
  'genres',
  'num_episodes',
].join(',')

type MalPicture = {
  medium?: string
  large?: string
}

type MalGenre = {
  id: number
  name: string
}

type MalAnime = {
  id: number
  title: string
  main_picture?: MalPicture
  alternative_titles?: {
    en?: string
    ja?: string
  }
  synopsis?: string
  start_season?: {
    year?: number
  }
  status?: string
  genres?: MalGenre[]
  num_episodes?: number
}

type MalListResponse = {
  data: Array<{ node: MalAnime }>
}

export class MyAnimeListCatalog {
  constructor(private readonly clientId: string) {
    if (!clientId) {
      throw new Error('MAL_CLIENT_ID is required')
    }
  }

  async catalog(request: CatalogRequest): Promise<CatalogResponse> {
    const limit = clampLimit(request.limit)
    if (request.query) {
      return this.search({ query: request.query, limit })
    }

    const rankingType = request.catalog_id === 'airing' ? 'airing' : 'all'
    const response = await this.get<MalListResponse>(
      `/anime/ranking?ranking_type=${rankingType}&limit=${limit}&fields=${FIELDS}`,
    )
    return { items: response.data.map(({ node }) => toPreview(node)) }
  }

  async search(request: SearchRequest): Promise<CatalogResponse> {
    const limit = clampLimit(request.limit)
    const response = await this.get<MalListResponse>(
      `/anime?q=${encodeURIComponent(request.query)}&limit=${limit}&fields=${FIELDS}`,
    )
    return { items: response.data.map(({ node }) => toPreview(node)) }
  }

  async anime(id: string): Promise<AnimeMetadata> {
    const anime = await this.get<MalAnime>(`/anime/${encodeURIComponent(id)}?fields=${FIELDS}`)
    return toMetadata(anime)
  }

  private async get<T>(path: string): Promise<T> {
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
    poster: anime.main_picture?.large ?? anime.main_picture?.medium ?? null,
    year: anime.start_season?.year ?? null,
    content_type: 'anime',
  }
}

function toMetadata(anime: MalAnime): AnimeMetadata {
  const poster = anime.main_picture?.large ?? anime.main_picture?.medium ?? null
  return {
    id: String(anime.id),
    title: anime.title,
    original_title: anime.alternative_titles?.ja ?? null,
    synopsis: anime.synopsis ?? null,
    poster,
    banner: poster,
    year: anime.start_season?.year ?? null,
    status: anime.status ?? null,
    genres: anime.genres?.map((genre) => genre.name) ?? [],
    episodes: createEpisodes(String(anime.id), anime.num_episodes ?? 0),
    updated_at: new Date().toISOString(),
  }
}

function createEpisodes(animeId: string, count: number) {
  return Array.from({ length: count }, (_, index) => {
    const number = index + 1
    return {
      id: `${animeId}:${number}`,
      anime_id: animeId,
      number,
      title: null,
      synopsis: null,
      thumbnail: null,
      aired_at: null,
    }
  })
}

function clampLimit(limit: number | undefined) {
  return Math.min(Math.max(limit ?? 24, 1), 100)
}
