# Media Deliver and Resizing

## Architecture

### Uploading

We don't resize anything until the image is fetched.

```
Prepare: Browser -> upload-prepare
Upload: Browser -> Backblaze
Complete: Browser -> upload-complete
```

### Fetching

Media is resized on demand when the client fetches it.

```
Browser -> Cloudflare -> Traefik -> Imagor (resizes) -> ATS (caches) -> Backblaze (storage)
```

1. Cloudflare serves requests with a cache
1. Traefik is responsible for expanding the URL in to the appropriate preset for Imagor (see below on DoS
   protection)
1. Imagor will attempt to fetch the cached image. If it can't find it, it will fetch the master image and
   resize and cache the resized image appropriately.
1. ATS will cache any images that were recently fetched from Backblaze
1. Backblaze is responsible for storing the master images

## Prefixes for DoS Protection

Vanilla Imagor allows for applying any arbitrary resize and filter to images. This enables attackers to
request large amounts of image resizes and cripples our caching layer.

Treafik is configured to expose a pre-defined set of filter that get applied to the image and Treafik will
automatically prefix the URL with the correct filter. This way, attackers can't define their own filters and
we have a limited number of images we're responsible for resizing.
