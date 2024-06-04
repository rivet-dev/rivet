# Licensing Philosophy

Rivet is licensed under [Apache License 2.0](https://en.wikipedia.org/wiki/Apache_License#Apache_License_2.0)
which is a **permissive open source license**.

## A quick primer

Open source licenses come in three broad flavors:

|                    | Examples         | TLDR                                                                                                                               |
| ------------------ | ---------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| ⭐️ **Permissive** | Apache, MIT, MPL | Allow you to use, modify, and distribute the code as you like, even for commercial purposes, with minimal requirements             |
| **Copyleft**       | GPL, AGPL, SSPL  | Business-hostile license ensures that derivatives of the software also remain free and open source                                 |
| **Public domain**  | PD, CC0          | Work is not subject to copyright and is freely available for anyone to use, modify, and distribute without any restrictions at all |

There are other types of proprietary licenses that we won't cover.

## Why (sometimes) avoid copyleft licenses

A lot of fantastic software is written under copyleft licenses, including GCC, MySQL, Blender, and WireGuard.

The original intention of these types of licenses is to keep software free and encourage sharing source code.
This is done by ensuring that all derivatives of software (e.g. forking and modifying) are open source. Some
variations like SSPL and AGPL include a more strict definition of what is considered a "derivative."

However, licenses like SSPL and AGPL licenses have been abused by for-profit companies in recent years by
companies like MongoDB, Elasticsearch, Confluent, and Redis. Companies like these leverage the branding of
"open source" but in practice, you can't realistically use their software without using their closed-source
cloud offering.

By using copyleft licenses, you lose almost all of the benefits of open source:

- Can't off ramp to other providers if the main provider shuts down or becomes too expensive (i.e.
  anti-competitive)
- Inability to modify software without open sourcing it
- Potential legal risks of misusing software using this license
- Incompatibility with other licenses

More resources:

- [SSPL is not an open source license](https://blog.opensource.org/the-sspl-is-not-an-open-source-license/)
- [The Case Against the SSPL](https://thenewstack.io/the-case-against-the-server-side-public-license-sspl/)

> **Technicalities**
>
> There are a lot of nuances in the types of copyleft licenses and what you can do. We hope this gives a brief
> overview of what to look out for, do your own research if you're looking at using copyleft software.

## Provide competitive value instead of using a business-hostile license

We believe in providing more value to customers at Rivet instead of licensing our source code in a way that is
hostile to businesses.

As a permissive open source company, it's our job to make sure that Rivet Cloud is the most reliable and
fairly priced option on the market. We also provide support plans and enterprise features for people who have
built successful businesses around Rivet.

## Why not another permissive license

There's many other permissive open source licenses, like MIT and MPL.

We chose Apache 2.0 for the following reasons:

- **Well established** Apache 2.0 is used by the majority of large companies for their open source software
- **Trademark protections** Explicitly states the limitations on the use of our trademarks
- **Patent rights** Explicitly gives the right to file patents on our software
- **Compatibility** Apache 2.0 can be used within GPL-licensed software

## Feedback

If you have any questions, comments, or concerns about our licensing, we'd love to hear from you on
[Discord](https://discord.gg/BG2vqsJczH).
