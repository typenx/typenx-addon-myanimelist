# Typenx MyAnimeList Addon

The official Typenx metadata addon for MyAnimeList.

This service speaks the Typenx addon protocol on top of the MyAnimeList API. It returns catalogs, search results, anime and manga metadata, and personalized recommendations without hosting media or returning stream URLs. Plug it into a [Typenx Core](https://github.com/typenx/typenx-core) instance and MAL becomes one more interchangeable source.

## What it provides

- `GET /manifest` - declares `catalog`, `search`, `anime_meta`, and `recommendations` resources.
- `GET /health` - reports whether `MAL_CLIENT_ID` is configured.
- `POST /catalog` - popular, top-ranked, currently airing, and manga feeds.
- `POST /search` - full-text search against the MAL anime or manga index.
- `GET /anime/:id` - full anime metadata: synopsis, genres, studios, episode counts, external links.
- `GET /manga/:id` - full manga metadata: synopsis, authors, serialization, chapter or volume counts, external links.
- `POST /recommendations` - ranked candidates from a like/dislike taste profile.

## Recommendations

`POST /recommendations` accepts liked and disliked anime IDs:

```json
{
  "liked": [{ "anime_id": "5114", "score": 10 }],
  "disliked": [{ "anime_id": "20", "score": 4 }],
  "limit": 24,
  "include_reasons": true
}
```

The first recommender is a hybrid scoring layer: it builds positive and negative taste profiles from MAL metadata, ranks candidates with affinity, score, and recency signals, and diversifies the final list. It is intentionally deterministic and explainable so Typenx can collect clean feedback before training heavier collaborative-filtering models.

Roadmap: persist user feedback, add implicit-feedback matrix factorization, blend collaborative and content scores, and run A/B tests for retention, completion rate, novelty, and dislike avoidance.

## Local development

```powershell
$env:MAL_CLIENT_ID = "your-mal-client-id"
cargo run
```

The service listens on `http://127.0.0.1:8787` by default. You can get a client ID from your MyAnimeList API console.

## Wiring it into Typenx Core

```env
TYPENX_BUILTIN_ADDONS=http://127.0.0.1:8787
```

Use `TYPENX_DEFAULT_ADDONS` instead if you want users to be able to disable it.
