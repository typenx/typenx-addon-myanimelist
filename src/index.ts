import { createTypenxAddon, serveTypenxAddon } from '@typenx/addon-ts-sdk'
import { MyAnimeListCatalog } from './mal.js'

const mal = new MyAnimeListCatalog(process.env.MAL_CLIENT_ID ?? '')

const addon = createTypenxAddon({
  manifest: {
    id: 'typenx-addon-myanimelist',
    name: 'MyAnimeList',
    version: '0.1.0',
    description: 'Official Typenx metadata addon backed by MyAnimeList.',
    icon: 'https://cdn.myanimelist.net/img/sp/icon/apple-touch-icon-256.png',
    resources: ['catalog', 'search', 'anime_meta'],
    catalogs: [
      {
        id: 'popular',
        name: 'Popular Anime',
        content_type: 'anime',
        filters: [],
      },
      {
        id: 'airing',
        name: 'Airing Anime',
        content_type: 'anime',
        filters: [],
      },
    ],
  },
  handlers: {
    catalog: (request) => mal.catalog(request),
    search: (request) => mal.search(request),
    anime: (id) => mal.anime(id),
  },
})

serveTypenxAddon(addon)
