# SaltStack Gotchas

## How to generate source hashes?

SlatStack requires anything downloaded from a URL to have a provided source hash.

Some files don't provide a source hash on a server. In this case, you must run the following:

```bash
curl $URL | sha512sum
```

Copy the output without the extra dash.

