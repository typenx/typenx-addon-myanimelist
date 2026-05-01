# Typenx MyAnimeList Addon

Official Typenx addon service backed by the MyAnimeList API.

This addon provides metadata only:

- Popular anime catalog
- Airing anime catalog
- Anime search
- Anime metadata

It does not return stream URLs or host media.

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
