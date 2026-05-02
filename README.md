# Typenx MyAnimeList Addon

Official Typenx addon service backed by the MyAnimeList API.

This addon provides metadata only:

- Popular anime catalog
- Airing anime catalog
- Anime search
- Anime metadata
- Personalized recommendations

It does not return stream URLs or host media.

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

The first recommender is a hybrid scoring layer: it builds a positive and negative taste profile from MAL metadata, ranks candidates with affinity, score, and recency signals, and diversifies the final list. It is intentionally deterministic and explainable so Typenx can collect clean feedback before training heavier collaborative-filtering models.

Next ML milestones: persist user feedback, add implicit-feedback matrix factorization, blend collaborative and content scores, and run A/B tests for retention, completion rate, novelty, and dislike avoidance.

## Local Development

```powershell
npm install
$env:MAL_CLIENT_ID = "your-mal-client-id"
npm run dev
```

The service listens on `http://127.0.0.1:8787` by default.

Configure Typenx Core with:

```env
TYPENX_BUILTIN_ADDONS=http://127.0.0.1:8787
```
