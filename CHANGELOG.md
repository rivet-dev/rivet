# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Calendar Versioning](https://calver.org/).

## [24.6.2-rc.1](https://github.com/rivet-gg/rivet/compare/v24.6.1...v24.6.2-rc.1) (2025-01-05)


### Bug Fixes

* **fern:** fix generation script with latest nix-shell version on macos ([#1794](https://github.com/rivet-gg/rivet/issues/1794)) ([2acda68](https://github.com/rivet-gg/rivet/commit/2acda689598f8cbc18b0d14aeaa3e79d39474b8a))
* **release:** correct quotes on release please commit message ([c38d89b](https://github.com/rivet-gg/rivet/commit/c38d89ba8eb6682485a1231029c6dd9d8399b0f7))
* **release:** update release script ([#1799](https://github.com/rivet-gg/rivet/issues/1799)) ([ae7f5d4](https://github.com/rivet-gg/rivet/commit/ae7f5d40e25bb7da5b0ccc588331a28a72d27c7c))


### Chores

* **docker:** add universal dockerfile ([#1792](https://github.com/rivet-gg/rivet/issues/1792)) ([89e31b2](https://github.com/rivet-gg/rivet/commit/89e31b2848a9cb6f28e625d2a7df1d7ee4962bf6))
* **fern:** remove building typescript archives ([#1793](https://github.com/rivet-gg/rivet/issues/1793)) ([bc02f63](https://github.com/rivet-gg/rivet/commit/bc02f633ce39a51f2e5385923e023a9fc36448b1))
* **main:** release 24.6.2-rc.1' ([#1797](https://github.com/rivet-gg/rivet/issues/1797)) ([4c3b7ce](https://github.com/rivet-gg/rivet/commit/4c3b7ce5ec6b844729d1235f664d6c6311c81c6b))
* release 24.6.2-rc.1 ([ff0c81b](https://github.com/rivet-gg/rivet/commit/ff0c81b143dac1437086d4f488546ecf940524e4))
* **release:** update version to 24.6.2-rc.1 ([75efeac](https://github.com/rivet-gg/rivet/commit/75efeac98a869ee6dafdf8215196f87ee7eb8587))
* **release:** update version to 24.6.2-rc.1 ([55f9c0b](https://github.com/rivet-gg/rivet/commit/55f9c0b34e202677d94783692c615c8d871d0a62))
* update release script to be fully automated dispatch ([#1795](https://github.com/rivet-gg/rivet/issues/1795)) ([7d105ba](https://github.com/rivet-gg/rivet/commit/7d105baeeb1631dec50d42715888b927aa6b438c))

## [24.6.1](https://github.com/rivet-gg/rivet/compare/v24.6.0...v24.6.1) (2025-01-03)


### Features

* add easy to use isolate runner test ([#1750](https://github.com/rivet-gg/rivet/issues/1750)) ([1994555](https://github.com/rivet-gg/rivet/commit/1994555b5561c1aa1a66d1832d15f78817a56d46))
* add pb logs rotation and retention ([#1745](https://github.com/rivet-gg/rivet/issues/1745)) ([911358e](https://github.com/rivet-gg/rivet/commit/911358e7af815d430dfce995b336a72f50e6df3a))


### Bug Fixes

* **actor:** increase max actor & build tag length ([#1758](https://github.com/rivet-gg/rivet/issues/1758)) ([de64037](https://github.com/rivet-gg/rivet/commit/de64037af431ea478aca4b66db0c6e6d8c227ce5))
* add ignore future state for pegboard actors ([#1719](https://github.com/rivet-gg/rivet/issues/1719)) ([29dc8ea](https://github.com/rivet-gg/rivet/commit/29dc8ea0e787a7eac72da74951d621a435016c10))
* add metric for dup client events ([#1714](https://github.com/rivet-gg/rivet/issues/1714)) ([8222348](https://github.com/rivet-gg/rivet/commit/822234806f54615ce26b8584a9f3e44bce1eb1ec))
* add nonreporting server metrics ([#1763](https://github.com/rivet-gg/rivet/issues/1763)) ([3fbcdbd](https://github.com/rivet-gg/rivet/commit/3fbcdbd2f3dd2695ede8b1326e6d495d73b463f3))
* add pb manager debug metrics, handle unknown isolate runner gracefully ([#1711](https://github.com/rivet-gg/rivet/issues/1711)) ([f1634fe](https://github.com/rivet-gg/rivet/commit/f1634fe7dfa6b1bee9513212ac354f709f4627b2))
* add pegboard client metrics ([#1757](https://github.com/rivet-gg/rivet/issues/1757)) ([f5913c2](https://github.com/rivet-gg/rivet/commit/f5913c20a9105c9485e7eed2e2155a096da59447))
* add unique wf dispatch ([#1710](https://github.com/rivet-gg/rivet/issues/1710)) ([8fabc7a](https://github.com/rivet-gg/rivet/commit/8fabc7a89e290e7fe4cb0f8bf34534fe12c4b8ab))
* add wf metrics ([#1695](https://github.com/rivet-gg/rivet/issues/1695)) ([346088b](https://github.com/rivet-gg/rivet/commit/346088b138144b7079cbdf06aa72c34b1b26ac66))
* **dev:** fix codesigning issue on macos arm ([#1774](https://github.com/rivet-gg/rivet/issues/1774)) ([30e4499](https://github.com/rivet-gg/rivet/commit/30e44992f0848a11541b026f26c7c4932c0742c3))
* **docker/dev-full:** update target folder for monolithic workspace ([#1773](https://github.com/rivet-gg/rivet/issues/1773)) ([c2d338e](https://github.com/rivet-gg/rivet/commit/c2d338e208b9a6e31aa01a0b42396c73dc6dc7b9))
* **docker:** update client & monolith to copy client binaries from monorepo ([#1787](https://github.com/rivet-gg/rivet/issues/1787)) ([7b98944](https://github.com/rivet-gg/rivet/commit/7b9894448cd8f24e10e3cafb7ac8387403ddaf10))
* **ds:** allow destroy early during retry backoff ([#1718](https://github.com/rivet-gg/rivet/issues/1718)) ([3c6da10](https://github.com/rivet-gg/rivet/commit/3c6da10af6d46ffedf82c5e7d1ce8ff7007987f8))
* fix actor cleanup ([#1708](https://github.com/rivet-gg/rivet/issues/1708)) ([3e040c7](https://github.com/rivet-gg/rivet/commit/3e040c7d16497914f4f362328c8149e788c95bd7))
* fix eq check for actor kv ([#1752](https://github.com/rivet-gg/rivet/issues/1752)) ([4e09fee](https://github.com/rivet-gg/rivet/commit/4e09fee7307e5ff62e5fe6dfd61677a3ed022700))
* fix pb dc alloc query ([#1709](https://github.com/rivet-gg/rivet/issues/1709)) ([f540dfb](https://github.com/rivet-gg/rivet/commit/f540dfb01ca7fbd3601d5b647be845396aa3c359))
* fix systemd configs ([#1707](https://github.com/rivet-gg/rivet/issues/1707)) ([a380f2e](https://github.com/rivet-gg/rivet/commit/a380f2ed91a874b92c4f8a867d0aa95f7adaa51c))
* fix toolchain tags patch hack ([#1754](https://github.com/rivet-gg/rivet/issues/1754)) ([e003143](https://github.com/rivet-gg/rivet/commit/e00314314d2da51e9ecb01591abd920c5751e5e5))
* fix wf history command ([#1755](https://github.com/rivet-gg/rivet/issues/1755)) ([ea2b099](https://github.com/rivet-gg/rivet/commit/ea2b099a58ea1d2d34d92c107eb0b062649f3f91))
* fix wf version bug with errored activities ([#1706](https://github.com/rivet-gg/rivet/issues/1706)) ([98a76cf](https://github.com/rivet-gg/rivet/commit/98a76cffdbde90e56b516b391710898bb6383443))
* improve scale sorting ([#1717](https://github.com/rivet-gg/rivet/issues/1717)) ([3841840](https://github.com/rivet-gg/rivet/commit/3841840897c9fea3bb85042f3f6c7645e01ba013))
* remove alertmanager ([#1712](https://github.com/rivet-gg/rivet/issues/1712)) ([0ef1c17](https://github.com/rivet-gg/rivet/commit/0ef1c17d1534429282bebbf77dc9848e576ff306))
* remove deno serve listening msg ([#1716](https://github.com/rivet-gg/rivet/issues/1716)) ([bc9d6db](https://github.com/rivet-gg/rivet/commit/bc9d6db32144db6c5300d09aa16a860f2ede0737))
* **sdks/actor/client:** make disconnect async & await for socket close ([#1733](https://github.com/rivet-gg/rivet/issues/1733)) ([d702a98](https://github.com/rivet-gg/rivet/commit/d702a981aaa5cff9e7d3d0e019e391e2e80dfb0b))
* **sdks/actor/runtime:** fix initialize getBatch not reading keys correctly ([#1735](https://github.com/rivet-gg/rivet/issues/1735)) ([0915789](https://github.com/rivet-gg/rivet/commit/0915789f14740bd914f4e32c88fbdddfeabbf6ae))
* **sdks/actor/runtime:** fix throttling code missing final call ([#1737](https://github.com/rivet-gg/rivet/issues/1737)) ([7874e21](https://github.com/rivet-gg/rivet/commit/7874e212fbc496b04725297823c9f4a9ef92ddfc))
* **sdks/actor/runtime:** make actor constructor public ([#1783](https://github.com/rivet-gg/rivet/issues/1783)) ([40f04d3](https://github.com/rivet-gg/rivet/commit/40f04d32eec96578d6fbcd77ecce56501050d593))
* **site:** fix sales layout ([#1697](https://github.com/rivet-gg/rivet/issues/1697)) ([0c8003b](https://github.com/rivet-gg/rivet/commit/0c8003be9c9dcb8095e3e97a42419cc6a73a2426))
* **site:** Remove Landing Animation ([#1781](https://github.com/rivet-gg/rivet/issues/1781)) ([cc5b347](https://github.com/rivet-gg/rivet/commit/cc5b3475d773c7d500ad376c08da46a585462f6b))
* **toolchain:** add back name & access tags to builds ([#1778](https://github.com/rivet-gg/rivet/issues/1778)) ([aa5e0b2](https://github.com/rivet-gg/rivet/commit/aa5e0b2e82f5fc8aee0a85595873213e710d9413))
* **toolchain:** allow deploying tsx & jsx extensions ([#1740](https://github.com/rivet-gg/rivet/issues/1740)) ([e3ab5a5](https://github.com/rivet-gg/rivet/commit/e3ab5a5a796d56c8319f21c11379af49655ebbec))
* **toolchain:** fix js-utils build paths on windows ([#1720](https://github.com/rivet-gg/rivet/issues/1720)) ([fd525f5](https://github.com/rivet-gg/rivet/commit/fd525f5b5431a4bd1dfefe484f20073a3f819494))
* **toolchain:** typo in public access tag ([#1784](https://github.com/rivet-gg/rivet/issues/1784)) ([fdda763](https://github.com/rivet-gg/rivet/commit/fdda7633b9e8bc1aaa2cc749d9fe8ea02097e693))
* use inline videos for bg videos ([#1700](https://github.com/rivet-gg/rivet/issues/1700)) ([1812a22](https://github.com/rivet-gg/rivet/commit/1812a225821bcbb515bb9e42600210ee1f6f83ae))


### Documentation

* add manage & update sdk layout ([#1702](https://github.com/rivet-gg/rivet/issues/1702)) ([400ce31](https://github.com/rivet-gg/rivet/commit/400ce315fc7a31139eebb81b089219a8b8df20da))
* configure max parallel builds for docker compose ([#1772](https://github.com/rivet-gg/rivet/issues/1772)) ([2dc099a](https://github.com/rivet-gg/rivet/commit/2dc099a77de9c57c8539e1d967b3bf8d9a6fec48))
* fix errors in manage docs ([#1704](https://github.com/rivet-gg/rivet/issues/1704)) ([d45bf55](https://github.com/rivet-gg/rivet/commit/d45bf556e903404ab2df053c533d9806071e115d))
* remove broken use cases link ([#1680](https://github.com/rivet-gg/rivet/issues/1680)) ([3f2eefc](https://github.com/rivet-gg/rivet/commit/3f2eefc81f272287f934906a171216d5d02efc15))
* **sdks/actor:** add inline docs for actor runtime & client ([#1751](https://github.com/rivet-gg/rivet/issues/1751)) ([aaec797](https://github.com/rivet-gg/rivet/commit/aaec79724d3e1d885a4acb56196d411afe36f101))
* update bsky link ([2264539](https://github.com/rivet-gg/rivet/commit/2264539de6b022546d357428b0a2da15b181b501))
* update code snippets to latest sdk version ([#1746](https://github.com/rivet-gg/rivet/issues/1746)) ([8e114ca](https://github.com/rivet-gg/rivet/commit/8e114ca337307590ccf4439a9e6eae3b62aaf45d))
* update docker-compose commands to include --build ([#1741](https://github.com/rivet-gg/rivet/issues/1741)) ([e19924a](https://github.com/rivet-gg/rivet/commit/e19924a8d029ae0e9c216ef4d5a82960b13d2955))


### Continuous Integration

* expose token for pulling gh actions in rust builds ([#1738](https://github.com/rivet-gg/rivet/issues/1738)) ([58a1c11](https://github.com/rivet-gg/rivet/commit/58a1c11394eee8785bb819134ec6b71cd5d5b68e))
* move release bin & docker to manual workflow dispatches ([#1764](https://github.com/rivet-gg/rivet/issues/1764)) ([24009d8](https://github.com/rivet-gg/rivet/commit/24009d8bc5e2e2c36f1aeadc5f0f33467440b08c))


### Chores

* "copmile" typo ([#1671](https://github.com/rivet-gg/rivet/issues/1671)) ([e66e875](https://github.com/rivet-gg/rivet/commit/e66e8751ee660125e9bf3c4ac7a88867b71c3408))
* add contributing.md ([#1691](https://github.com/rivet-gg/rivet/issues/1691)) ([fcd03c0](https://github.com/rivet-gg/rivet/commit/fcd03c065fa27d799d3259bb46c429b4a49f0023))
* add conventions to contributing ([#1693](https://github.com/rivet-gg/rivet/issues/1693)) ([5d07b7f](https://github.com/rivet-gg/rivet/commit/5d07b7fbb8013f342a9b97bda851c94780163ec0))
* add infra client to main workspace ([#1753](https://github.com/rivet-gg/rivet/issues/1753)) ([68b99a7](https://github.com/rivet-gg/rivet/commit/68b99a7a6a7f5d1a3f211eff88081e61a039567f))
* add justfile ([#1780](https://github.com/rivet-gg/rivet/issues/1780)) ([62786d5](https://github.com/rivet-gg/rivet/commit/62786d572549150316c34ffae3818e5696c8823c))
* confirm closews works ([#1715](https://github.com/rivet-gg/rivet/issues/1715)) ([ffeff66](https://github.com/rivet-gg/rivet/commit/ffeff66e43db13104978230a70accaed527f77b3))
* fix deploy ([#1769](https://github.com/rivet-gg/rivet/issues/1769)) ([cb7177b](https://github.com/rivet-gg/rivet/commit/cb7177bbcdc5544140a4396652e38a93c5049229))
* fix metrics ([#1771](https://github.com/rivet-gg/rivet/issues/1771)) ([a001a81](https://github.com/rivet-gg/rivet/commit/a001a81db40ab643f1b530e17e9023cefcdcc1e0))
* **sdks/actor/client:** log sent message lengths ([#1736](https://github.com/rivet-gg/rivet/issues/1736)) ([7abe987](https://github.com/rivet-gg/rivet/commit/7abe987027c7e818308898850d439c15b2c936b0))
* **sdks/actor/runtime:** use new getBatch api ([#1782](https://github.com/rivet-gg/rivet/issues/1782)) ([5bef166](https://github.com/rivet-gg/rivet/commit/5bef1663f11e96d5ddec2aefb22a9e1e7715ba1f))
* **sdks/actor:** add helpful links to jsr readmes ([#1748](https://github.com/rivet-gg/rivet/issues/1748)) ([ae7df22](https://github.com/rivet-gg/rivet/commit/ae7df2203528efae6b52e3adf86656f549e88c8e))
* **sdks/actor:** fix generic param to `OnBeforeConnectOptions` to accept an actor ([#1749](https://github.com/rivet-gg/rivet/issues/1749)) ([0b4f6e6](https://github.com/rivet-gg/rivet/commit/0b4f6e6df2df5b0685de4d72be8162aedc72b6f9))
* **site:** add press kit ([#1722](https://github.com/rivet-gg/rivet/issues/1722)) ([05a7553](https://github.com/rivet-gg/rivet/commit/05a7553cebcdbb8561cff09e6256e29900cff73c))
* **system-test:** test kv e2e ([#1779](https://github.com/rivet-gg/rivet/issues/1779)) ([65be0b0](https://github.com/rivet-gg/rivet/commit/65be0b0f80b112bd6644fb732bfa0dcf9ada9af8))
* **toolchain:** update printed dashboard endpoints to use slugs instead of uuids ([#1776](https://github.com/rivet-gg/rivet/issues/1776)) ([dbe0027](https://github.com/rivet-gg/rivet/commit/dbe002797a593e4c48ebe224869ac884f8abec8f))
* update framer (pricing mobile & sales bug) ([#1688](https://github.com/rivet-gg/rivet/issues/1688)) ([a9204f0](https://github.com/rivet-gg/rivet/commit/a9204f0b5cca66d18ea69d7dd664eda12a5be279))
* update version ([72ee696](https://github.com/rivet-gg/rivet/commit/72ee696b6cebbe6485d99ed242f6455f21969d68))

## [24.6.0](https://github.com/rivet-gg/rivet/compare/v24.5.2...v24.6.0) (2024-12-20)


### Features

* actors api ([#1228](https://github.com/rivet-gg/rivet/issues/1228)) ([75bb7e2](https://github.com/rivet-gg/rivet/commit/75bb7e2dbbf904d4c0fb8dcf43d5ef9691f11ca2))
* add .actor. domain for ds ([#1383](https://github.com/rivet-gg/rivet/issues/1383)) ([a990a42](https://github.com/rivet-gg/rivet/commit/a990a429b9b52af7bde14956e157ad589a068eae))
* add @rivet-gg/actors ([#1476](https://github.com/rivet-gg/rivet/issues/1476)) ([f268644](https://github.com/rivet-gg/rivet/commit/f26864433d0eea74bec9b37c32ede88f351e7b8b))
* add actor api ([#1231](https://github.com/rivet-gg/rivet/issues/1231)) ([bf506f6](https://github.com/rivet-gg/rivet/commit/bf506f639084182cc8c4ff3e377bc85fe292e661))
* add actor runtime docs ([#1559](https://github.com/rivet-gg/rivet/issues/1559)) ([3af0f5f](https://github.com/rivet-gg/rivet/commit/3af0f5f66bc06e012ac119e150d8743d87477fcb))
* add ats failover to pb ([#1556](https://github.com/rivet-gg/rivet/issues/1556)) ([4f3d489](https://github.com/rivet-gg/rivet/commit/4f3d489e048340816c3b0f284d883e91c3d1c87a))
* add batch actor upgrade ([#1480](https://github.com/rivet-gg/rivet/issues/1480)) ([c2558d4](https://github.com/rivet-gg/rivet/commit/c2558d49a2b5b7b8a89487c2eb55d1c48aa64f30))
* add check_version to wf ([#1560](https://github.com/rivet-gg/rivet/issues/1560)) ([a3c99f8](https://github.com/rivet-gg/rivet/commit/a3c99f8ce220beaeeb8559cd08069c58fffc852c))
* add creating actor by build tags, upgrading actors ([#1388](https://github.com/rivet-gg/rivet/issues/1388)) ([b1fc1aa](https://github.com/rivet-gg/rivet/commit/b1fc1aa17df018c072ead3ae9dc93c8f66006fb2))
* add durability to ds ([#1364](https://github.com/rivet-gg/rivet/issues/1364)) ([293be3d](https://github.com/rivet-gg/rivet/commit/293be3da4ecd1e9f8b420bea707b309e95d7bd6b))
* add endpoint to fetch fdb ips ([#1355](https://github.com/rivet-gg/rivet/issues/1355)) ([a99f713](https://github.com/rivet-gg/rivet/commit/a99f71382684d08c57ba081a71a54b21f6c0deb7))
* add fdb driver ([#1326](https://github.com/rivet-gg/rivet/issues/1326)) ([896f77a](https://github.com/rivet-gg/rivet/commit/896f77a76b9ac489c8ef1e0d6a81af31321b95af))
* add fdb pool ([#1304](https://github.com/rivet-gg/rivet/issues/1304)) ([3ee013b](https://github.com/rivet-gg/rivet/commit/3ee013b619ea6763cb2f61f3ec5dd195b408972b))
* add internal retryability to pegboard manager ([#1300](https://github.com/rivet-gg/rivet/issues/1300)) ([1845baf](https://github.com/rivet-gg/rivet/commit/1845baf914ce47fc7bc77574fe30ba3d09075503))
* add kv get ext, fix runtime termination ([#1324](https://github.com/rivet-gg/rivet/issues/1324)) ([3179e94](https://github.com/rivet-gg/rivet/commit/3179e9492979ffec3532f8422566f1ccfde8dc28))
* add kv limits to docs ([#1558](https://github.com/rivet-gg/rivet/issues/1558)) ([dbe463c](https://github.com/rivet-gg/rivet/commit/dbe463c30d792de63897c3617ffe905687e478f2))
* add metadata to actor env ([#1482](https://github.com/rivet-gg/rivet/issues/1482)) ([113d334](https://github.com/rivet-gg/rivet/commit/113d33479dc5915151bbcd321dee60e8abd94459))
* add pegboard isolate pooltype ([#1223](https://github.com/rivet-gg/rivet/issues/1223)) ([3e685b3](https://github.com/rivet-gg/rivet/commit/3e685b3752ebd3b8285d2bb176bda40642b8474c))
* add put and delete ops to kv ([#1325](https://github.com/rivet-gg/rivet/issues/1325)) ([bc65b95](https://github.com/rivet-gg/rivet/commit/bc65b95d378d839466664c9fff547c8516c0cc22))
* add region proxying to actor client ([#1540](https://github.com/rivet-gg/rivet/issues/1540)) ([355c245](https://github.com/rivet-gg/rivet/commit/355c245b0a710ab6118f32c71f0e876d31d9b5eb))
* automatically recommend region for actor creation ([#1471](https://github.com/rivet-gg/rivet/issues/1471)) ([9757b15](https://github.com/rivet-gg/rivet/commit/9757b1595982125fb5b80f1f8f7fc6beb3bb70ea))
* **clusters:** add pegboard pool ([#1153](https://github.com/rivet-gg/rivet/issues/1153)) ([bffa76f](https://github.com/rivet-gg/rivet/commit/bffa76fa8241816911b2d8741071cd498adc0139))
* **clusters:** add pegboard pool type ([#1152](https://github.com/rivet-gg/rivet/issues/1152)) ([b4a1d16](https://github.com/rivet-gg/rivet/commit/b4a1d1681e392865e6641e5cb1d6ca7408ba46e1))
* config docs ([#1499](https://github.com/rivet-gg/rivet/issues/1499)) ([9485a51](https://github.com/rivet-gg/rivet/commit/9485a510e602f3458ca76ce294829db79e2e5b38))
* ds input validation ([#1411](https://github.com/rivet-gg/rivet/issues/1411)) ([fa26199](https://github.com/rivet-gg/rivet/commit/fa2619941520aa39e7132fcd4cc9ccfafb9cb8d1))
* **ds:** implement pegboard dynamic server ([#1158](https://github.com/rivet-gg/rivet/issues/1158)) ([149de29](https://github.com/rivet-gg/rivet/commit/149de292ca8a238e77b3c849c4b8416a940a7a68))
* **ds:** shard server workflow in two ([#1157](https://github.com/rivet-gg/rivet/issues/1157)) ([8329433](https://github.com/rivet-gg/rivet/commit/83294332c45bb71512d3743bb4ab373b329a38e9))
* extend tuple system to js ([#1349](https://github.com/rivet-gg/rivet/issues/1349)) ([94ac482](https://github.com/rivet-gg/rivet/commit/94ac482bfaf36fa311827a2a945667e1b87ea340))
* fancy subnav ([#1537](https://github.com/rivet-gg/rivet/issues/1537)) ([e6bb55d](https://github.com/rivet-gg/rivet/commit/e6bb55d87251b0875ed1ea938335118f0cfd012f))
* implement topology for all pool types ([#1225](https://github.com/rivet-gg/rivet/issues/1225)) ([f659729](https://github.com/rivet-gg/rivet/commit/f6597290b1eaa4ebccda7c26e53b642465408b9c))
* make actors reschedule indefinitely with backoff ([#1475](https://github.com/rivet-gg/rivet/issues/1475)) ([04ec2bd](https://github.com/rivet-gg/rivet/commit/04ec2bd7c38fd3069e744802ff65a88906bc5314))
* **pegboard:** add client wf ([#1143](https://github.com/rivet-gg/rivet/issues/1143)) ([1589118](https://github.com/rivet-gg/rivet/commit/1589118e578ec617b38971ad1abbf42f5d13e5c7))
* **pegboard:** add container runner and manager ([#1144](https://github.com/rivet-gg/rivet/issues/1144)) ([799c059](https://github.com/rivet-gg/rivet/commit/799c0592e44c7568fa2cb2e9b4c7261aa8e1253c))
* **pegboard:** add dc wf, refactor tier, move stuff around ([#1159](https://github.com/rivet-gg/rivet/issues/1159)) ([6d7416e](https://github.com/rivet-gg/rivet/commit/6d7416e3d845a4492c483d9710387b8bb953e6f6))
* **pegboard:** add gc service ([#1160](https://github.com/rivet-gg/rivet/issues/1160)) ([34b391f](https://github.com/rivet-gg/rivet/commit/34b391fb6e385fe85eef57164034fe5f7444cbf6))
* **pegboard:** add js builds ([#1212](https://github.com/rivet-gg/rivet/issues/1212)) ([f2d04d1](https://github.com/rivet-gg/rivet/commit/f2d04d1ef6a7f7f494b2120431d8114b07c7d28e))
* **pegboard:** add logs from edge ([#1255](https://github.com/rivet-gg/rivet/issues/1255)) ([9d304d1](https://github.com/rivet-gg/rivet/commit/9d304d1dcf14b624346a9301288657abdb696d4a))
* **pegboard:** add process monitoring, wip sqlite ([#1146](https://github.com/rivet-gg/rivet/issues/1146)) ([3019a65](https://github.com/rivet-gg/rivet/commit/3019a651526ba93e695b3ed398bec286205bb59a))
* **pegboard:** add ws tunnel ([#1145](https://github.com/rivet-gg/rivet/issues/1145)) ([5f0085d](https://github.com/rivet-gg/rivet/commit/5f0085d484b7f1f2211de015ab1d20e21ccd7cce))
* **pegboard:** create robust testing system ([#1171](https://github.com/rivet-gg/rivet/issues/1171)) ([2b26b6a](https://github.com/rivet-gg/rivet/commit/2b26b6ab47ac7a4ea1fec099089d5b34e9c1e02d))
* **pegboard:** get container connection e2e ([#1165](https://github.com/rivet-gg/rivet/issues/1165)) ([7b93bec](https://github.com/rivet-gg/rivet/commit/7b93bec0f6689db13cc656341f9580f0a8906f9e))
* **pegboard:** implement v8 isolate runner ([#1213](https://github.com/rivet-gg/rivet/issues/1213)) ([dc4d97e](https://github.com/rivet-gg/rivet/commit/dc4d97e5dd17c77be2d364e4268c50f8e361b9e0))
* **pegboard:** implement ws wf ([#1141](https://github.com/rivet-gg/rivet/issues/1141)) ([3cfbfb5](https://github.com/rivet-gg/rivet/commit/3cfbfb55505c9a06d92d8480a8de39f0cfd4a3c4))
* **pegboard:** integrate setup scripts ([#1147](https://github.com/rivet-gg/rivet/issues/1147)) ([cd85660](https://github.com/rivet-gg/rivet/commit/cd85660e27225c1ab72126f4cd1df9de4b0964be))
* **pegboard:** make protocol more robust, implement db sync ([#1148](https://github.com/rivet-gg/rivet/issues/1148)) ([166991e](https://github.com/rivet-gg/rivet/commit/166991e4613b0516c1fe9378d09f594e528cf449))
* port auth for ds ([#1294](https://github.com/rivet-gg/rivet/issues/1294)) ([f42c814](https://github.com/rivet-gg/rivet/commit/f42c8140e96429b4472a542ef6ab9b5f8439a327))
* **workflows:** add workflow migrations ([#1203](https://github.com/rivet-gg/rivet/issues/1203)) ([763036a](https://github.com/rivet-gg/rivet/commit/763036a5758e68618f52f64b626d601b603d8495))
* **workflows:** implement listen with timeout ([#1156](https://github.com/rivet-gg/rivet/issues/1156)) ([4ed45c4](https://github.com/rivet-gg/rivet/commit/4ed45c4947f8835c30a746a95558e7f0cf28d2b3))


### Bug Fixes

* add back api docs ([#1549](https://github.com/rivet-gg/rivet/issues/1549)) ([fa6d78b](https://github.com/rivet-gg/rivet/commit/fa6d78b28eaa7cd753854ba17a851b5869e5c07d))
* add cluster description to pegboard client config ([#1461](https://github.com/rivet-gg/rivet/issues/1461)) ([ddb8a99](https://github.com/rivet-gg/rivet/commit/ddb8a993d47677f49cd4975f277575b4c233ff40))
* add disk limit to pb actors, fix dockerfiles ([#1469](https://github.com/rivet-gg/rivet/issues/1469)) ([e020054](https://github.com/rivet-gg/rivet/commit/e020054d9c9190ac2e6d152095f82c2deadb20e9))
* add fdb to shell nix ([#1391](https://github.com/rivet-gg/rivet/issues/1391)) ([bc26c62](https://github.com/rivet-gg/rivet/commit/bc26c6271e93dc8283e95194c65088735bad49de))
* add hosts file to runc ([#1386](https://github.com/rivet-gg/rivet/issues/1386)) ([76089bb](https://github.com/rivet-gg/rivet/commit/76089bba6fa692442daccd67a70d4ac9df35d766))
* add kv op docs ([#1551](https://github.com/rivet-gg/rivet/issues/1551)) ([869fdfc](https://github.com/rivet-gg/rivet/commit/869fdfca5a9298dae437908a649a73e79d2d1192))
* add lost state for pb actors ([#1378](https://github.com/rivet-gg/rivet/issues/1378)) ([8baec56](https://github.com/rivet-gg/rivet/commit/8baec56a164d8abc407d0570405d9be659fa0a1c))
* add opt auth to actors ([#1321](https://github.com/rivet-gg/rivet/issues/1321)) ([0069899](https://github.com/rivet-gg/rivet/commit/006989998ba09ca9e6229a1051b3af72ba0367a9))
* add protoc to shell.nix ([#1463](https://github.com/rivet-gg/rivet/issues/1463)) ([76dad38](https://github.com/rivet-gg/rivet/commit/76dad38c2eb7dde631b538c4f59f7be27d8f88d6))
* add shell nix ([#1382](https://github.com/rivet-gg/rivet/issues/1382)) ([eaee5a5](https://github.com/rivet-gg/rivet/commit/eaee5a54741843a6630262c9aab5a455585577a9))
* add shutdown method to actors ([#1578](https://github.com/rivet-gg/rivet/issues/1578)) ([91e2aad](https://github.com/rivet-gg/rivet/commit/91e2aade9576169df12803a803f5b043fd73e235))
* add toggle for actor fs mounts ([#1478](https://github.com/rivet-gg/rivet/issues/1478)) ([dea5633](https://github.com/rivet-gg/rivet/commit/dea563372af6502a6d91f4a4b81871c743b8d181))
* add toolchain schema to docs ([#1527](https://github.com/rivet-gg/rivet/issues/1527)) ([ce7f63c](https://github.com/rivet-gg/rivet/commit/ce7f63ced687663a8e2eb65680a806449e5bf5cf))
* allow using service tokens for actor endpoints ([#1495](https://github.com/rivet-gg/rivet/issues/1495)) ([eee4dea](https://github.com/rivet-gg/rivet/commit/eee4dea438d85c56061f54b5da3cb9af8aec262e))
* artifact dockerfile, tokio versions ([#1232](https://github.com/rivet-gg/rivet/issues/1232)) ([d5e44c5](https://github.com/rivet-gg/rivet/commit/d5e44c56be9d2aa2b5c8184b6598300f5dcd4980))
* **bolt:** fix wf commands sql connection on distributed clusters, add errors to history command ([#1197](https://github.com/rivet-gg/rivet/issues/1197)) ([0ef9611](https://github.com/rivet-gg/rivet/commit/0ef961143e6ec988db381f286e4991b418239bca))
* **bolt:** require --all flag when not using filters for cluster commands ([#1162](https://github.com/rivet-gg/rivet/issues/1162)) ([b140971](https://github.com/rivet-gg/rivet/commit/b140971460f13185567c8a4151f9390080e57c10))
* **bolt:** wait for service monitor crd ([#1202](https://github.com/rivet-gg/rivet/issues/1202)) ([17cfd73](https://github.com/rivet-gg/rivet/commit/17cfd7333e71860da320cb6f2147a67eae6511dc))
* change ints to timestamps in actor api ([#1483](https://github.com/rivet-gg/rivet/issues/1483)) ([0bd4953](https://github.com/rivet-gg/rivet/commit/0bd4953275cc63116ca2b747c798a3cfadd357e2))
* change wf metrics ([#1632](https://github.com/rivet-gg/rivet/issues/1632)) ([20c4a1d](https://github.com/rivet-gg/rivet/commit/20c4a1d93aa623d33cf047f11a633a1fd17bc606))
* **cluster:** use s3 endpoint without trailing slash for s3 region map ([#1654](https://github.com/rivet-gg/rivet/issues/1654)) ([d610476](https://github.com/rivet-gg/rivet/commit/d6104764c04474d5705a34aec31697a59d686b2d))
* conditionally remove cluster & ds code dependent on external providers ([#1328](https://github.com/rivet-gg/rivet/issues/1328)) ([fdae5a4](https://github.com/rivet-gg/rivet/commit/fdae5a4a83d28bf955643e57e70d772a621f8ac5))
* configure sqlite defaults, add txn for events ([#1379](https://github.com/rivet-gg/rivet/issues/1379)) ([78877a6](https://github.com/rivet-gg/rivet/commit/78877a67d9cc64ac18ae54512efbbe4fde9c2e96))
* configure systemd priorities ([#1384](https://github.com/rivet-gg/rivet/issues/1384)) ([7639d46](https://github.com/rivet-gg/rivet/commit/7639d460d39475587261d213336d4befde70138b))
* correct actor-core symlink ([#1481](https://github.com/rivet-gg/rivet/issues/1481)) ([b63ae81](https://github.com/rivet-gg/rivet/commit/b63ae81be481dbbc95f13d2e75b5ff58f6dbef94))
* correctly substitute VITE_APP_API_URL ([#1320](https://github.com/rivet-gg/rivet/issues/1320)) ([3e68aee](https://github.com/rivet-gg/rivet/commit/3e68aee6697fb9d01b71603ba799682335336b92))
* create manager actor with bridge & https ([#1621](https://github.com/rivet-gg/rivet/issues/1621)) ([8c53cc7](https://github.com/rivet-gg/rivet/commit/8c53cc767a75a4bd580c7bc4fe0c1b9084cdd3b4))
* default resources for isolates ([#1496](https://github.com/rivet-gg/rivet/issues/1496)) ([1d5652c](https://github.com/rivet-gg/rivet/commit/1d5652c2d3158c6c8d8103a45a5ec5884416d2ba))
* disable eval for isolates ([#1404](https://github.com/rivet-gg/rivet/issues/1404)) ([959002e](https://github.com/rivet-gg/rivet/commit/959002ea7c8ad49548c39f99432bc137e9746d55))
* dont return bool from kv delete ([#1434](https://github.com/rivet-gg/rivet/issues/1434)) ([b67e5dc](https://github.com/rivet-gg/rivet/commit/b67e5dc612d2bacf6c07d9542a6c5129645567a3))
* enable host networking and root users on dev ([#1387](https://github.com/rivet-gg/rivet/issues/1387)) ([a8ef10e](https://github.com/rivet-gg/rivet/commit/a8ef10e91474253501e56f8372232c6a30a1823d))
* enable opt_auth for regions & enable service tokens for upgrading actors ([#1618](https://github.com/rivet-gg/rivet/issues/1618)) ([12ecaf3](https://github.com/rivet-gg/rivet/commit/12ecaf3558e1cda244af7e75446acc95e2e873f6))
* fix api url parsing, pegboard install ([#1240](https://github.com/rivet-gg/rivet/issues/1240)) ([b420f2e](https://github.com/rivet-gg/rivet/commit/b420f2e1c709673274cefff22ab77d27aaf0799f))
* fix backwards compat issue with bootstrap ([#1614](https://github.com/rivet-gg/rivet/issues/1614)) ([33ede46](https://github.com/rivet-gg/rivet/commit/33ede461d1d2eb1cc7a1c32273f813990be6ed0e))
* fix build validation ([#1448](https://github.com/rivet-gg/rivet/issues/1448)) ([c647e41](https://github.com/rivet-gg/rivet/commit/c647e41fe5e7c3967f8bacf1e498d86daedd8104))
* fix cargo tomls ([#1428](https://github.com/rivet-gg/rivet/issues/1428)) ([d8be291](https://github.com/rivet-gg/rivet/commit/d8be291372b22a5aff76bd2cc511ef70891890bf))
* fix clickhouse health check ([#1395](https://github.com/rivet-gg/rivet/issues/1395)) ([725c794](https://github.com/rivet-gg/rivet/commit/725c794080fd94774b5550e522f9e7044ec71445))
* fix compile errors in manager ([#1291](https://github.com/rivet-gg/rivet/issues/1291)) ([062a85b](https://github.com/rivet-gg/rivet/commit/062a85b68044bbc832d5806f07e0d5ace2310b7f))
* fix dc topo error, actor backoff ([#1617](https://github.com/rivet-gg/rivet/issues/1617)) ([7be4871](https://github.com/rivet-gg/rivet/commit/7be48716169b3076bbd4db6d0284d05120f9a31b))
* fix dynamic config query ([#1276](https://github.com/rivet-gg/rivet/issues/1276)) ([96155e2](https://github.com/rivet-gg/rivet/commit/96155e2d504d4f20ffbb8c68d7e4a5f0fe9b4f3e))
* fix event sending race condition for pb manager ([#1455](https://github.com/rivet-gg/rivet/issues/1455)) ([0e2642a](https://github.com/rivet-gg/rivet/commit/0e2642aef3ae5eb48717c81b2759bfa5090ad1bd))
* fix fdb mount in docker compose ([#1573](https://github.com/rivet-gg/rivet/issues/1573)) ([e5301c1](https://github.com/rivet-gg/rivet/commit/e5301c1e7c46a3ee60f5e87e90932b37cdf3d84d))
* fix global query deserialize ([#1313](https://github.com/rivet-gg/rivet/issues/1313)) ([f45c1d0](https://github.com/rivet-gg/rivet/commit/f45c1d07092391a66dcf40b4b18c2d583ac6e307))
* fix indexes for wf driver ([#1226](https://github.com/rivet-gg/rivet/issues/1226)) ([46389aa](https://github.com/rivet-gg/rivet/commit/46389aa6d543394477f76b49b2f7bed90bdcb730))
* fix infra artifacts dockerfiles, wf commands ([#1252](https://github.com/rivet-gg/rivet/issues/1252)) ([dcf5b03](https://github.com/rivet-gg/rivet/commit/dcf5b031c5e41e1b0bb94422da7ac33968076fa6))
* fix js glue ([#1477](https://github.com/rivet-gg/rivet/issues/1477)) ([496d9dc](https://github.com/rivet-gg/rivet/commit/496d9dca5fe16f67b6806a9780af6fc0655277fc))
* fix legacy bolt get ([#1193](https://github.com/rivet-gg/rivet/issues/1193)) ([68285cb](https://github.com/rivet-gg/rivet/commit/68285cb271b0fabd49ddb8bad9f75aa8fcd3faff))
* fix merge ([#1380](https://github.com/rivet-gg/rivet/issues/1380)) ([93710fc](https://github.com/rivet-gg/rivet/commit/93710fc7c0c194b712af40327b54364d4b350280))
* fix monolith welcome message ([#1413](https://github.com/rivet-gg/rivet/issues/1413)) ([4eedc91](https://github.com/rivet-gg/rivet/commit/4eedc91a8ee8a8817c1e867414185bef329d0f21))
* fix multiple patching exclusive tags ([#1498](https://github.com/rivet-gg/rivet/issues/1498)) ([62d5dc9](https://github.com/rivet-gg/rivet/commit/62d5dc96f0391e5d2c654a7afe19eca756918f77))
* fix patch build tags ([#1486](https://github.com/rivet-gg/rivet/issues/1486)) ([8482e28](https://github.com/rivet-gg/rivet/commit/8482e283cd8fef783a055e85aa1cddbf40e6baac))
* fix path to deno config for manager ([#1593](https://github.com/rivet-gg/rivet/issues/1593)) ([d3ec98e](https://github.com/rivet-gg/rivet/commit/d3ec98ef1e0f025706a627dd199b0e7b98a57b0f))
* fix pb manager receive error ([#1323](https://github.com/rivet-gg/rivet/issues/1323)) ([02d73b6](https://github.com/rivet-gg/rivet/commit/02d73b62ad5ddd2fbeb5cdc2c4f5c4819ed4fd4c))
* fix pegboard gc ([#1637](https://github.com/rivet-gg/rivet/issues/1637)) ([788fa95](https://github.com/rivet-gg/rivet/commit/788fa95fc250bae77a0dffd5fa6a126c6c096cb2))
* fix pegboard tests ([#1297](https://github.com/rivet-gg/rivet/issues/1297)) ([a9a1983](https://github.com/rivet-gg/rivet/commit/a9a1983608c8e748c500398c1d9ee2a96ff2cc2a))
* fix port auth sql ([#1296](https://github.com/rivet-gg/rivet/issues/1296)) ([688f76a](https://github.com/rivet-gg/rivet/commit/688f76a7b84999a2db579500b330f82d274c2c5c))
* fix telemetry queries ([#1277](https://github.com/rivet-gg/rivet/issues/1277)) ([a2d5120](https://github.com/rivet-gg/rivet/commit/a2d51205a5c3c54fb0d3efc5ffb6713ac5cb6f7e))
* fix ts generator ([#1545](https://github.com/rivet-gg/rivet/issues/1545)) ([ed768ab](https://github.com/rivet-gg/rivet/commit/ed768ab71958bc3c839addf59202b96c7357b1fe))
* fix ts types for actor bindings ([#1470](https://github.com/rivet-gg/rivet/issues/1470)) ([6ca6764](https://github.com/rivet-gg/rivet/commit/6ca6764f638e068de32be4e86cca9e6ea23fd6db))
* fix tunnels ([#1227](https://github.com/rivet-gg/rivet/issues/1227)) ([30fda93](https://github.com/rivet-gg/rivet/commit/30fda93bac067e78bfce9832bbaf3a712d8e898d))
* fix type definitions for rpcs ([#1535](https://github.com/rivet-gg/rivet/issues/1535)) ([c95ecc3](https://github.com/rivet-gg/rivet/commit/c95ecc3a6dadae3ca8b5f0c9941f541753a9a029))
* fix url version conflicts with deno ([#1234](https://github.com/rivet-gg/rivet/issues/1234)) ([002024d](https://github.com/rivet-gg/rivet/commit/002024dedabb6db33e91521e2b82e908e686a646))
* fix various bugs ([#1356](https://github.com/rivet-gg/rivet/issues/1356)) ([61efc1a](https://github.com/rivet-gg/rivet/commit/61efc1aa9d8a5148344ba9065a48215d7bc2c902))
* fmt and downgrade tokio ([#1222](https://github.com/rivet-gg/rivet/issues/1222)) ([2f9ae5b](https://github.com/rivet-gg/rivet/commit/2f9ae5bc0fe756025e9c3de859ad4d7ef0e30da3))
* get pegboard -&gt; fdb connection over vlan ([#1362](https://github.com/rivet-gg/rivet/issues/1362)) ([12f01f7](https://github.com/rivet-gg/rivet/commit/12f01f700b0c18742e5c4eab0931443eadd6eda6))
* get pegboard working e2e ([#1253](https://github.com/rivet-gg/rivet/issues/1253)) ([858a88b](https://github.com/rivet-gg/rivet/commit/858a88b0c625c44751d5953e60a67ef1ab827f88))
* implement host networking for ds ([#1273](https://github.com/rivet-gg/rivet/issues/1273)) ([2c1a05a](https://github.com/rivet-gg/rivet/commit/2c1a05ad938eb165c90480e9bec051021da6930f))
* improve isolate error handling ([#1451](https://github.com/rivet-gg/rivet/issues/1451)) ([47d01df](https://github.com/rivet-gg/rivet/commit/47d01df3f0714c040afb9f0c1b1621d0e9e37acf))
* incorrect use of `Metrics` in `rivet.health` config ([#1269](https://github.com/rivet-gg/rivet/issues/1269)) ([aa29bd4](https://github.com/rivet-gg/rivet/commit/aa29bd4bee89cfe9655d3a1c917f2b8f8c58d85b))
* js bundle unarchiving ([#1456](https://github.com/rivet-gg/rivet/issues/1456)) ([9a89891](https://github.com/rivet-gg/rivet/commit/9a898917759a55242c9674ed2ffe3d7f1b722634))
* latent history bug ([#1636](https://github.com/rivet-gg/rivet/issues/1636)) ([f2c58c9](https://github.com/rivet-gg/rivet/commit/f2c58c9e8f06c268e77c436dd0ce2861df0cfb56))
* listen for destroy on actor create endpoint ([#1466](https://github.com/rivet-gg/rivet/issues/1466)) ([bcc5852](https://github.com/rivet-gg/rivet/commit/bcc5852aca2f105bdabb46aef1072d14889b921e))
* lower upload limit for js builds ([#1472](https://github.com/rivet-gg/rivet/issues/1472)) ([2ffc65b](https://github.com/rivet-gg/rivet/commit/2ffc65b9a97018f55c6b618dac86cf1400e4bd40))
* make isolate runtime local ([#1517](https://github.com/rivet-gg/rivet/issues/1517)) ([8a8b8a2](https://github.com/rivet-gg/rivet/commit/8a8b8a24b9649bf38878df9a3163bae53f7a0d3d))
* make kv metadata version to bytes ([#1435](https://github.com/rivet-gg/rivet/issues/1435)) ([44d9f7c](https://github.com/rivet-gg/rivet/commit/44d9f7c8de4bdaddaea6a233400c93df12c271e4))
* make pull addresses optional ([#1629](https://github.com/rivet-gg/rivet/issues/1629)) ([1db7070](https://github.com/rivet-gg/rivet/commit/1db7070a5dadb6264848b29527887ac3dc2dc6db))
* **mm:** serialize RIVET_API_ENDPOINT without trailing slash ([#1645](https://github.com/rivet-gg/rivet/issues/1645)) ([29e1642](https://github.com/rivet-gg/rivet/commit/29e16420c861075a046ffc3081363b3c40ecc9f6))
* move cluster config to rivet config ([#1295](https://github.com/rivet-gg/rivet/issues/1295)) ([472c54f](https://github.com/rivet-gg/rivet/commit/472c54f79a9ba826a46a16bcc5f29b39b7d2a84f))
* move default builds to tf, bump s3 sdk ([#1237](https://github.com/rivet-gg/rivet/issues/1237)) ([0194031](https://github.com/rivet-gg/rivet/commit/01940317cc79d4aa2b566002eaf47a87595f03ca))
* move hosts file into oci bundle ([#1433](https://github.com/rivet-gg/rivet/issues/1433)) ([8c45cbd](https://github.com/rivet-gg/rivet/commit/8c45cbd2edccafe45183689108f9e2b1504257e1))
* nomad url templating ([#1257](https://github.com/rivet-gg/rivet/issues/1257)) ([80201f8](https://github.com/rivet-gg/rivet/commit/80201f81f3eb019fd8fef8060bc54b04ce98e37a))
* **pegboard:** add dc id to pb client ([#1163](https://github.com/rivet-gg/rivet/issues/1163)) ([6be42ba](https://github.com/rivet-gg/rivet/commit/6be42baba4c8826a020810ba625cd75c3b0684c7))
* **pegboard:** automatically disconnect clients with complete pegboard workflows ([#1652](https://github.com/rivet-gg/rivet/issues/1652)) ([821b9d1](https://github.com/rivet-gg/rivet/commit/821b9d13d9723c8951a4acaa2307ff546ba5176b))
* **pegboard:** automatically remove actor directory on cleanup ([#1651](https://github.com/rivet-gg/rivet/issues/1651)) ([87ee685](https://github.com/rivet-gg/rivet/commit/87ee685049e3775f1dd1696f9d1e46a4ffbe7342))
* **pegboard:** fix clients connected without active workflow ([#1655](https://github.com/rivet-gg/rivet/issues/1655)) ([b08ce51](https://github.com/rivet-gg/rivet/commit/b08ce5175150226fc5f3b423ace6d87e5ef2ee74))
* **pegboard:** fix load balancing for isolates ([#1648](https://github.com/rivet-gg/rivet/issues/1648)) ([9d60d5a](https://github.com/rivet-gg/rivet/commit/9d60d5a1dce85ee4bde827c9e19305c57cb198f1))
* **pegboard:** fix migrations ([#1230](https://github.com/rivet-gg/rivet/issues/1230)) ([3f10f74](https://github.com/rivet-gg/rivet/commit/3f10f74f04c29ca6792971bb4d06e7fee0a21cfd))
* **pegboard:** get pb running on edge ([#1154](https://github.com/rivet-gg/rivet/issues/1154)) ([f0c1a18](https://github.com/rivet-gg/rivet/commit/f0c1a189432a699ec08e0bb8c503dd9e6bdd64d1))
* **pegboard:** handle conflicts for client events gracefully ([#1647](https://github.com/rivet-gg/rivet/issues/1647)) ([fc13e83](https://github.com/rivet-gg/rivet/commit/fc13e8393c84ddf73468d8b1ea10596126d02a36))
* **pegboard:** implement cleanup, rebuild, fix queries ([#1150](https://github.com/rivet-gg/rivet/issues/1150)) ([2bc957b](https://github.com/rivet-gg/rivet/commit/2bc957b84dbc66cb144e7275ebf81080155ee5bb))
* **pegboard:** implement port choosing ([#1155](https://github.com/rivet-gg/rivet/issues/1155)) ([b0825e1](https://github.com/rivet-gg/rivet/commit/b0825e1655dfd1e5591ddf5d0f3d6d467c360687))
* rebuild isolate runner handle before starting runner socket ([#1376](https://github.com/rivet-gg/rivet/issues/1376)) ([cdda797](https://github.com/rivet-gg/rivet/commit/cdda797bd7401f0a6832185ff6d3793ac6893013))
* remove actor args from api ([#1450](https://github.com/rivet-gg/rivet/issues/1450)) ([f4e2f42](https://github.com/rivet-gg/rivet/commit/f4e2f426123a3f2c4815c9285fc4a977f893d92d))
* remove cgroups from pb manager ([#1385](https://github.com/rivet-gg/rivet/issues/1385)) ([57d6d66](https://github.com/rivet-gg/rivet/commit/57d6d66040d42f59debaf3c78cefe9d67d4efd4b))
* remove chirp-workflow as a downstream dependency for pegboard-manager ([#1293](https://github.com/rivet-gg/rivet/issues/1293)) ([8ede99b](https://github.com/rivet-gg/rivet/commit/8ede99b8444ad47e7d559170724d3b4c8ea14c4a))
* remove multipart flag ([#1431](https://github.com/rivet-gg/rivet/issues/1431)) ([d927f15](https://github.com/rivet-gg/rivet/commit/d927f1500265b6f6422525a69db5e9cabed27011))
* remove non-deterministic fern check ([#1415](https://github.com/rivet-gg/rivet/issues/1415)) ([2fbecba](https://github.com/rivet-gg/rivet/commit/2fbecba45bf15e50edbb8959f27ddb2f921b2f0c))
* rename ports vars to uppercase ([#1485](https://github.com/rivet-gg/rivet/issues/1485)) ([d59dd0e](https://github.com/rivet-gg/rivet/commit/d59dd0e554dbf92bb740a7356000388f2f738372))
* rename RivetClient back to Rivet ([#1536](https://github.com/rivet-gg/rivet/issues/1536)) ([2dff9e5](https://github.com/rivet-gg/rivet/commit/2dff9e56f3a884f3c696ef72dac7dd8cb9396ec9))
* resolve actor create build tags to the env instead of game ([#1430](https://github.com/rivet-gg/rivet/issues/1430)) ([ad50f4a](https://github.com/rivet-gg/rivet/commit/ad50f4af2c8cacef58b4f778bc908cfc4445ae98))
* restructure pegboard install ([#1221](https://github.com/rivet-gg/rivet/issues/1221)) ([a26418e](https://github.com/rivet-gg/rivet/commit/a26418e4141e9949d53d4243b8c852dfc8bd2f80))
* return early if actor already destroyed ([#1497](https://github.com/rivet-gg/rivet/issues/1497)) ([4ac0a4c](https://github.com/rivet-gg/rivet/commit/4ac0a4cf442fc1f0cfe42a999c76a216413e2332))
* rework failing for ds wf ([#1449](https://github.com/rivet-gg/rivet/issues/1449)) ([4614742](https://github.com/rivet-gg/rivet/commit/4614742a0f954f9b753304cae1d67d4ce589d2a9))
* run vector in container ([#1302](https://github.com/rivet-gg/rivet/issues/1302)) ([aba8fb4](https://github.com/rivet-gg/rivet/commit/aba8fb401d147f66ebde67e008b2d8c64226ffc7))
* **sdk:** fix auto-building manager ([#1677](https://github.com/rivet-gg/rivet/issues/1677)) ([ed28aa3](https://github.com/rivet-gg/rivet/commit/ed28aa39951467b0069bfc452ee3144d432a32ad))
* standardize presigned requests output for actor builds, build tags on create ([#1397](https://github.com/rivet-gg/rivet/issues/1397)) ([cb012d7](https://github.com/rivet-gg/rivet/commit/cb012d7fbc85fdde58e6ceceffa5a9159c2b4b1f))
* temp disable port auth ([#1577](https://github.com/rivet-gg/rivet/issues/1577)) ([23aca8c](https://github.com/rivet-gg/rivet/commit/23aca8ce762c8203f754b5c44c119722a05eb5bd))
* toggle secure for clickhouse migration ([#1630](https://github.com/rivet-gg/rivet/issues/1630)) ([b301a52](https://github.com/rivet-gg/rivet/commit/b301a52cc19071b8cfb629da1cd1d000c2e143ab))
* update architecture diagram ([#1521](https://github.com/rivet-gg/rivet/issues/1521)) ([965f734](https://github.com/rivet-gg/rivet/commit/965f734cb34e4ab4b47fe785587cdd9328bcfd61))
* update cluster paths for rivet-client ([#1315](https://github.com/rivet-gg/rivet/issues/1315)) ([f2ec038](https://github.com/rivet-gg/rivet/commit/f2ec03869a632b916162514ed50d9dfc98f9e438))
* update default hub origin to have /ui/ path ([#1464](https://github.com/rivet-gg/rivet/issues/1464)) ([0e53a0e](https://github.com/rivet-gg/rivet/commit/0e53a0eada9fe427bdc37ebaa1cb8541720b74e0))
* update ds and builds limits ([#1579](https://github.com/rivet-gg/rivet/issues/1579)) ([fbdaa21](https://github.com/rivet-gg/rivet/commit/fbdaa21b01b317661d2f01db4116a44c99e68a74))
* update ds state when failed to allocate ([#1375](https://github.com/rivet-gg/rivet/issues/1375)) ([2acaf82](https://github.com/rivet-gg/rivet/commit/2acaf8223692e080178b73ccae76ac455f36d048))
* update gitignores to include externals ([#1394](https://github.com/rivet-gg/rivet/issues/1394)) ([f61b800](https://github.com/rivet-gg/rivet/commit/f61b800c99714999eacdbc9dbb0242bd6b8cd8dd))
* update runtime audiences ([#1393](https://github.com/rivet-gg/rivet/issues/1393)) ([9457bee](https://github.com/rivet-gg/rivet/commit/9457beeb638b3e7be63fdf5dcf56378656fccd69))
* use correct endpoint for upload-prepare preisgned requests & presigning docs ([#1396](https://github.com/rivet-gg/rivet/issues/1396)) ([c4f2062](https://github.com/rivet-gg/rivet/commit/c4f2062776ee82c6b924b99439df30f9a9ffc47c))
* use EndpointKind::EdgeInternal for BuildDeliveryMethod::S3Direct ([#1429](https://github.com/rivet-gg/rivet/issues/1429)) ([b2ad415](https://github.com/rivet-gg/rivet/commit/b2ad415899d15bfd19433125b514413502064053))
* use internal endpoint for s3 provision ([#1318](https://github.com/rivet-gg/rivet/issues/1318)) ([357589c](https://github.com/rivet-gg/rivet/commit/357589cac04452345f260188072f889399dcdde8))
* validate build belongs to game ([#1523](https://github.com/rivet-gg/rivet/issues/1523)) ([14f3312](https://github.com/rivet-gg/rivet/commit/14f3312d00e8eae2f71f702529701cffc6e62e3a))
* validate tags arent empty ([#1528](https://github.com/rivet-gg/rivet/issues/1528)) ([4a1d9ad](https://github.com/rivet-gg/rivet/commit/4a1d9ad73541b0c6242f6b41cdcbb31028749643))
* **workflows:** filter messages by tags ([#1142](https://github.com/rivet-gg/rivet/issues/1142)) ([f07c270](https://github.com/rivet-gg/rivet/commit/f07c270caf61a89076e8db93444071c44dc151b1))
* **workflows:** get workflows working again with new history management ([#1210](https://github.com/rivet-gg/rivet/issues/1210)) ([f718812](https://github.com/rivet-gg/rivet/commit/f71881288a6f45f9077817b69daec25a3e6bd450))


### Documentation

* doc limitations ([#1567](https://github.com/rivet-gg/rivet/issues/1567)) ([9d4bd24](https://github.com/rivet-gg/rivet/commit/9d4bd24f64fa3952f255ef83901a541667c845e4))
* remove extra "follow setup guide" in setup ([5bcb988](https://github.com/rivet-gg/rivet/commit/5bcb9888ca54e79f090b513673c4edf9aa87dfc6))
* reorg docs ([#1259](https://github.com/rivet-gg/rivet/issues/1259)) ([34602e1](https://github.com/rivet-gg/rivet/commit/34602e1ad2b7da5529c6ebcd0f14dc0b27edf768))


### Code Refactoring

* docs framer cleanup ([#1520](https://github.com/rivet-gg/rivet/issues/1520)) ([125786b](https://github.com/rivet-gg/rivet/commit/125786b0a633ddc700e44ee63beacd4eb5fa62ba))
* **ds:** wait for ds poll events before marking actor as ready ([#1644](https://github.com/rivet-gg/rivet/issues/1644)) ([404fb90](https://github.com/rivet-gg/rivet/commit/404fb9073da8ffab56810f92ae6933ecb083a591))
* new framer pages & subtle adjustments ([#1635](https://github.com/rivet-gg/rivet/issues/1635)) ([3608b0b](https://github.com/rivet-gg/rivet/commit/3608b0b0da5325e5aaf3a88acd564fd6f9bc5413))


### Continuous Integration

* build & upload binaries ([#1658](https://github.com/rivet-gg/rivet/issues/1658)) ([cf4d331](https://github.com/rivet-gg/rivet/commit/cf4d3315a0008c01b3c99eaee60be5a258224038))
* remove docker arm targets for containers that rely on fdb ([#1656](https://github.com/rivet-gg/rivet/issues/1656)) ([2a8b5d8](https://github.com/rivet-gg/rivet/commit/2a8b5d83ed45d2cd6e89f3e888e73ce5134742f3))


### Chores

* **actors-sdk:** auto-determine api endpoint & environment to select region from ([#1588](https://github.com/rivet-gg/rivet/issues/1588)) ([3f06c21](https://github.com/rivet-gg/rivet/commit/3f06c21b7364352a577bcd7432c454e76f48cf67))
* **actors-sdk:** convert this._connections to map ([#1612](https://github.com/rivet-gg/rivet/issues/1612)) ([5daa00e](https://github.com/rivet-gg/rivet/commit/5daa00e1d52f46420babd88cec3d10a57c429f10))
* add `rivet manager endpoint` ([#1539](https://github.com/rivet-gg/rivet/issues/1539)) ([d1a8188](https://github.com/rivet-gg/rivet/commit/d1a8188a620aae92860576618e46b8adb686f130))
* add access controls to actor manager ([#1538](https://github.com/rivet-gg/rivet/issues/1538)) ([ac301b5](https://github.com/rivet-gg/rivet/commit/ac301b5c4f2ec3dfe48ea1f70ef38d600b1a0f02))
* add actor upgrades to deploys ([#1543](https://github.com/rivet-gg/rivet/issues/1543)) ([f70947c](https://github.com/rivet-gg/rivet/commit/f70947c1ee14a220ee664aa7ed6e3c3922448f7e))
* add actor wildcard dns records for gg nodes ([#1619](https://github.com/rivet-gg/rivet/issues/1619)) ([7b3fd7a](https://github.com/rivet-gg/rivet/commit/7b3fd7a523e03caea61ea6a5e2e9c2104b29643b))
* add actor-types sdk & move js ext to typescript ([#1458](https://github.com/rivet-gg/rivet/issues/1458)) ([e800f20](https://github.com/rivet-gg/rivet/commit/e800f20156113f0fc1242bd34862696eeb29e818))
* add client config to pb clients ([#1329](https://github.com/rivet-gg/rivet/issues/1329)) ([ff2c7e4](https://github.com/rivet-gg/rivet/commit/ff2c7e4efa7d81219dfdb6a5a7d951320b4702ed))
* add client container to docker compose ([#1289](https://github.com/rivet-gg/rivet/issues/1289)) ([15c844c](https://github.com/rivet-gg/rivet/commit/15c844c4712352660c3d3734169546610a28e1d5))
* add default user/team/project/env ([#1310](https://github.com/rivet-gg/rivet/issues/1310)) ([1dc72f2](https://github.com/rivet-gg/rivet/commit/1dc72f2b67846857b585e7bfd2602f972f5abd63))
* add deno passthrough ([#1541](https://github.com/rivet-gg/rivet/issues/1541)) ([10fe8d7](https://github.com/rivet-gg/rivet/commit/10fe8d7f47b2de347665af1782e1d99434296221))
* add docker-compose.yaml ([#1254](https://github.com/rivet-gg/rivet/issues/1254)) ([58d53e1](https://github.com/rivet-gg/rivet/commit/58d53e1243514a4e983db46c992dda9f5d9c5b99))
* add docs on handling of jwt ([#1353](https://github.com/rivet-gg/rivet/issues/1353)) ([571e9f6](https://github.com/rivet-gg/rivet/commit/571e9f62012694fbb63cba452cd27d580fa33d1e))
* add explicit error types for actors ([#1606](https://github.com/rivet-gg/rivet/issues/1606)) ([706b623](https://github.com/rivet-gg/rivet/commit/706b623ede3bdb1e400c0b89b8d7ac53e5d804cc))
* add foundationdb to dev-full & document avx support ([#1426](https://github.com/rivet-gg/rivet/issues/1426)) ([f6708f3](https://github.com/rivet-gg/rivet/commit/f6708f397a55a404e358aefbedf2ab04f3432e5b))
* add framer to index page ([#1390](https://github.com/rivet-gg/rivet/issues/1390)) ([25c62b3](https://github.com/rivet-gg/rivet/commit/25c62b312f4ec93a3ccbe29552b61205a424bddd))
* add health checks to docker ([#1262](https://github.com/rivet-gg/rivet/issues/1262)) ([3b4b1f0](https://github.com/rivet-gg/rivet/commit/3b4b1f096bbfd5ed230135a665496d794d838026))
* add implicit rpc calls to actors ([#1516](https://github.com/rivet-gg/rivet/issues/1516)) ([c92a40b](https://github.com/rivet-gg/rivet/commit/c92a40ba45947caca6e58c7cd4bbb85f7f47dd75))
* add jsr readme ([#1675](https://github.com/rivet-gg/rivet/issues/1675)) ([23729df](https://github.com/rivet-gg/rivet/commit/23729dfc31bf6bcb382c85fa91aa9927a4e0c118))
* add logging filters, reduce excess logging ([#1287](https://github.com/rivet-gg/rivet/issues/1287)) ([6e56610](https://github.com/rivet-gg/rivet/commit/6e56610f17d79c791be7a9c7c474b563bfd35b67))
* add manual e2e test for js actors ([#1322](https://github.com/rivet-gg/rivet/issues/1322)) ([6885adb](https://github.com/rivet-gg/rivet/commit/6885adbf60167359abf4b2c45290d76cac7485b4))
* add max message size ([#1611](https://github.com/rivet-gg/rivet/issues/1611)) ([5b6b382](https://github.com/rivet-gg/rivet/commit/5b6b382ca1d1908474939f96dffcda858d27bc43))
* add monolith container ([#1363](https://github.com/rivet-gg/rivet/issues/1363)) ([1d5e805](https://github.com/rivet-gg/rivet/commit/1d5e805d76f593b2fc1c03d0ab3ed25b9b35a10c))
* add monolith entry crate ([#1208](https://github.com/rivet-gg/rivet/issues/1208)) ([c2a4c86](https://github.com/rivet-gg/rivet/commit/c2a4c8635df696459015ac22c36b48d5cae57050))
* add os info to telemetry beacon ([#1235](https://github.com/rivet-gg/rivet/issues/1235)) ([d00491f](https://github.com/rivet-gg/rivet/commit/d00491f5bacf6d09b68a4dc775ea2a4764f79e59))
* add path-based routing for actors & add rg to docker compose ([#1548](https://github.com/rivet-gg/rivet/issues/1548)) ([cf8acc9](https://github.com/rivet-gg/rivet/commit/cf8acc9b6edde605f436aab13cbc4dc47efc0739))
* add pb runner troubleshooting ([#1432](https://github.com/rivet-gg/rivet/issues/1432)) ([a9db3de](https://github.com/rivet-gg/rivet/commit/a9db3de11cb7a3628baa7c3eb3f1aa78cca2040c))
* add prepareConnection, fix protected actor methods ([#1533](https://github.com/rivet-gg/rivet/issues/1533)) ([d73d026](https://github.com/rivet-gg/rivet/commit/d73d026f36797a7f46251d791c4de700122c53ce))
* add region id & name to rivet meta ([#1511](https://github.com/rivet-gg/rivet/issues/1511)) ([9765a83](https://github.com/rivet-gg/rivet/commit/9765a83f2bd2f25935060f251c1193a6ae27d155))
* add runner socket observability ([#1546](https://github.com/rivet-gg/rivet/issues/1546)) ([38e2003](https://github.com/rivet-gg/rivet/commit/38e2003e963c82f2113a06a2bb21d9643f186041))
* add servers backwards compatability ([#1335](https://github.com/rivet-gg/rivet/issues/1335)) ([a470cec](https://github.com/rivet-gg/rivet/commit/a470cec246dd8aafb27c1816a90271bf3987c562))
* add shell container & add back db commands ([#1215](https://github.com/rivet-gg/rivet/issues/1215)) ([d7df31d](https://github.com/rivet-gg/rivet/commit/d7df31dde76fdb4145c19b1899a65c69476523ba))
* add support for dumping builds ([#1586](https://github.com/rivet-gg/rivet/issues/1586)) ([1dfff4a](https://github.com/rivet-gg/rivet/commit/1dfff4af8be314306ced28d36f182e7f754e6b83))
* add support for multiple protocols ([#1610](https://github.com/rivet-gg/rivet/issues/1610)) ([568cc43](https://github.com/rivet-gg/rivet/commit/568cc43cfbbd8b51494ac4dfe8348c89ab855835))
* add support for running pb client without cgroups ([#1371](https://github.com/rivet-gg/rivet/issues/1371)) ([ffb294d](https://github.com/rivet-gg/rivet/commit/ffb294df3abf6fc8ef6ae502088e0ae520efc2bd))
* add welcome message to monolith ([#1405](https://github.com/rivet-gg/rivet/issues/1405)) ([d73fa4d](https://github.com/rivet-gg/rivet/commit/d73fa4da43efdb94e9b7dd4bb67eaadf6ed5f808))
* add zod for ws protocol ([#1608](https://github.com/rivet-gg/rivet/issues/1608)) ([5941785](https://github.com/rivet-gg/rivet/commit/5941785cd531a72e23bc84eb5a1f930b482b1a71))
* allow configuring isolate runner port ([#1366](https://github.com/rivet-gg/rivet/issues/1366)) ([746a2f1](https://github.com/rivet-gg/rivet/commit/746a2f1687f71b86a5ad7afdce3698427b89bd75))
* allow serivce tokens to access builds api ([#1587](https://github.com/rivet-gg/rivet/issues/1587)) ([b57bf31](https://github.com/rivet-gg/rivet/commit/b57bf31848199c414d4f49b9b2b2089f9d169ebd))
* **api-status:** add check for actor isolates ([#1642](https://github.com/rivet-gg/rivet/issues/1642)) ([5f23cb4](https://github.com/rivet-gg/rivet/commit/5f23cb492ddd3d5f6d610a4f8c321f5c2a2e22d3))
* authenticate default development user ([#1312](https://github.com/rivet-gg/rivet/issues/1312)) ([82d653d](https://github.com/rivet-gg/rivet/commit/82d653d7435cf0706c4a6d9b1f8da1953f8096cb))
* auto-publish docker images ([#1373](https://github.com/rivet-gg/rivet/issues/1373)) ([f3c07c3](https://github.com/rivet-gg/rivet/commit/f3c07c30fca491b912c215f7ae3d56be9bcaabb4))
* auto-read manager endpoint in tests ([#1542](https://github.com/rivet-gg/rivet/issues/1542)) ([f8f29b9](https://github.com/rivet-gg/rivet/commit/f8f29b9e7e9002bf74df7b47e8065d03fbb72aa1))
* automatically print admin access token url ([#1285](https://github.com/rivet-gg/rivet/issues/1285)) ([c4dddc2](https://github.com/rivet-gg/rivet/commit/c4dddc20562e2d0a8ddf66cdf87b4b905ebd8d5e))
* **bolt:** make cluster optional ([#1164](https://github.com/rivet-gg/rivet/issues/1164)) ([b625213](https://github.com/rivet-gg/rivet/commit/b6252134e0a98c08b9addf064aa03ea6d1629d2c))
* bump hub ui ([#1347](https://github.com/rivet-gg/rivet/issues/1347)) ([b94f2fc](https://github.com/rivet-gg/rivet/commit/b94f2fcde9450eb3684041135b35b8fcc75b97dc))
* check actor before deploy ([#1580](https://github.com/rivet-gg/rivet/issues/1580)) ([fb424d5](https://github.com/rivet-gg/rivet/commit/fb424d5aa1a13b74b9a896c237f33226eca4c6e8))
* clean up client config & e2e working manual test ([#1333](https://github.com/rivet-gg/rivet/issues/1333)) ([84c5d03](https://github.com/rivet-gg/rivet/commit/84c5d037557410842585e3f19ae9747440ce0f87))
* clean up openapi gen script ([#1504](https://github.com/rivet-gg/rivet/issues/1504)) ([d71c685](https://github.com/rivet-gg/rivet/commit/d71c68519fe52f55f7adf85ad5ac258460d9cb77))
* **cluster:** remove default provisioning config ([#1643](https://github.com/rivet-gg/rivet/issues/1643)) ([17de3c8](https://github.com/rivet-gg/rivet/commit/17de3c8919429068e39afd0a09c79b218a566df3))
* **cluster:** remove dns records when pruning servers ([#1646](https://github.com/rivet-gg/rivet/issues/1646)) ([ee62ca1](https://github.com/rivet-gg/rivet/commit/ee62ca1b2e4ec4913cc70c49a09424486765cac2))
* **cluster:** upgrade traefik to 3.2.1 ([#1653](https://github.com/rivet-gg/rivet/issues/1653)) ([d0290dc](https://github.com/rivet-gg/rivet/commit/d0290dc134804d407e540331d60cad3572896cb8))
* configure cluster for faster migrations in dev ([#1368](https://github.com/rivet-gg/rivet/issues/1368)) ([4dd3324](https://github.com/rivet-gg/rivet/commit/4dd3324ec730facff1109094bc57106e5958f81d))
* convert health checks & metrics to services ([#1267](https://github.com/rivet-gg/rivet/issues/1267)) ([45f0247](https://github.com/rivet-gg/rivet/commit/45f024750693c9fac1f236b34ab4bdbc6178f3da))
* convert more actor params to optional ([#1389](https://github.com/rivet-gg/rivet/issues/1389)) ([c96d528](https://github.com/rivet-gg/rivet/commit/c96d528d1509d31455602c109cd102ad4345c7d4))
* disable deno update check ([#1590](https://github.com/rivet-gg/rivet/issues/1590)) ([8ccda51](https://github.com/rivet-gg/rivet/commit/8ccda518d32c857283ca75d2968336a1e9d9c7aa))
* disable release build on monolith ([#1407](https://github.com/rivet-gg/rivet/issues/1407)) ([bfd14c9](https://github.com/rivet-gg/rivet/commit/bfd14c92fcf84a2b9137ca42da1d5362cb1633db))
* disable use_mounts in docker clients ([#1491](https://github.com/rivet-gg/rivet/issues/1491)) ([e2cea9a](https://github.com/rivet-gg/rivet/commit/e2cea9a88ae0ec93424a900210c597dc6c4d4db9))
* document naming conventions & aliases ([#1358](https://github.com/rivet-gg/rivet/issues/1358)) ([c23e5f8](https://github.com/rivet-gg/rivet/commit/c23e5f8c219a660f803a3af7b634f0b8222bccf8))
* don't query prometheus if disabled ([#1369](https://github.com/rivet-gg/rivet/issues/1369)) ([e0d2b98](https://github.com/rivet-gg/rivet/commit/e0d2b98f2cc9b13e1abf4b036ab78135b6c829b7))
* downgrate workflow logs to debug ([#1367](https://github.com/rivet-gg/rivet/issues/1367)) ([a7dafa6](https://github.com/rivet-gg/rivet/commit/a7dafa68b41fafcb85af64f8ad454274b38914c4))
* expose logging to actors ([#1602](https://github.com/rivet-gg/rivet/issues/1602)) ([77e7036](https://github.com/rivet-gg/rivet/commit/77e7036a14261d6c847e5c11d846799d2f308371))
* expose s3 endpoint publicly for docker compose ([#1319](https://github.com/rivet-gg/rivet/issues/1319)) ([6108a95](https://github.com/rivet-gg/rivet/commit/6108a9580a969cc0757d39110349cff4ac37af06))
* fail fast on service initialization ([#1264](https://github.com/rivet-gg/rivet/issues/1264)) ([970cd9d](https://github.com/rivet-gg/rivet/commit/970cd9d2bbfd13836f7c4c4a6b054c730ee82c45))
* fallback to server error handler for servers ([#1266](https://github.com/rivet-gg/rivet/issues/1266)) ([a4f625f](https://github.com/rivet-gg/rivet/commit/a4f625f75a29d6eae31a7148a4600efcbac85d45))
* finalize pegboard env ([#1420](https://github.com/rivet-gg/rivet/issues/1420)) ([e51f3ed](https://github.com/rivet-gg/rivet/commit/e51f3ed821e14a6d39a2dc8ee0856acb2fe195b4))
* finalize query syntax for manager ([#1607](https://github.com/rivet-gg/rivet/issues/1607)) ([a414285](https://github.com/rivet-gg/rivet/commit/a414285b209854dcc8503a8ca1694ce3c6272259))
* fix & fmt ([#1343](https://github.com/rivet-gg/rivet/issues/1343)) ([73f4b4e](https://github.com/rivet-gg/rivet/commit/73f4b4e40693a1fe6dc4321cbcce7dbee71ddcd2))
* fix actor sdk biome lints ([#1600](https://github.com/rivet-gg/rivet/issues/1600)) ([5191d0f](https://github.com/rivet-gg/rivet/commit/5191d0f0475dee9feb14d6ccbd9f0e9029ad402a))
* fix allowed deps ([#1337](https://github.com/rivet-gg/rivet/issues/1337)) ([b1b95d8](https://github.com/rivet-gg/rivet/commit/b1b95d813dd4c245cada277ab6473ab7541c2cf5))
* fix api-edge cdn perf issue ([#1638](https://github.com/rivet-gg/rivet/issues/1638)) ([2d94cd9](https://github.com/rivet-gg/rivet/commit/2d94cd97dbfa4c6a8db1338b46983bb050c50598))
* fix check version, fix ds rescheduling ([#1634](https://github.com/rivet-gg/rivet/issues/1634)) ([808c60f](https://github.com/rivet-gg/rivet/commit/808c60f1f511837ce011a3f66b997a0ea1c977cf))
* fix cluster deploy ([#1605](https://github.com/rivet-gg/rivet/issues/1605)) ([cea5333](https://github.com/rivet-gg/rivet/commit/cea53337cb0a4e1578208251c4938f3eef2fdd36))
* fix compile errors ([#1427](https://github.com/rivet-gg/rivet/issues/1427)) ([ad47c71](https://github.com/rivet-gg/rivet/commit/ad47c717bff4f43ab6ab080f325706d455685290))
* fix dev clickhouse migrations & storage engine ([#1350](https://github.com/rivet-gg/rivet/issues/1350)) ([e6021cc](https://github.com/rivet-gg/rivet/commit/e6021cca636f74eb700377d1d4d1a1d7737a7441))
* fix generated guard public hostname to use actor subdomain ([#1622](https://github.com/rivet-gg/rivet/issues/1622)) ([2659afa](https://github.com/rivet-gg/rivet/commit/2659afa0199ce1abec3a4c86836e0a179502d95b))
* fix incorrect cron syntax ([#1286](https://github.com/rivet-gg/rivet/issues/1286)) ([9e6726a](https://github.com/rivet-gg/rivet/commit/9e6726af6cac33df1565e69acaf6eab232b2004a))
* fix license ([#1406](https://github.com/rivet-gg/rivet/issues/1406)) ([e319fe0](https://github.com/rivet-gg/rivet/commit/e319fe075eb2698ac42f3356c90917346ec89c7d))
* fix lints ([#1342](https://github.com/rivet-gg/rivet/issues/1342)) ([a7e2e49](https://github.com/rivet-gg/rivet/commit/a7e2e493ef308afe31bde0b51dfacae9930b13d3))
* fix loading config in tests ([#1238](https://github.com/rivet-gg/rivet/issues/1238)) ([de562cb](https://github.com/rivet-gg/rivet/commit/de562cb0e792faba6c97e5dbe83f9a3d27ed997f))
* fix merge bugs ([#1354](https://github.com/rivet-gg/rivet/issues/1354)) ([c0d097d](https://github.com/rivet-gg/rivet/commit/c0d097dfd0d5d0f0409561f82d09c8358502a99e))
* fix pb server install ([#1615](https://github.com/rivet-gg/rivet/issues/1615)) ([a84a33f](https://github.com/rivet-gg/rivet/commit/a84a33f849f56f01e15a2501ce5c1e25269df039))
* fix protocol for clickhouse http ([#1359](https://github.com/rivet-gg/rivet/issues/1359)) ([6a8189a](https://github.com/rivet-gg/rivet/commit/6a8189a440a9ae5a2b6a388c4b975d5044f8829d))
* fix region not being used in actor sdk ([#1623](https://github.com/rivet-gg/rivet/issues/1623)) ([f5e2741](https://github.com/rivet-gg/rivet/commit/f5e27413c0db623615b762df43d58de748f93ce9))
* fix warnings from cargo check ([#1576](https://github.com/rivet-gg/rivet/issues/1576)) ([147a298](https://github.com/rivet-gg/rivet/commit/147a298919f3592e13ed8cfafa6916edb97ef97f))
* gracefully handle returning servers when dns not configured ([#1334](https://github.com/rivet-gg/rivet/issues/1334)) ([c7ec0af](https://github.com/rivet-gg/rivet/commit/c7ec0af8f82c6b2b68185e83a49be0d1a45e6eb7))
* hard crash on fail to bind server ([#1268](https://github.com/rivet-gg/rivet/issues/1268)) ([e7da3c0](https://github.com/rivet-gg/rivet/commit/e7da3c067581f7c70b972eee2297c2bbcfb6f4c8))
* impelment vector-client & vector-server configs, add support for dns addresses for vector ([#1351](https://github.com/rivet-gg/rivet/issues/1351)) ([6f1faf7](https://github.com/rivet-gg/rivet/commit/6f1faf7850aa0c8208675b731fd6c46e92103b9a))
* increase min crdb connections for dev ([#1348](https://github.com/rivet-gg/rivet/issues/1348)) ([2f06756](https://github.com/rivet-gg/rivet/commit/2f06756a9444db3381453d33da1843d20206fe33))
* make actor create runtime optional & fix inconsistent snake case in api ([#1494](https://github.com/rivet-gg/rivet/issues/1494)) ([f260ab5](https://github.com/rivet-gg/rivet/commit/f260ab5612ee1a05bc8153086891a774ed29824b))
* merge rivet-gg/site in to docs/ folder ([#1250](https://github.com/rivet-gg/rivet/issues/1250)) ([dc823b6](https://github.com/rivet-gg/rivet/commit/dc823b691d98eb4f993ad805efea18c3ebfed469))
* merge toolchain in to monorepo ([#1501](https://github.com/rivet-gg/rivet/issues/1501)) ([904c584](https://github.com/rivet-gg/rivet/commit/904c58447d306b858c864241f42dad3dd02bfdcc))
* migrate to seaweedfs ([#1263](https://github.com/rivet-gg/rivet/issues/1263)) ([3f8dbf0](https://github.com/rivet-gg/rivet/commit/3f8dbf08f0e0e3d511b16c05ae10fc4f0c77dc42))
* migrate to workspace package config ([#1399](https://github.com/rivet-gg/rivet/issues/1399)) ([b9969f1](https://github.com/rivet-gg/rivet/commit/b9969f1357ac7a98d32a0a51ea4fc9e9e01c58bb))
* misc ([#1200](https://github.com/rivet-gg/rivet/issues/1200)) ([bebc9bf](https://github.com/rivet-gg/rivet/commit/bebc9bf66ce3f56936b517b9d0e5458b6be9b655))
* misc fixes ([#1224](https://github.com/rivet-gg/rivet/issues/1224)) ([0bea10f](https://github.com/rivet-gg/rivet/commit/0bea10f7638446dca91b3ee1fc2c7a1e81254aa0))
* move actor sdk logging to @std/log ([#1601](https://github.com/rivet-gg/rivet/issues/1601)) ([799bee3](https://github.com/rivet-gg/rivet/commit/799bee330fd9568e74dd7d9134887c873e39ede6))
* move bolt cli ([#1216](https://github.com/rivet-gg/rivet/issues/1216)) ([f8ba24d](https://github.com/rivet-gg/rivet/commit/f8ba24d31daa0733fa10ae45fcc2664616f45a6e))
* move crons to use native cron type ([#1272](https://github.com/rivet-gg/rivet/issues/1272)) ([15eb44b](https://github.com/rivet-gg/rivet/commit/15eb44b2ec349ec1f635518c6e17e620886d801f))
* move docs -&gt; site and add docs symlink ([#1664](https://github.com/rivet-gg/rivet/issues/1664)) ([c152e39](https://github.com/rivet-gg/rivet/commit/c152e39f381315d2ab95a3994a5db7ee3d333289))
* move docs-old -&gt; docs-internal ([#1344](https://github.com/rivet-gg/rivet/issues/1344)) ([39d95d0](https://github.com/rivet-gg/rivet/commit/39d95d0f65d66995551fee1bfaebbf708e6aa70d))
* move docs-old to docs-internal & remove old docs ([#1345](https://github.com/rivet-gg/rivet/issues/1345)) ([ece020b](https://github.com/rivet-gg/rivet/commit/ece020ba9fecafb22c2370fc6b6ed242a8cb2e36))
* move env vars to config ([#1220](https://github.com/rivet-gg/rivet/issues/1220)) ([d07e6f2](https://github.com/rivet-gg/rivet/commit/d07e6f2d544cf0f48e06d05f2df9393a05c8a46e))
* move errors/ to lib/formatted-errors/errorsf ([#1244](https://github.com/rivet-gg/rivet/issues/1244)) ([a34e69e](https://github.com/rivet-gg/rivet/commit/a34e69ed7966d32d30b490fa8189b1d525c12aba))
* move fern to sdks folder ([#1246](https://github.com/rivet-gg/rivet/issues/1246)) ([e74805d](https://github.com/rivet-gg/rivet/commit/e74805df46dc8e67ef85c7e0505e07d2bcf2b90b))
* move media to subfolder ([#1245](https://github.com/rivet-gg/rivet/issues/1245)) ([43c2c79](https://github.com/rivet-gg/rivet/commit/43c2c79f66145aa8efa72ca91ba43d231c776f30))
* move migrate to rivet binary ([#1209](https://github.com/rivet-gg/rivet/issues/1209)) ([9ba8730](https://github.com/rivet-gg/rivet/commit/9ba8730a529d1e33de167a8a5937e4031b8e80ec))
* move networking config to pb clients ([#1332](https://github.com/rivet-gg/rivet/issues/1332)) ([0f64653](https://github.com/rivet-gg/rivet/commit/0f6465342b5b9dc32cd240e86727d4a273e38bd1))
* move pb binary urls in to cluster config ([#1309](https://github.com/rivet-gg/rivet/issues/1309)) ([f638e9c](https://github.com/rivet-gg/rivet/commit/f638e9cae06e64b6a5639a5e1f3fc168c0510531))
* move pegboard to container ([#1305](https://github.com/rivet-gg/rivet/issues/1305)) ([edebc41](https://github.com/rivet-gg/rivet/commit/edebc418ca434130a38aefeffb89e0003c30705b))
* move proto to legacy ([#1243](https://github.com/rivet-gg/rivet/issues/1243)) ([ed62de3](https://github.com/rivet-gg/rivet/commit/ed62de3213179b305c772e6b80bae2da1913ba3c))
* move resources/docker -&gt; docker ([#1261](https://github.com/rivet-gg/rivet/issues/1261)) ([9ca338b](https://github.com/rivet-gg/rivet/commit/9ca338b691f55504c347400ff52a51064f1b25b0))
* move sdk -&gt; sdk/api ([#1457](https://github.com/rivet-gg/rivet/issues/1457)) ([27f0bb2](https://github.com/rivet-gg/rivet/commit/27f0bb25af99fa82bcbc9a9654f090998dd7b69b))
* **pegboard:** get connection working e2e ([#1161](https://github.com/rivet-gg/rivet/issues/1161)) ([f4737bb](https://github.com/rivet-gg/rivet/commit/f4737bb66fd61d661547bd2cce88650e4c9c43b5))
* polish actor api ([#1357](https://github.com/rivet-gg/rivet/issues/1357)) ([3c985dd](https://github.com/rivet-gg/rivet/commit/3c985dd01c9b5d13a33765b496ed2d2febcf0696))
* print correct urls for actors & builds on deploy ([#1592](https://github.com/rivet-gg/rivet/issues/1592)) ([3653358](https://github.com/rivet-gg/rivet/commit/36533580cb498814c604efd7d3c4a0c31c23e417))
* proof v1 docs ([#1529](https://github.com/rivet-gg/rivet/issues/1529)) ([a3d9ea0](https://github.com/rivet-gg/rivet/commit/a3d9ea077665a5ed735ce332b9c6244d2cd6414c))
* reduce chirp logging ([#1279](https://github.com/rivet-gg/rivet/issues/1279)) ([a98977c](https://github.com/rivet-gg/rivet/commit/a98977c1844d0443d4ea953f549235533a67330d))
* reduce fdb process count ([#1620](https://github.com/rivet-gg/rivet/issues/1620)) ([abd7b27](https://github.com/rivet-gg/rivet/commit/abd7b27d637e3bbc038aa8fe2ed7c9e1ee3dc497))
* reduce startup log verbosity ([#1270](https://github.com/rivet-gg/rivet/issues/1270)) ([86f5f71](https://github.com/rivet-gg/rivet/commit/86f5f71d6c0d22c8cda6e46d70a8155613838909))
* remove access token ([#1311](https://github.com/rivet-gg/rivet/issues/1311)) ([c6c710e](https://github.com/rivet-gg/rivet/commit/c6c710e2d2c3cdd5c6066819e7107d381cd04c27))
* remove api-admin artifacts ([#1317](https://github.com/rivet-gg/rivet/issues/1317)) ([742c4ba](https://github.com/rivet-gg/rivet/commit/742c4bac3a5f8d4a5f3c4c81268538c58c18ad31))
* remove build name from api ([#1547](https://github.com/rivet-gg/rivet/issues/1547)) ([71a2fde](https://github.com/rivet-gg/rivet/commit/71a2fdeef7df1be89c8722e607af1d937cad8f6f))
* remove default cni network interface ([#1339](https://github.com/rivet-gg/rivet/issues/1339)) ([5787f7d](https://github.com/rivet-gg/rivet/commit/5787f7dea60a8bb15addb46cc2943b6859042273))
* remove devcontainer ([#1503](https://github.com/rivet-gg/rivet/issues/1503)) ([9293d35](https://github.com/rivet-gg/rivet/commit/9293d354ed4f04bb034137859565cf07792b7f68))
* remove excess pb-ws logs ([#1370](https://github.com/rivet-gg/rivet/issues/1370)) ([861c94e](https://github.com/rivet-gg/rivet/commit/861c94e283d67d618f6f81b11093029a391dbb6d))
* remove fdb from shell.nix on macos ([#1424](https://github.com/rivet-gg/rivet/issues/1424)) ([bdd7ba3](https://github.com/rivet-gg/rivet/commit/bdd7ba3594a522dacd5b89154170cb4639181c00))
* remove grafana & prometheus from dev setup ([#1352](https://github.com/rivet-gg/rivet/issues/1352)) ([2f0187c](https://github.com/rivet-gg/rivet/commit/2f0187c37e81497b0a5ce38083776b6ab0e76139))
* remove imagor reference ([#1283](https://github.com/rivet-gg/rivet/issues/1283)) ([d6e3295](https://github.com/rivet-gg/rivet/commit/d6e329501482a2e8ade74f543ac5bac91ed412d9))
* remove json enums ([#1151](https://github.com/rivet-gg/rivet/issues/1151)) ([46dc2fc](https://github.com/rivet-gg/rivet/commit/46dc2fc83684d6095c8490d1689b2c3a5240d343))
* remove old examples ([#1585](https://github.com/rivet-gg/rivet/issues/1585)) ([ecd4df3](https://github.com/rivet-gg/rivet/commit/ecd4df36b36b5ece7295ed2b466586ca267f4dd9))
* remove public_ prefix ([#1550](https://github.com/rivet-gg/rivet/issues/1550)) ([27b3a1f](https://github.com/rivet-gg/rivet/commit/27b3a1fba3db6d2d85c99761bb051bc22309d613))
* remove remaining tf infra scripts ([#1242](https://github.com/rivet-gg/rivet/issues/1242)) ([a6ccd5e](https://github.com/rivet-gg/rivet/commit/a6ccd5e7ff14a44fb150809a75bc8795f82e6a4b))
* remove sccache ([#1381](https://github.com/rivet-gg/rivet/issues/1381)) ([8432381](https://github.com/rivet-gg/rivet/commit/843238171f451cccf27138f2872753e2683f1a59))
* remove submodules ([#1403](https://github.com/rivet-gg/rivet/issues/1403)) ([5ee441c](https://github.com/rivet-gg/rivet/commit/5ee441ca44f771d747dad4b73a140de1a2900690))
* remove unneeded nix configs ([#1248](https://github.com/rivet-gg/rivet/issues/1248)) ([1b0d5d7](https://github.com/rivet-gg/rivet/commit/1b0d5d7e638976a4dadec2e511956a1d610fa515))
* remove unneeded workflow backfill ([#1271](https://github.com/rivet-gg/rivet/issues/1271)) ([5f03a98](https://github.com/rivet-gg/rivet/commit/5f03a983b4099b3e3ff39ba795ba04bd6de8c431))
* remove unused fern routes ([#1282](https://github.com/rivet-gg/rivet/issues/1282)) ([392e780](https://github.com/rivet-gg/rivet/commit/392e7807bc37434911ea3e805b18b509de850252))
* remove unused services ([#1280](https://github.com/rivet-gg/rivet/issues/1280)) ([2f332cc](https://github.com/rivet-gg/rivet/commit/2f332cc0d5c36a359a2bdfee5726b46b71352185))
* remove user-presence ([#1207](https://github.com/rivet-gg/rivet/issues/1207)) ([450f1ab](https://github.com/rivet-gg/rivet/commit/450f1ab3860585b0fb3e1afa86e956239d4b69f0))
* rename actor stakeholder -&gt; manager ([#1336](https://github.com/rivet-gg/rivet/issues/1336)) ([99b4c86](https://github.com/rivet-gg/rivet/commit/99b4c86f2a06956eb041fb91a90c8354a82215b7))
* rename api sdk Rivet -&gt; RivetClient ([#1462](https://github.com/rivet-gg/rivet/issues/1462)) ([e03abf4](https://github.com/rivet-gg/rivet/commit/e03abf4a3388de357f66a51ff1f22c5603f17e75))
* rename api-internal -&gt; api-edge, add api-private, move admin to api-private ([#1284](https://github.com/rivet-gg/rivet/issues/1284)) ([4659b2e](https://github.com/rivet-gg/rivet/commit/4659b2efd6657f1c73a5bb694ceeba03b489dc6e))
* rename game_guard -&gt; rivet guard ([#1417](https://github.com/rivet-gg/rivet/issues/1417)) ([76312de](https://github.com/rivet-gg/rivet/commit/76312def5c625a8132cac8b14d1b0bfd2746647d))
* rename protected properties to `_*` ([#1603](https://github.com/rivet-gg/rivet/issues/1603)) ([2e55803](https://github.com/rivet-gg/rivet/commit/2e558036332c553ae493282e0acab964c7b9a9ed))
* reorg binary names for rivet-server and rivet-client ([#1308](https://github.com/rivet-gg/rivet/issues/1308)) ([74c4f00](https://github.com/rivet-gg/rivet/commit/74c4f00324320d79a6e69d0c00e796fbdf6b1db8))
* reorg rust packages ([#1247](https://github.com/rivet-gg/rivet/issues/1247)) ([26ce993](https://github.com/rivet-gg/rivet/commit/26ce9938d38ccd89e0db2b2f9b07ab2dc2c369d7))
* replace nix bolt with cargo run command ([#1214](https://github.com/rivet-gg/rivet/issues/1214)) ([aaafef2](https://github.com/rivet-gg/rivet/commit/aaafef2d27efb2fb5b701456a906f9eda9240f03))
* replace submodules with workspace deps ([#1402](https://github.com/rivet-gg/rivet/issues/1402)) ([8fd4cce](https://github.com/rivet-gg/rivet/commit/8fd4ccec95fac37cef28af4f1c793ffb96581566))
* restructure docs for actors ([#1519](https://github.com/rivet-gg/rivet/issues/1519)) ([37e2ac9](https://github.com/rivet-gg/rivet/commit/37e2ac958bf9e463f147d265a27596c3012d3fb1))
* return TOKEN_REVOKED if user no longer exists ([#1372](https://github.com/rivet-gg/rivet/issues/1372)) ([89fe88d](https://github.com/rivet-gg/rivet/commit/89fe88d076475c05481edd4927121484d7d9ce6e))
* run dev db migrations in parallel for faster startups ([#1341](https://github.com/rivet-gg/rivet/issues/1341)) ([9210932](https://github.com/rivet-gg/rivet/commit/921093229ed4d642c67b80e844bb082e97737916))
* serve hub on api ([#1258](https://github.com/rivet-gg/rivet/issues/1258)) ([939b076](https://github.com/rivet-gg/rivet/commit/939b07642bac7c0e266af7654c3410bceeb07995))
* serve ui from api ([#1260](https://github.com/rivet-gg/rivet/issues/1260)) ([5bf95b3](https://github.com/rivet-gg/rivet/commit/5bf95b33401a681530485ae4390bd1f4e32032a1))
* **servers:** increase rate limit ([#1194](https://github.com/rivet-gg/rivet/issues/1194)) ([c265a37](https://github.com/rivet-gg/rivet/commit/c265a373a0bb546c1cc462eede49af28c9e28aad))
* shorten protocol keys ([#1609](https://github.com/rivet-gg/rivet/issues/1609)) ([1bacf59](https://github.com/rivet-gg/rivet/commit/1bacf5986278f6c95d6a01db8d7d04415e0dea12))
* simplify build config ([#1589](https://github.com/rivet-gg/rivet/issues/1589)) ([fab93de](https://github.com/rivet-gg/rivet/commit/fab93de39d0aa82922de3792734a0eb242685ab9))
* **site:** update links & nav ([#1669](https://github.com/rivet-gg/rivet/issues/1669)) ([c45544e](https://github.com/rivet-gg/rivet/commit/c45544eb37f613fc101f9e2f052efd4e3f581f29))
* standardize licenses ([#1338](https://github.com/rivet-gg/rivet/issues/1338)) ([016392a](https://github.com/rivet-gg/rivet/commit/016392a60561c14cf1efd15cc2105af3b312358f))
* switch back to main fern repo ([#1327](https://github.com/rivet-gg/rivet/issues/1327)) ([e2d5587](https://github.com/rivet-gg/rivet/commit/e2d558768b4b1edee279ce3bbf9b981ad6c0b6c0))
* switch client from openssl to rustls ([#1365](https://github.com/rivet-gg/rivet/issues/1365)) ([b1d1156](https://github.com/rivet-gg/rivet/commit/b1d1156c0caa83bfd0c81e79feabc9b0f801de56))
* throttle save state ([#1604](https://github.com/rivet-gg/rivet/issues/1604)) ([0310f1e](https://github.com/rivet-gg/rivet/commit/0310f1ee408c18ff2a665d08e7796a9565eebf55))
* tweak pb manager logs ([#1377](https://github.com/rivet-gg/rivet/issues/1377)) ([af7a67b](https://github.com/rivet-gg/rivet/commit/af7a67bdcf528ed977e1b7581930cefebb749fae))
* tweak release config ([#1307](https://github.com/rivet-gg/rivet/issues/1307)) ([b577339](https://github.com/rivet-gg/rivet/commit/b577339a6a61d76a93511c3db4ca5393f67c2956))
* update .gitattributes ([#1484](https://github.com/rivet-gg/rivet/issues/1484)) ([fa3214c](https://github.com/rivet-gg/rivet/commit/fa3214c4f809e5f054469125b9d85802484e2918))
* update actor api ([#1306](https://github.com/rivet-gg/rivet/issues/1306)) ([551821a](https://github.com/rivet-gg/rivet/commit/551821ad26255e574770775858693ec9cbcc599a))
* update bolt with new service definition ([#1211](https://github.com/rivet-gg/rivet/issues/1211)) ([6208bc2](https://github.com/rivet-gg/rivet/commit/6208bc2d47935b7e50a8124f9f51d899ddbbb5c6))
* update default pb config to match new structure ([#1340](https://github.com/rivet-gg/rivet/issues/1340)) ([4973a45](https://github.com/rivet-gg/rivet/commit/4973a459cbcbb7c3fcaa6b6abfae1662ea4faba3))
* update dev-full restart policy to unless-stopped ([#1500](https://github.com/rivet-gg/rivet/issues/1500)) ([94ff2d8](https://github.com/rivet-gg/rivet/commit/94ff2d8cb913709f7e52a9031b2d8073fc8c7ab6))
* update init projects ([#1591](https://github.com/rivet-gg/rivet/issues/1591)) ([c8dfaed](https://github.com/rivet-gg/rivet/commit/c8dfaedc5a627c11228f7afca49236fbee49ad3c))
* update manual e2e test for regions ([#1361](https://github.com/rivet-gg/rivet/issues/1361)) ([1ce77ec](https://github.com/rivet-gg/rivet/commit/1ce77ec08f22a48f04c4e7c14f207743774f490e))
* **workflows:** tidy up internals ([#1199](https://github.com/rivet-gg/rivet/issues/1199)) ([3f0ff1e](https://github.com/rivet-gg/rivet/commit/3f0ff1e3fa7ce913118672dd8f4770b8d039f6b7))
* write release script ([#1657](https://github.com/rivet-gg/rivet/issues/1657)) ([a3db31f](https://github.com/rivet-gg/rivet/commit/a3db31f3b5c351061d665003a6a8dfbf3372690b))

## [24.5.2](https://github.com/rivet-gg/rivet/compare/v24.5.1...v24.5.2) (2024-09-30)


### Features

* add hash to bootstrap ([#1168](https://github.com/rivet-gg/rivet/issues/1168)) ([4b0be5f](https://github.com/rivet-gg/rivet/commit/4b0be5f071b27d524c824e9b7ed4039e8354fe45))
* **bolt:** add workflow commands to bolt ([#1131](https://github.com/rivet-gg/rivet/issues/1131)) ([a47b296](https://github.com/rivet-gg/rivet/commit/a47b296939d04d204428f7ce91b68c75550d9b30))
* **builds:** add prewarm ats for builds ([#1176](https://github.com/rivet-gg/rivet/issues/1176)) ([0a269b4](https://github.com/rivet-gg/rivet/commit/0a269b46e578650f7672656569bff8e703cbd9f0))


### Bug Fixes

* allocation sizes for nomad ([#1127](https://github.com/rivet-gg/rivet/issues/1127)) ([2e3217e](https://github.com/rivet-gg/rivet/commit/2e3217edf975cfc8c0cbc035f3ada92c2641b610))
* **bolt:** add forwarded and persistent db shells ([#1130](https://github.com/rivet-gg/rivet/issues/1130)) ([f9b6707](https://github.com/rivet-gg/rivet/commit/f9b6707888e34a7ea9564a79a05024434e80610f))
* **builds:** allow null tags ([#1177](https://github.com/rivet-gg/rivet/issues/1177)) ([1c75e3b](https://github.com/rivet-gg/rivet/commit/1c75e3b68af0347bd457cdda4d6efb6eac073434))
* **builds:** fix exclusive tags query ([#1173](https://github.com/rivet-gg/rivet/issues/1173)) ([b8c323a](https://github.com/rivet-gg/rivet/commit/b8c323aa725d7d7a1450e5fb72c8654ea7ed93b7))
* **clusters:** allow dns deletion when draining and tainted ([#1132](https://github.com/rivet-gg/rivet/issues/1132)) ([c808d08](https://github.com/rivet-gg/rivet/commit/c808d08df13083d292399d516a5cf080f8a622ca))
* **cluster:** skip pruning servers without provider server id ([#1133](https://github.com/rivet-gg/rivet/issues/1133)) ([ca43432](https://github.com/rivet-gg/rivet/commit/ca43432fae665a98f0efbecf88013c6985ebb9f3))
* **ds, mm:** hard code disk per core ([#1134](https://github.com/rivet-gg/rivet/issues/1134)) ([5ee2809](https://github.com/rivet-gg/rivet/commit/5ee28094409aa4a620d8caff4df739f3fd08381b))
* **ds:** add back runc cleanup ([#1172](https://github.com/rivet-gg/rivet/issues/1172)) ([8e08889](https://github.com/rivet-gg/rivet/commit/8e0888933c9d4481863df0e6b9677089c2f1b73c))
* fix build tags ([#1190](https://github.com/rivet-gg/rivet/issues/1190)) ([6e2d214](https://github.com/rivet-gg/rivet/commit/6e2d2148b595f3ebcd3f12bc34e67c1af432e643))
* fix documentation link for errors ([#1174](https://github.com/rivet-gg/rivet/issues/1174)) ([eb7fdaf](https://github.com/rivet-gg/rivet/commit/eb7fdaff5ac63f1bed9d01db9b04d24ef0d969e3))
* **job-run:** fix dupe allocs, re-enable drain all ([#1128](https://github.com/rivet-gg/rivet/issues/1128)) ([d019e01](https://github.com/rivet-gg/rivet/commit/d019e01cfc9f6420492f9e9213563b4823ff23fe))
* **mm, ds:** fix dupe alloc killing ([#1124](https://github.com/rivet-gg/rivet/issues/1124)) ([dcdb06a](https://github.com/rivet-gg/rivet/commit/dcdb06a050b12561a679a84f4b243417a15d3071))
* more accurate job-run cpu metrics ([#1122](https://github.com/rivet-gg/rivet/issues/1122)) ([312958e](https://github.com/rivet-gg/rivet/commit/312958e79b97f5b57eb297fcc03d2e1a3d2cd695))
* reduce scheduler skew on distributed clusters ([#1175](https://github.com/rivet-gg/rivet/issues/1175)) ([2794e09](https://github.com/rivet-gg/rivet/commit/2794e0986fe14147ee92f8381cab7c517a0f28fb))
* **worfklows:** add silence ts ([#1129](https://github.com/rivet-gg/rivet/issues/1129)) ([06d965b](https://github.com/rivet-gg/rivet/commit/06d965bf9088b1a69177f9e7fac7d3b7d9e8371d))
* **workflows:** add error message for max sql retries ([#1125](https://github.com/rivet-gg/rivet/issues/1125)) ([80a33f0](https://github.com/rivet-gg/rivet/commit/80a33f00509b7c08aa3a670ded38f180fc9d29f7))
* **workflows:** add retry delay for txn errors ([#1138](https://github.com/rivet-gg/rivet/issues/1138)) ([614846b](https://github.com/rivet-gg/rivet/commit/614846be4f712e570bfcf5e3cf595848ce60852d))
* **workflows:** use unions instead of OR ([#1170](https://github.com/rivet-gg/rivet/issues/1170)) ([1ca8ab6](https://github.com/rivet-gg/rivet/commit/1ca8ab64e3793eb1b53839b7139b73c1b800377f))


### Chores

* add back node exporter metrics ([#1136](https://github.com/rivet-gg/rivet/issues/1136)) ([1eeedcb](https://github.com/rivet-gg/rivet/commit/1eeedcb83be74398c8ac71888bd652305c154666))
* enable batch ssh commands ([#1119](https://github.com/rivet-gg/rivet/issues/1119)) ([505c09c](https://github.com/rivet-gg/rivet/commit/505c09cd7b74db003324c5f283638fbfb8acc4b8))
* increase install timeout ([#1139](https://github.com/rivet-gg/rivet/issues/1139)) ([38584c9](https://github.com/rivet-gg/rivet/commit/38584c92f6f7b879070d63e43d46fbf8d7d34242))
* increase nomad heartbeat ttl ([#1140](https://github.com/rivet-gg/rivet/issues/1140)) ([437494a](https://github.com/rivet-gg/rivet/commit/437494ab658889543f886082355c8a68767ed54c))
* **linode:** pin kernel version ([#1123](https://github.com/rivet-gg/rivet/issues/1123)) ([48686f7](https://github.com/rivet-gg/rivet/commit/48686f7cad56aa5ed35d1657a4957d43781dc7d3))
* release 24.5.2 ([90318ca](https://github.com/rivet-gg/rivet/commit/90318ca55d19a89e68c264e507b80f2140221d73))
* remove bolt templates ([#1135](https://github.com/rivet-gg/rivet/issues/1135)) ([f0925f0](https://github.com/rivet-gg/rivet/commit/f0925f0ae989c55a913e383fac43b9e5ebfa03bf))
* revert new node exporter metrics ([#1118](https://github.com/rivet-gg/rivet/issues/1118)) ([07b6095](https://github.com/rivet-gg/rivet/commit/07b60954115ba10e324512f8b221ace507cc50df))

## [24.5.1](https://github.com/rivet-gg/rivet/compare/v24.5.0...v24.5.1) (2024-09-04)


### Features

* **workflows:** clean up dispatching syntax ([#1079](https://github.com/rivet-gg/rivet/issues/1079)) ([233efcc](https://github.com/rivet-gg/rivet/commit/233efcc2df179e2e1a8b456b14eb9116253fe1aa))


### Bug Fixes

* **clusters:** add drain padding to nomad ([#1100](https://github.com/rivet-gg/rivet/issues/1100)) ([01ee21b](https://github.com/rivet-gg/rivet/commit/01ee21b5da0b46b8aa2356ced8af3b6d9966ba7e))
* **clusters:** fix list lost op ([#1110](https://github.com/rivet-gg/rivet/issues/1110)) ([8ae85d2](https://github.com/rivet-gg/rivet/commit/8ae85d29e8fce11ed62a935f0de0119e36770589))
* **clusters:** gracefully handle node not found ([#1099](https://github.com/rivet-gg/rivet/issues/1099)) ([b460374](https://github.com/rivet-gg/rivet/commit/b4603749ad711e3baa7bdcb482b255c4c01e3dcf))
* **clusters:** remove nomad drain complete signal ([#1101](https://github.com/rivet-gg/rivet/issues/1101)) ([c117224](https://github.com/rivet-gg/rivet/commit/c117224484b49070213fc3c2f98a2d399c097dc9))
* **clusters:** switch from drain to ineligible system ([#1102](https://github.com/rivet-gg/rivet/issues/1102)) ([09f5143](https://github.com/rivet-gg/rivet/commit/09f5143ea94c199ee006b9051e4c97ebc53d70bd))
* **ds:** change nomad prefix ([#1113](https://github.com/rivet-gg/rivet/issues/1113)) ([705a470](https://github.com/rivet-gg/rivet/commit/705a470cbdb6a21bd21209445d4c507b2e39356e))
* **ds:** implement nomad monitors with signals ([#1105](https://github.com/rivet-gg/rivet/issues/1105)) ([238a8e9](https://github.com/rivet-gg/rivet/commit/238a8e98d5dd561fd442866f59862777e0e68b56))
* fix signal history divergence ([#1115](https://github.com/rivet-gg/rivet/issues/1115)) ([3cbfc1b](https://github.com/rivet-gg/rivet/commit/3cbfc1b593bcfebf4f3acee261926ac6ab613531))
* **job-run:** delete second allocation immediately ([#1104](https://github.com/rivet-gg/rivet/issues/1104)) ([78b73fd](https://github.com/rivet-gg/rivet/commit/78b73fd30b56f192fd23d07f7b57475a88a2f5e2))
* **nomad:** readd allocation metrics ([#1109](https://github.com/rivet-gg/rivet/issues/1109)) ([600d4fb](https://github.com/rivet-gg/rivet/commit/600d4fb6df38bfd123045bf7cd35182b2b37342f))
* update api endpoint names ([#1080](https://github.com/rivet-gg/rivet/issues/1080)) ([33e780d](https://github.com/rivet-gg/rivet/commit/33e780dacad902e0041cad7c0881d0bf308b61fe))
* **workflows:** add retry to internal sql queries ([#1112](https://github.com/rivet-gg/rivet/issues/1112)) ([ef010d0](https://github.com/rivet-gg/rivet/commit/ef010d089652693985efbda45e59421dff698d78))
* **workflows:** implement backoff for timeouts ([#1111](https://github.com/rivet-gg/rivet/issues/1111)) ([6659b34](https://github.com/rivet-gg/rivet/commit/6659b34f41e9e0e1f31b21ffd45485cfba4d6e09))


### Chores

* **main:** release 24.5.0 ([#1103](https://github.com/rivet-gg/rivet/issues/1103)) ([7652421](https://github.com/rivet-gg/rivet/commit/7652421cb2083de84413c1c2fccb1ac0f83ff118))
* release 24.5.0 ([1657c7c](https://github.com/rivet-gg/rivet/commit/1657c7c233e73c83c70443c2e205c119ef6ee34d))
* release 24.5.1 ([12f7ee9](https://github.com/rivet-gg/rivet/commit/12f7ee9e8f02c91229657292c580f4811a984518))
* update all uses of workflows to new syntax ([#1108](https://github.com/rivet-gg/rivet/issues/1108)) ([0079be8](https://github.com/rivet-gg/rivet/commit/0079be80559487bbbe2c1b2bd52e797ac3e0cd05))
* **workflows:** clean up internal contexts ([#1107](https://github.com/rivet-gg/rivet/issues/1107)) ([2148f9e](https://github.com/rivet-gg/rivet/commit/2148f9e4d5846eb3294dc78786b1629b8dab2110))

## [24.5.0](https://github.com/rivet-gg/rivet/compare/v24.4.1...v24.5.0) (2024-08-27)


### Features

* add json cache ([#939](https://github.com/rivet-gg/rivet/issues/939)) ([7c2897a](https://github.com/rivet-gg/rivet/commit/7c2897a1b5bb9cca714fa0ed915554b67fb5ebf7))
* add ready_ts to servers endpoint ([#1006](https://github.com/rivet-gg/rivet/issues/1006)) ([8b44a7c](https://github.com/rivet-gg/rivet/commit/8b44a7c3617f6ef578b2238771a218811230f432))
* add server logs endpoint ([#1005](https://github.com/rivet-gg/rivet/issues/1005)) ([a23073b](https://github.com/rivet-gg/rivet/commit/a23073b5b25a9cecc8730a91005b3121e1f94ee8))
* **better_uptime:** allow disabling notifications ([#923](https://github.com/rivet-gg/rivet/issues/923)) ([7eb12b0](https://github.com/rivet-gg/rivet/commit/7eb12b0a8898a37affc5d4c23c01ae5c75d254ae))
* **bolt:** add k9s to nix-shell ([#903](https://github.com/rivet-gg/rivet/issues/903)) ([7668942](https://github.com/rivet-gg/rivet/commit/7668942cbeaad0974bef8df2e2be592ea9ea073b))
* **bolt:** add lost servers list and prune commands ([#1096](https://github.com/rivet-gg/rivet/issues/1096)) ([0480702](https://github.com/rivet-gg/rivet/commit/04807026edb5da025177f5345189cebeb42e4c91))
* **bolt:** build svcs as docker containers locally ([#945](https://github.com/rivet-gg/rivet/issues/945)) ([11f4258](https://github.com/rivet-gg/rivet/commit/11f425878ffad357d4a1da3bfef7cd07ca8d98f4))
* **bolt:** run tests in containers ([#947](https://github.com/rivet-gg/rivet/issues/947)) ([08a53e3](https://github.com/rivet-gg/rivet/commit/08a53e353df3678e333f4e95c55275e2fa6dfcf5))
* **clusters:** add toggle for prebakes ([#932](https://github.com/rivet-gg/rivet/issues/932)) ([09890e5](https://github.com/rivet-gg/rivet/commit/09890e5e7783877d2c6bb2681a39601b1bf9e4fa))
* **clusters:** convert clusters to new workflow system ([#974](https://github.com/rivet-gg/rivet/issues/974)) ([0c5558b](https://github.com/rivet-gg/rivet/commit/0c5558b27e22007cec0bb547d20d4c30c96aa98f))
* **clusters:** gg monitor for better uptime ([#921](https://github.com/rivet-gg/rivet/issues/921)) ([152c55b](https://github.com/rivet-gg/rivet/commit/152c55b28a2a41fb57115bc706e85b91bf09642e))
* combine ops and workers into one svc type ([#957](https://github.com/rivet-gg/rivet/issues/957)) ([774da5c](https://github.com/rivet-gg/rivet/commit/774da5c46be49cd284379d682c1b2d31cec7c540))
* **ds:** add datacenters endpoint ([#1065](https://github.com/rivet-gg/rivet/issues/1065)) ([32d448e](https://github.com/rivet-gg/rivet/commit/32d448e55ca9013442023ebd243d963761f57aef))
* **ds:** add server create failed message ([#1068](https://github.com/rivet-gg/rivet/issues/1068)) ([82daf2d](https://github.com/rivet-gg/rivet/commit/82daf2db2c4cfffb8cb49ec7a1642e70b86b11f6))
* **ds:** rewrite dynamic servers on workflows ([#1060](https://github.com/rivet-gg/rivet/issues/1060)) ([c9b5578](https://github.com/rivet-gg/rivet/commit/c9b5578336588f01de4451ed56751c99db0ca82e))
* **infra:** auto-create dev tunnel & public ip ([#979](https://github.com/rivet-gg/rivet/issues/979)) ([0d82155](https://github.com/rivet-gg/rivet/commit/0d821554ab4b7f072887f5b88a857c359b1d51ce))
* **infra:** enable configuring min & max cockroach pool conns ([#922](https://github.com/rivet-gg/rivet/issues/922)) ([e8e7255](https://github.com/rivet-gg/rivet/commit/e8e725538d53c5e317163f5ffad81ca572c871be))
* **runtime:** switch from json to logfmt ([#984](https://github.com/rivet-gg/rivet/issues/984)) ([10a0e6c](https://github.com/rivet-gg/rivet/commit/10a0e6cd6134554dfd150b34e808a2bb386ea4da))
* **svc:** add servers create endpoint ([#740](https://github.com/rivet-gg/rivet/issues/740)) ([77f1b3f](https://github.com/rivet-gg/rivet/commit/77f1b3f1f37199553a20f53917638b7e0e68ded5))
* update billing to use tiers ([#900](https://github.com/rivet-gg/rivet/issues/900)) ([918038a](https://github.com/rivet-gg/rivet/commit/918038a02a995f7eea73910181bc1adf61cac0ff))
* **workflows, clusters:** add workflow backfill service ([#1000](https://github.com/rivet-gg/rivet/issues/1000)) ([e69b767](https://github.com/rivet-gg/rivet/commit/e69b76791b7b59b6f7599a6110ff288d16e3652f))
* **workflows:** add api ctx for workflows ([#865](https://github.com/rivet-gg/rivet/issues/865)) ([1a468d3](https://github.com/rivet-gg/rivet/commit/1a468d3cd62c902c927b482e79c1469091c8f5f9))
* **workflows:** add loops ([#1001](https://github.com/rivet-gg/rivet/issues/1001)) ([272a09d](https://github.com/rivet-gg/rivet/commit/272a09d627614e86a71483183dd82c64e180b277))
* **workflows:** add message and signal history ([#987](https://github.com/rivet-gg/rivet/issues/987)) ([0003acc](https://github.com/rivet-gg/rivet/commit/0003acc30b9719691a6c29b6cff0f1dfc6283b8a))
* **workflows:** add messages ([#977](https://github.com/rivet-gg/rivet/issues/977)) ([38c1171](https://github.com/rivet-gg/rivet/commit/38c1171caea473eca5c8fadda662c67121e4a90a))
* **workflows:** add metrics ([#1008](https://github.com/rivet-gg/rivet/issues/1008)) ([a4837e2](https://github.com/rivet-gg/rivet/commit/a4837e261287b65186200674b4d621e1fb3fe2b1))
* **workflows:** add nats worker wake ([#1039](https://github.com/rivet-gg/rivet/issues/1039)) ([1fc72f1](https://github.com/rivet-gg/rivet/commit/1fc72f150f6c64cae08ecfa9a5a1e86b8974efd6))
* **workflows:** add observe workflows fn ([#901](https://github.com/rivet-gg/rivet/issues/901)) ([22a1ebd](https://github.com/rivet-gg/rivet/commit/22a1ebd5f27926ca5c6eaeddd8fa2818e0c5cf3d))
* **workflows:** add operations service type ([#898](https://github.com/rivet-gg/rivet/issues/898)) ([0a0d377](https://github.com/rivet-gg/rivet/commit/0a0d377a7bc2cfda92848beae454b596cdd4bc2b))
* **workflows:** add sleep fn ([#1077](https://github.com/rivet-gg/rivet/issues/1077)) ([c477ba9](https://github.com/rivet-gg/rivet/commit/c477ba942a84b0904b4a4facf96ce73c7813d1e7))
* **workflows:** add tags ([#956](https://github.com/rivet-gg/rivet/issues/956)) ([36494eb](https://github.com/rivet-gg/rivet/commit/36494ebd826477ff9a5732a37e290f208a60e3d1))
* **workflows:** allow changing tags in workflow ([#962](https://github.com/rivet-gg/rivet/issues/962)) ([01ecf86](https://github.com/rivet-gg/rivet/commit/01ecf860783ad06fef5411722fb8ea217b23b620))
* **workflows:** implement retry backoff for activity errors ([#999](https://github.com/rivet-gg/rivet/issues/999)) ([6e8560e](https://github.com/rivet-gg/rivet/commit/6e8560ede591c475212eb336933a307e7541721e))


### Bug Fixes

* add ip whitelist to tunnels ([#930](https://github.com/rivet-gg/rivet/issues/930)) ([88ce4b3](https://github.com/rivet-gg/rivet/commit/88ce4b3313a812d2ef80e7db13eb371c05bc25c3))
* add players and servers db indexes ([#960](https://github.com/rivet-gg/rivet/issues/960)) ([53dc398](https://github.com/rivet-gg/rivet/commit/53dc3981f3bc53bb7d2dbe9ccc73afdeccef5318))
* add priority class to nats ([#1019](https://github.com/rivet-gg/rivet/issues/1019)) ([954d864](https://github.com/rivet-gg/rivet/commit/954d864a903574a419d8c896c6e0f64c9c0ebcf8))
* **api:** move cors verification to endpoint level ([#1094](https://github.com/rivet-gg/rivet/issues/1094)) ([4a4b4fe](https://github.com/rivet-gg/rivet/commit/4a4b4feb94fca4ddc1a8c06963653b70e91138d9))
* backfill script, crdb usage type ([#1089](https://github.com/rivet-gg/rivet/issues/1089)) ([ad0a260](https://github.com/rivet-gg/rivet/commit/ad0a260ea4de6d42f368dc129011fea1d6ff73d1))
* **better_uptime:** handle null verify_ssl ([#950](https://github.com/rivet-gg/rivet/issues/950)) ([e9d8edb](https://github.com/rivet-gg/rivet/commit/e9d8edbf220690788cb207e8ecb422902f42079c))
* **bolt:** correctly hash untracked files ([#1047](https://github.com/rivet-gg/rivet/issues/1047)) ([2b885e5](https://github.com/rivet-gg/rivet/commit/2b885e536f6f4906d1ae875ce2708871f229415b))
* **bolt:** exclude volumes when using native docker builder ([#969](https://github.com/rivet-gg/rivet/issues/969)) ([8ac0a55](https://github.com/rivet-gg/rivet/commit/8ac0a55776c2a648527d9f9562ecc2077f03f3a4))
* **bolt:** explicitly handle no nomad leader error ([#971](https://github.com/rivet-gg/rivet/issues/971)) ([20822fc](https://github.com/rivet-gg/rivet/commit/20822fcc6f780f298f472dc2871670ababf06c10))
* **bolt:** update opengb -&gt; backend env var name ([#1058](https://github.com/rivet-gg/rivet/issues/1058)) ([4250808](https://github.com/rivet-gg/rivet/commit/425080859e102e1af61f176e9f8290a5a0ca6e77))
* **bolt:** validate hub regex in ns config ([#1093](https://github.com/rivet-gg/rivet/issues/1093)) ([b2d5cca](https://github.com/rivet-gg/rivet/commit/b2d5ccae34172a5f887f9740e7bef2ef6f51c942))
* **cache:** mixed values in Cache::fetch_all ([#927](https://github.com/rivet-gg/rivet/issues/927)) ([d69a072](https://github.com/rivet-gg/rivet/commit/d69a0727019fa512ee686993ab7344168d44e006))
* **captcha:** sanitize form body ([#1098](https://github.com/rivet-gg/rivet/issues/1098)) ([9b56efc](https://github.com/rivet-gg/rivet/commit/9b56efca740500711966d9599614046fea4bd867))
* **chirp:** write message tail when history is disabled ([#997](https://github.com/rivet-gg/rivet/issues/997)) ([9f377ba](https://github.com/rivet-gg/rivet/commit/9f377bacf49979f65154f2a370844e8fca263890))
* **cloud:** add clean timeout for matchmaker logs ([#942](https://github.com/rivet-gg/rivet/issues/942)) ([a395e3f](https://github.com/rivet-gg/rivet/commit/a395e3f0e432b10d670a8cef4a488c125bb177d0))
* **cluster:** dc-get column mismatch ([#958](https://github.com/rivet-gg/rivet/issues/958)) ([53e276a](https://github.com/rivet-gg/rivet/commit/53e276abcd403b6460e06da7fada6ee92165f3fd))
* **cluster:** dns creation ([#1066](https://github.com/rivet-gg/rivet/issues/1066)) ([1ef72e6](https://github.com/rivet-gg/rivet/commit/1ef72e6f544efcf0b53b0c1ceacca3180e748d2a))
* **clusters:** add network_out metrics for hardware ([#1016](https://github.com/rivet-gg/rivet/issues/1016)) ([30d15c3](https://github.com/rivet-gg/rivet/commit/30d15c32f5a9000877be17d1aba95a12af959057))
* **clusters:** backfill json columns ([#1015](https://github.com/rivet-gg/rivet/issues/1015)) ([2292103](https://github.com/rivet-gg/rivet/commit/229210306e72dbdd7251ecdfdbba5add03693689))
* **clusters:** continue provisioning a server even when marked for deletion ([#924](https://github.com/rivet-gg/rivet/issues/924)) ([8b551f4](https://github.com/rivet-gg/rivet/commit/8b551f4ebbb1f1bb20612bc840fa9953fb684c84))
* **clusters:** dont delete servers immediately with linode ([#1040](https://github.com/rivet-gg/rivet/issues/1040)) ([6142837](https://github.com/rivet-gg/rivet/commit/61428379ef01aaf29f59bd52c044f748108d8aaa))
* **clusters:** fix backfill signal names ([#1086](https://github.com/rivet-gg/rivet/issues/1086)) ([2c8ae1c](https://github.com/rivet-gg/rivet/commit/2c8ae1c7fbc056fe838f90ee4b29f70d26123d63))
* **clusters:** fix dc scale job downscale logic, prebake disk waiting ([#1078](https://github.com/rivet-gg/rivet/issues/1078)) ([bda60e0](https://github.com/rivet-gg/rivet/commit/bda60e0ed0fbcf7b81a264c659ca299f7ec46a14))
* **clusters:** fix dns and unrecoverable error bugs ([#1083](https://github.com/rivet-gg/rivet/issues/1083)) ([273e5a3](https://github.com/rivet-gg/rivet/commit/273e5a332f9ce82d5c46c17d08916a0b6b19b5ef))
* **clusters:** fix linode cleanup logic ([#1034](https://github.com/rivet-gg/rivet/issues/1034)) ([f7d021c](https://github.com/rivet-gg/rivet/commit/f7d021ccecdd6d25966126e0bed91c58ec5179ee))
* **clusters:** fix linode-gc query ([#1063](https://github.com/rivet-gg/rivet/issues/1063)) ([eb0223c](https://github.com/rivet-gg/rivet/commit/eb0223cb1a479ab87116ab3b93e18f8f3569902e))
* **clusters:** fix tls renew query ([#1026](https://github.com/rivet-gg/rivet/issues/1026)) ([81a7b7a](https://github.com/rivet-gg/rivet/commit/81a7b7a58273d0021ca0e366bf42de107c5a5e80))
* **clusters:** fix trafficserver run dir permissions on reboot ([#1021](https://github.com/rivet-gg/rivet/issues/1021)) ([746198b](https://github.com/rivet-gg/rivet/commit/746198bf84bbc636306eae9bedb8a8e51e5fad8f))
* **clusters:** fix vlan ip query ([#911](https://github.com/rivet-gg/rivet/issues/911)) ([0ab1ec9](https://github.com/rivet-gg/rivet/commit/0ab1ec9faba864b4ab527da8cd7ff89556ebfbcf))
* **cluster:** split up backfill query from schema change ([#1023](https://github.com/rivet-gg/rivet/issues/1023)) ([4987029](https://github.com/rivet-gg/rivet/commit/4987029cab47ed7f5f903b5ade1c4c03cae3ee7b))
* **clusters:** query vlan ips per datacenter ([#961](https://github.com/rivet-gg/rivet/issues/961)) ([c2a7e3f](https://github.com/rivet-gg/rivet/commit/c2a7e3f707346ac5d0bc27a8d72a65066500273f))
* **clusters:** resolve ip by create ts ([#1037](https://github.com/rivet-gg/rivet/issues/1037)) ([7033c6e](https://github.com/rivet-gg/rivet/commit/7033c6e4e68cf8154acf7fea08644b37dd67d58b))
* **clusters:** run scale workflow instead of signal ([#1041](https://github.com/rivet-gg/rivet/issues/1041)) ([cbe6f89](https://github.com/rivet-gg/rivet/commit/cbe6f89e974ea81ab84d6c0876414bf392b8f546))
* **clusters:** update pools in dc-update ([#959](https://github.com/rivet-gg/rivet/issues/959)) ([9b31345](https://github.com/rivet-gg/rivet/commit/9b3134583cd9063f26861d5ec275687544b244a6))
* disable job migrations and reschedules ([#1017](https://github.com/rivet-gg/rivet/issues/1017)) ([91e869d](https://github.com/rivet-gg/rivet/commit/91e869d880aad745afcbf09a13141ec7b83ca6e3))
* **ds:** add back allocation signal ([#1069](https://github.com/rivet-gg/rivet/issues/1069)) ([453a19b](https://github.com/rivet-gg/rivet/commit/453a19b00d391085d93adb9fe16bc14e879f3f31))
* **ds:** cache traefik routes ([#1081](https://github.com/rivet-gg/rivet/issues/1081)) ([4b3a1ab](https://github.com/rivet-gg/rivet/commit/4b3a1ab3737916bbbfa82df305c37fcf5ac32c1d))
* **ds:** disable retries for nomad monitors ([#1091](https://github.com/rivet-gg/rivet/issues/1091)) ([945b5bb](https://github.com/rivet-gg/rivet/commit/945b5bbd244394c33484578d6b3520fbaec76230))
* **ds:** fix destroy query ([#1067](https://github.com/rivet-gg/rivet/issues/1067)) ([f67150f](https://github.com/rivet-gg/rivet/commit/f67150fa3581f6ba2def9aaa64e74325c840deb6))
* **ds:** fix ds tests, traefik, nomad monitors, job server drain ([#1085](https://github.com/rivet-gg/rivet/issues/1085)) ([d29bb3f](https://github.com/rivet-gg/rivet/commit/d29bb3fb1b0c9c0a1d978b0b8a5cc8dd8065df16))
* **ds:** fix logs ([#1074](https://github.com/rivet-gg/rivet/issues/1074)) ([21dbd6c](https://github.com/rivet-gg/rivet/commit/21dbd6c5109d1440b4ec0bcc510b439bb108f1e2))
* **ds:** fix server list & nomad monitor alloc plan queries ([#1071](https://github.com/rivet-gg/rivet/issues/1071)) ([eb0252c](https://github.com/rivet-gg/rivet/commit/eb0252cd7314c66ecefdc710e58f0bc64f74616b))
* **ds:** fix servers ([#1061](https://github.com/rivet-gg/rivet/issues/1061)) ([4e8185b](https://github.com/rivet-gg/rivet/commit/4e8185b5cb89cd9c609d2263c382cd00090b0a7d))
* **ds:** remove reschedule block ([#1082](https://github.com/rivet-gg/rivet/issues/1082)) ([4488c74](https://github.com/rivet-gg/rivet/commit/4488c74d5e41b9f6fe8282f2bfdc3095256e989a))
* **ds:** update auth endpoints ([#1044](https://github.com/rivet-gg/rivet/issues/1044)) ([11416c4](https://github.com/rivet-gg/rivet/commit/11416c44155d11248b2779eaa2d6f0ed295c8ed0))
* fix ds echo build ([#1032](https://github.com/rivet-gg/rivet/issues/1032)) ([ad1146e](https://github.com/rivet-gg/rivet/commit/ad1146e1fafe38c8861e650780a11392893ff9f0))
* **group:** require &gt; 1 use count on invites ([#985](https://github.com/rivet-gg/rivet/issues/985)) ([b37565a](https://github.com/rivet-gg/rivet/commit/b37565afc2283c77134a31c7619f0778d80e273f))
* **infra:** dynamically generate nomad server count in install script ([#981](https://github.com/rivet-gg/rivet/issues/981)) ([9c433d8](https://github.com/rivet-gg/rivet/commit/9c433d8e9dbc4e032c22c25dc4790301a887592e))
* **infra:** force linux/amd64 platform for building job-runner artifact ([#937](https://github.com/rivet-gg/rivet/issues/937)) ([1a32f90](https://github.com/rivet-gg/rivet/commit/1a32f90f2c60883e4a9f4a04d5d8a7ff6b445f93))
* **infra:** pass dynamic tunnel host port to cluster-server-install ([#980](https://github.com/rivet-gg/rivet/issues/980)) ([8be472f](https://github.com/rivet-gg/rivet/commit/8be472f2bce95fa8149f454ff92b98033fb44623))
* **infra:** re-run sshd config if dev tunnel machine recreated ([#978](https://github.com/rivet-gg/rivet/issues/978)) ([7fa5cff](https://github.com/rivet-gg/rivet/commit/7fa5cff9bb697090c6fab13228afe32f661f91c2))
* **infra:** remove dep on unused api_route secret ([#935](https://github.com/rivet-gg/rivet/issues/935)) ([7fca24b](https://github.com/rivet-gg/rivet/commit/7fca24b22b6e12ef8f361c5e25f29ae85cc629bc))
* **infra:** remove k8s_infra -&gt; cockroach_k8s circular dependency ([#936](https://github.com/rivet-gg/rivet/issues/936)) ([41b6cdb](https://github.com/rivet-gg/rivet/commit/41b6cdb7c82ce78245a322b0d44b0ef722be9f01))
* **infra:** resolve correct cockroachdb remote state ([#976](https://github.com/rivet-gg/rivet/issues/976)) ([8413349](https://github.com/rivet-gg/rivet/commit/8413349876f7e70d19b0b4a5497d571cf7bcef74))
* **ip:** cache ip queries ([#907](https://github.com/rivet-gg/rivet/issues/907)) ([c36d150](https://github.com/rivet-gg/rivet/commit/c36d15081096abea64679070aac50791d6c5ded7))
* **k3d:** mount host volume for PVCs ([#1018](https://github.com/rivet-gg/rivet/issues/1018)) ([07fae51](https://github.com/rivet-gg/rivet/commit/07fae51d196e6ea69bb5149c41fc84d6fcc9f3b0))
* loops and cache ([#1010](https://github.com/rivet-gg/rivet/issues/1010)) ([bccce31](https://github.com/rivet-gg/rivet/commit/bccce312dbb75bd8e49d40260e47a8d7d8b98bae))
* **mm:** clean up players from gc zset ([#914](https://github.com/rivet-gg/rivet/issues/914)) ([d6d05f6](https://github.com/rivet-gg/rivet/commit/d6d05f634d7fa2c57be97e41e808ad8742440982))
* **mm:** move runtime aggregate logic into query ([#966](https://github.com/rivet-gg/rivet/issues/966)) ([e545271](https://github.com/rivet-gg/rivet/commit/e545271d0c3a3fb00f19924c8c2618e4cc0c911c))
* **mm:** skip prewarming ats if no nodes booted ([#970](https://github.com/rivet-gg/rivet/issues/970)) ([61e9f14](https://github.com/rivet-gg/rivet/commit/61e9f14c046ff5fe395435e7919bb76e89ccef52))
* **opengb:** opengb. -&gt; backend. ([#919](https://github.com/rivet-gg/rivet/issues/919)) ([dfe5f8b](https://github.com/rivet-gg/rivet/commit/dfe5f8b17f017c43666d38edece48b886dfc8c28))
* remove trailing slash from endpoint ([#1012](https://github.com/rivet-gg/rivet/issues/1012)) ([b3bd44f](https://github.com/rivet-gg/rivet/commit/b3bd44fc461e07d60276ac9fe2ec6c0a01b5a105))
* revert hotfix ([#934](https://github.com/rivet-gg/rivet/issues/934)) ([115f02e](https://github.com/rivet-gg/rivet/commit/115f02e43db2c16b474dcc5dbee76f8d69ab1465))
* servers cors ([#1013](https://github.com/rivet-gg/rivet/issues/1013)) ([e46edfb](https://github.com/rivet-gg/rivet/commit/e46edfba82bff93b34adc2870f94635cbe26149e))
* **servers:** fix broken insert ([#1033](https://github.com/rivet-gg/rivet/issues/1033)) ([6e79bc7](https://github.com/rivet-gg/rivet/commit/6e79bc742a54d9451162a28ebb2677d882eb4cd8))
* **servers:** remove migrate block ([#1027](https://github.com/rivet-gg/rivet/issues/1027)) ([eab8ec4](https://github.com/rivet-gg/rivet/commit/eab8ec4032317b0b4e6a03dd45c0b7d1e84a95f1))
* **servers:** use correct timeout for sleeping ([#1076](https://github.com/rivet-gg/rivet/issues/1076)) ([0c58f83](https://github.com/rivet-gg/rivet/commit/0c58f835529da2cb0f8883425a6cbe824d12d759))
* **ssh:** force user for ssh commands ([#949](https://github.com/rivet-gg/rivet/issues/949)) ([ba02a16](https://github.com/rivet-gg/rivet/commit/ba02a166e7623a797fb329da63a858d82f8e8636))
* update cloudflare crate ([#1009](https://github.com/rivet-gg/rivet/issues/1009)) ([4e478f1](https://github.com/rivet-gg/rivet/commit/4e478f13f65e95cee7bf1b776a196263a962dc7c))
* workflow ts hotfix ([#933](https://github.com/rivet-gg/rivet/issues/933)) ([20796db](https://github.com/rivet-gg/rivet/commit/20796db7811683b6c3a16caaaa40fac865259b4c))
* **workflow:** fix sleep logic ([#1084](https://github.com/rivet-gg/rivet/issues/1084)) ([3202fdf](https://github.com/rivet-gg/rivet/commit/3202fdf64b24f1a62517d39659660222115beb8e))
* **workflows:** add back location bump to catch unrec ([#1087](https://github.com/rivet-gg/rivet/issues/1087)) ([4816533](https://github.com/rivet-gg/rivet/commit/48165331a3465371f74b84ba3fd9f021fc0a4538))
* **workflows:** add idx ([#1038](https://github.com/rivet-gg/rivet/issues/1038)) ([d825483](https://github.com/rivet-gg/rivet/commit/d825483be4dabb1f9641261cb994087c9e6f5bf3))
* **workflows:** add limit to pulling workflows ([#1020](https://github.com/rivet-gg/rivet/issues/1020)) ([6766ea0](https://github.com/rivet-gg/rivet/commit/6766ea08f66824f73a51782c21c133998ece07e1))
* **workflows:** add sql retries, improve history diverged errors ([#995](https://github.com/rivet-gg/rivet/issues/995)) ([9b0724f](https://github.com/rivet-gg/rivet/commit/9b0724f26d8b614fdea836b65849d9334cb9ed5e))
* **workflows:** add ts dt ([#943](https://github.com/rivet-gg/rivet/issues/943)) ([1b362fd](https://github.com/rivet-gg/rivet/commit/1b362fd83141328d4da0b3528a9691bdd14c5878))
* **workflows:** dont delete signal rows ([#965](https://github.com/rivet-gg/rivet/issues/965)) ([be67080](https://github.com/rivet-gg/rivet/commit/be670808bf2c130d1e077051a15d620dd12462cd))
* **workflows:** fix backfill ([#1025](https://github.com/rivet-gg/rivet/issues/1025)) ([6f7c94c](https://github.com/rivet-gg/rivet/commit/6f7c94c6bf63f48b2674787a5f1e869197043a13))
* **workflows:** fix docs on macros ([#1075](https://github.com/rivet-gg/rivet/issues/1075)) ([1175ae5](https://github.com/rivet-gg/rivet/commit/1175ae5f1dfd924f033c7c26c7bdc0e1896b628b))
* **workflows:** fix gc, event history graph, internal naming ([#963](https://github.com/rivet-gg/rivet/issues/963)) ([8b97b32](https://github.com/rivet-gg/rivet/commit/8b97b325a0c188b32033f73fde4ee67c67405e3c))
* **workflows:** fix invalid error wrapping ([#1092](https://github.com/rivet-gg/rivet/issues/1092)) ([7014d1b](https://github.com/rivet-gg/rivet/commit/7014d1bc24c93e52eef24519a42d4a1bb105ac0d))
* **workflows:** fix invalid event history graph ([#996](https://github.com/rivet-gg/rivet/issues/996)) ([fe2c38e](https://github.com/rivet-gg/rivet/commit/fe2c38ec19133b6613c404076039aaca74280634))
* **workflows:** fix listening traits ([#988](https://github.com/rivet-gg/rivet/issues/988)) ([0e56121](https://github.com/rivet-gg/rivet/commit/0e56121145d89d1856bc0528de9e507e2687cec9))
* **workflows:** fix loops queries ([#1042](https://github.com/rivet-gg/rivet/issues/1042)) ([63a7601](https://github.com/rivet-gg/rivet/commit/63a76013a4c6958be14cc16d0b0500e9a5908ea7))
* **workflows:** increase metrics publish interval ([#1050](https://github.com/rivet-gg/rivet/issues/1050)) ([b46300c](https://github.com/rivet-gg/rivet/commit/b46300caaa41e96da54d1d79df18cd6a68405f27))
* **workflows:** rename signals, improve failure handling for server install ([#1043](https://github.com/rivet-gg/rivet/issues/1043)) ([40cb84a](https://github.com/rivet-gg/rivet/commit/40cb84a9288bddb65f72f5fca86bb1391bfee9cd))
* **workflows:** Throw errors for duplicate workflows ([#1011](https://github.com/rivet-gg/rivet/issues/1011)) ([53c3aeb](https://github.com/rivet-gg/rivet/commit/53c3aebbec4ecbc58a24f45579f7c1220d7b2da1))


### Chores

* add build get endpoint ([#1046](https://github.com/rivet-gg/rivet/issues/1046)) ([e4f03fb](https://github.com/rivet-gg/rivet/commit/e4f03fbbcd52db1dafcd625649b2bceee7d7103d))
* add game id to server endpoints ([#1014](https://github.com/rivet-gg/rivet/issues/1014)) ([31f586f](https://github.com/rivet-gg/rivet/commit/31f586f5df2240c8cb688fd56013b8f0473afb54))
* add historical server query ([#1056](https://github.com/rivet-gg/rivet/issues/1056)) ([c3d7c96](https://github.com/rivet-gg/rivet/commit/c3d7c966724034ae89ad0ac4c233f5a36dc64d80))
* add lines to provisioning metrics ([#912](https://github.com/rivet-gg/rivet/issues/912)) ([d0371e0](https://github.com/rivet-gg/rivet/commit/d0371e00d9f05eecc7b6ec03e95061fa8c3dea49))
* add sqlx max connection timeout jitter ([#916](https://github.com/rivet-gg/rivet/issues/916)) ([4513a1f](https://github.com/rivet-gg/rivet/commit/4513a1fcadedcf7f4f3ca2609558f24a11c438d4))
* archive old linode servers table ([#1052](https://github.com/rivet-gg/rivet/issues/1052)) ([f6126f6](https://github.com/rivet-gg/rivet/commit/f6126f6762b799b7114ad72d928c3052230e38f5))
* **bolt:** add color to cargo build with docker ([#1035](https://github.com/rivet-gg/rivet/issues/1035)) ([7c324e5](https://github.com/rivet-gg/rivet/commit/7c324e5cfcfef929bc08537e1b13f226f0c4f4b8))
* **bolt:** update lockfile ([#1029](https://github.com/rivet-gg/rivet/issues/1029)) ([2140c0a](https://github.com/rivet-gg/rivet/commit/2140c0ac59f1b27366792e5a88315e23580ee6c1))
* **bolt:** upgrade rust to 1.80.0 ([#1028](https://github.com/rivet-gg/rivet/issues/1028)) ([44f6aa7](https://github.com/rivet-gg/rivet/commit/44f6aa76206faecfab59f1b71c6c350476babce0))
* **build:** add patching build tags ([#1048](https://github.com/rivet-gg/rivet/issues/1048)) ([812b7e2](https://github.com/rivet-gg/rivet/commit/812b7e231ddfc5f5d69af2756ea4ad10ffe1bef6))
* cache mm-config-version-get ([#913](https://github.com/rivet-gg/rivet/issues/913)) ([3b24383](https://github.com/rivet-gg/rivet/commit/3b2438338307e91d39117d7ea13592036163ff9d))
* clean up fern naming ([#1045](https://github.com/rivet-gg/rivet/issues/1045)) ([f4c13a8](https://github.com/rivet-gg/rivet/commit/f4c13a81725883181576570de708ba3a2600a86e))
* cleanup runtime aggregate op ([#902](https://github.com/rivet-gg/rivet/issues/902)) ([538d9b8](https://github.com/rivet-gg/rivet/commit/538d9b811ecb76ad95d201754745ed0c82224063))
* **cloud:** update default version format to not use special characters ([#1003](https://github.com/rivet-gg/rivet/issues/1003)) ([accb1d8](https://github.com/rivet-gg/rivet/commit/accb1d825fb433cde816338a954426da55015988))
* **cluster:** cache datacenter-get and datacenter-location-get ([#908](https://github.com/rivet-gg/rivet/issues/908)) ([8863a8b](https://github.com/rivet-gg/rivet/commit/8863a8b075e0cd31ed6a349e655559efd4da4427))
* **clusters:** remove git as a dependency for cluster util ([#931](https://github.com/rivet-gg/rivet/issues/931)) ([7c7eec3](https://github.com/rivet-gg/rivet/commit/7c7eec38fb617887d57e35458df32fe36e1ca55a))
* **ds:** fix started_at server conversion ([#1073](https://github.com/rivet-gg/rivet/issues/1073)) ([ec498fb](https://github.com/rivet-gg/rivet/commit/ec498fb4a26564326d8f3185790890d3767e3153))
* **ds:** split up destroy wf + add progress msg ([#1072](https://github.com/rivet-gg/rivet/issues/1072)) ([fb3168b](https://github.com/rivet-gg/rivet/commit/fb3168b11b9036286269a7c71139ee515e60a035))
* **fern:** update fern ([#1022](https://github.com/rivet-gg/rivet/issues/1022)) ([e6fe279](https://github.com/rivet-gg/rivet/commit/e6fe2795489653d2a328417f5b72b65dbcde7a33))
* fix dynamic servers merge ([#1007](https://github.com/rivet-gg/rivet/issues/1007)) ([07c4a75](https://github.com/rivet-gg/rivet/commit/07c4a75741892ff1df1c227ce1c4da2affbd16d1))
* fix monolith worker out of date ([#1055](https://github.com/rivet-gg/rivet/issues/1055)) ([387ee6b](https://github.com/rivet-gg/rivet/commit/387ee6b5149226c81c03472d110600691edcb430))
* group better uptime monitors ([#972](https://github.com/rivet-gg/rivet/issues/972)) ([f57ba69](https://github.com/rivet-gg/rivet/commit/f57ba693218065966945914649a6a684e9fad1fe))
* handle game version configs with bad proto migrations ([#926](https://github.com/rivet-gg/rivet/issues/926)) ([853cf06](https://github.com/rivet-gg/rivet/commit/853cf068d920e142a25994b7a4f824ecc05e762f))
* increase sql conn acquire rate limits ([#915](https://github.com/rivet-gg/rivet/issues/915)) ([deca712](https://github.com/rivet-gg/rivet/commit/deca712ddecf91fb3713c31669de4aaf6ca88109))
* increase ttl of public tokens ([#905](https://github.com/rivet-gg/rivet/issues/905)) ([93e705c](https://github.com/rivet-gg/rivet/commit/93e705c9a8373e52b7d4f5f3d6f37058daa330c7))
* increase workflow tick interval ([#941](https://github.com/rivet-gg/rivet/issues/941)) ([fb75556](https://github.com/rivet-gg/rivet/commit/fb75556fd902bbdcd558e1dc6a326e32607b1d53))
* **infra:** pin k3d image version ([#975](https://github.com/rivet-gg/rivet/issues/975)) ([088e05e](https://github.com/rivet-gg/rivet/commit/088e05e4ffa9724b437a2b043631ca036cfcb123))
* **k3d:** disable volumes if using use_local_repo ([#954](https://github.com/rivet-gg/rivet/issues/954)) ([c375325](https://github.com/rivet-gg/rivet/commit/c3753254ed2cd066a5705f8d3a87eceb4c7c5b53))
* make logs query consistent with nanoseconds ([#862](https://github.com/rivet-gg/rivet/issues/862)) ([4ffab51](https://github.com/rivet-gg/rivet/commit/4ffab516e794ff2d53dcfe58bacce8199efe8b78))
* migrate from game service to env service tokens ([#1054](https://github.com/rivet-gg/rivet/issues/1054)) ([2bf6db2](https://github.com/rivet-gg/rivet/commit/2bf6db2c9de45d86926ab600dedc591713cc26fc))
* migrate servers to use envs ([#1053](https://github.com/rivet-gg/rivet/issues/1053)) ([6b50e9e](https://github.com/rivet-gg/rivet/commit/6b50e9e7bc7fbea870ba619b9fe029ffb03b2cb3))
* read job-runner from ats ([#968](https://github.com/rivet-gg/rivet/issues/968)) ([3fa0611](https://github.com/rivet-gg/rivet/commit/3fa0611b336f45c014d550d03876412e955936ff))
* remove duplicate smithy code ([#946](https://github.com/rivet-gg/rivet/issues/946)) ([7ebe1f1](https://github.com/rivet-gg/rivet/commit/7ebe1f1ea3c3622354f88637c475c4fb27f49070))
* remove servers webhook ([#1051](https://github.com/rivet-gg/rivet/issues/1051)) ([6c6282d](https://github.com/rivet-gg/rivet/commit/6c6282dba19dff87282b98b9378db695786ac8cc))
* rename lib/types -&gt; lib/types-proto ([#986](https://github.com/rivet-gg/rivet/issues/986)) ([c4d40af](https://github.com/rivet-gg/rivet/commit/c4d40afa1e357f3e5eedae5f2453ebba27c470bd))
* **tls:** remove unneeded acme registration ([#953](https://github.com/rivet-gg/rivet/issues/953)) ([9c2e884](https://github.com/rivet-gg/rivet/commit/9c2e884015f04210e642780b8bc1c396d0d7f26c))
* traffic-server forward script ([#909](https://github.com/rivet-gg/rivet/issues/909)) ([a3528db](https://github.com/rivet-gg/rivet/commit/a3528db07bfac20a68b845e3650c960ae960bad2))
* tweak pool opts ([#1002](https://github.com/rivet-gg/rivet/issues/1002)) ([74e36c0](https://github.com/rivet-gg/rivet/commit/74e36c04b61675eb6b6df164a70193458a17948e))
* tweak pool opts ([#1004](https://github.com/rivet-gg/rivet/issues/1004)) ([786829f](https://github.com/rivet-gg/rivet/commit/786829f65bf614c4d625a788cf184d377ae02787))
* update opengb -&gt; backend rename ([#1049](https://github.com/rivet-gg/rivet/issues/1049)) ([a5febc2](https://github.com/rivet-gg/rivet/commit/a5febc2e80bfbe863b980c3385d365f61a7cd72e))
* update opengb cf worker names ([#1064](https://github.com/rivet-gg/rivet/issues/1064)) ([904c024](https://github.com/rivet-gg/rivet/commit/904c02487438008917fba50c49a754dd77cf4539))
* update start_ts to be set when networking is ready ([#1062](https://github.com/rivet-gg/rivet/issues/1062)) ([22b3fec](https://github.com/rivet-gg/rivet/commit/22b3fecf0ffa8f1cf10661ebf930ff78fc83f269))
* update typescript sdk ([#1031](https://github.com/rivet-gg/rivet/issues/1031)) ([0e6d5fb](https://github.com/rivet-gg/rivet/commit/0e6d5fb505027e4011805a4a3c154025e25ba5b7))
* update workspace ([#1030](https://github.com/rivet-gg/rivet/issues/1030)) ([f738b17](https://github.com/rivet-gg/rivet/commit/f738b17f5a57be369e1eee0a4a4b4b1ec9dca259))
* **workflows:** add workflow name to logs ([#928](https://github.com/rivet-gg/rivet/issues/928)) ([a3b31e0](https://github.com/rivet-gg/rivet/commit/a3b31e0a2a6abd4771dce8466612f29f1725344e))
* **workflows:** clean up imports ([#998](https://github.com/rivet-gg/rivet/issues/998)) ([9498cab](https://github.com/rivet-gg/rivet/commit/9498cab0bf47eed52459f88f11026db771fcb2a2))
* **workflows:** clean up internals ([#899](https://github.com/rivet-gg/rivet/issues/899)) ([b840019](https://github.com/rivet-gg/rivet/commit/b84001926ef052f37cc3e1f59f50c953f9b8dfa9))
* **workflows:** remove foo pkg ([#964](https://github.com/rivet-gg/rivet/issues/964)) ([7165aed](https://github.com/rivet-gg/rivet/commit/7165aed53534869ff00841122e10b5e1e7ea2629))

## [24.4.1](https://github.com/rivet-gg/rivet/compare/v24.4.0...v24.4.1) (2024-06-06)


### Features

* add compat layer between old ctx and new workflows ([#788](https://github.com/rivet-gg/rivet/issues/788)) ([787971b](https://github.com/rivet-gg/rivet/commit/787971ba5ff44580e45bb228ff9ec00f854a9278))
* add ray ids to workflows, clean up types ([#787](https://github.com/rivet-gg/rivet/issues/787)) ([3072bdc](https://github.com/rivet-gg/rivet/commit/3072bdcd5ba98fff23b1d1577bf0d5ab22fc1482))
* add workflows ([#783](https://github.com/rivet-gg/rivet/issues/783)) ([378d528](https://github.com/rivet-gg/rivet/commit/378d5283a94db8581b4f01108bd9e50ea9320949))
* global error raw variant ([#784](https://github.com/rivet-gg/rivet/issues/784)) ([4b11578](https://github.com/rivet-gg/rivet/commit/4b11578119a2a1cb3847f705e2a57fa4b4490b95))
* run sub workflows in the same process ([#789](https://github.com/rivet-gg/rivet/issues/789)) ([717e096](https://github.com/rivet-gg/rivet/commit/717e0963ca13c277a70c1486fa9aead50e7377f6))
* **workflows:** add retries and timeouts ([#860](https://github.com/rivet-gg/rivet/issues/860)) ([cc0b893](https://github.com/rivet-gg/rivet/commit/cc0b893adb8804e8b2bde60cf5659d4ff15dcce8))
* **workflows:** add worker instance failover ([#854](https://github.com/rivet-gg/rivet/issues/854)) ([c5a32a3](https://github.com/rivet-gg/rivet/commit/c5a32a3805dfe4efab498709cda9f70e0bcf5ebf))


### Bug Fixes

* cast workflow errors to raw global errors ([#785](https://github.com/rivet-gg/rivet/issues/785)) ([c90d939](https://github.com/rivet-gg/rivet/commit/c90d9394abbe31d4b4dcd366e93491a3e5bde4a1))
* draining and tainted server grafana chart ([#855](https://github.com/rivet-gg/rivet/issues/855)) ([d0cdb38](https://github.com/rivet-gg/rivet/commit/d0cdb38b09063f87f889429ff1c5ba8213b19843))
* **mm:** add index for run_proxied_ports ([#868](https://github.com/rivet-gg/rivet/issues/868)) ([e0785e9](https://github.com/rivet-gg/rivet/commit/e0785e9635f5051863b9e9fcb240dfe446d52729))
* **mm:** call mm-lobby-cleanup from mm-gc even for preemptive lobbies without sql row ([#856](https://github.com/rivet-gg/rivet/issues/856)) ([5315a9a](https://github.com/rivet-gg/rivet/commit/5315a9a4e675ed24e7d2926b1ce07b6ecd213e61))
* **mm:** correctly handle lobby not found error if joining direclty to lobby id that doesn't exist ([#867](https://github.com/rivet-gg/rivet/issues/867)) ([af3513a](https://github.com/rivet-gg/rivet/commit/af3513a5947a99f83bbcb9866117dc3578ff0efb))
* **mm:** require specifying matchmaker config for new game versions ([#895](https://github.com/rivet-gg/rivet/issues/895)) ([92d86fd](https://github.com/rivet-gg/rivet/commit/92d86fd8f71cfe54ba1c1e28215060e256c0105f))
* **tls:** provision cloudflare cert pack if opengb enabled ([#869](https://github.com/rivet-gg/rivet/issues/869)) ([1dafa9e](https://github.com/rivet-gg/rivet/commit/1dafa9ea943466fbcd6a5a5ed601877f9e94697b))


### Chores

* **infra:** move cdn to api-traefik-provider ([#857](https://github.com/rivet-gg/rivet/issues/857)) ([9370e9e](https://github.com/rivet-gg/rivet/commit/9370e9ef5a5a79bbfa460bcabd7831f3c3755feb))
* release 24.4.1 ([30cc822](https://github.com/rivet-gg/rivet/commit/30cc822d3511651f96dbc9e9beda445bb00d728e))

## [24.4.0](https://github.com/rivet-gg/rivet/compare/v24.3.0...v24.4.0) (2024-06-04)


### ⚠ BREAKING CHANGES

* Cleanup API definitions, module imports ([#534](https://github.com/rivet-gg/rivet/issues/534))

### Features

* add 1password integration docs ([#595](https://github.com/rivet-gg/rivet/issues/595)) ([29045ea](https://github.com/rivet-gg/rivet/commit/29045ea1cb4a166f0806ede2968908fac78f59a0))
* Add cluster admin cli ([#644](https://github.com/rivet-gg/rivet/issues/644)) ([5b1de57](https://github.com/rivet-gg/rivet/commit/5b1de575a4a5d6146ac36841824d956ecc59427c))
* add crdb data source to grafana ([#732](https://github.com/rivet-gg/rivet/issues/732)) ([f22694f](https://github.com/rivet-gg/rivet/commit/f22694fec03b65d82ad38e2dc248d24d74b42b1b))
* add env update error ([#814](https://github.com/rivet-gg/rivet/issues/814)) ([48a5883](https://github.com/rivet-gg/rivet/commit/48a58836f0e7aa7494f08658c2e269f06f4e5a71))
* add hacky secondary ingress route for game lobbies ([#567](https://github.com/rivet-gg/rivet/issues/567)) ([8bb6bd6](https://github.com/rivet-gg/rivet/commit/8bb6bd64e98d670b8f444cdfce09b81e7093cf45))
* add internal api monolith ([#641](https://github.com/rivet-gg/rivet/issues/641)) ([f25ffe4](https://github.com/rivet-gg/rivet/commit/f25ffe4e9cacb4efed1722ed529c918c7cdbe85a))
* Add managed OpenGB ([#535](https://github.com/rivet-gg/rivet/issues/535)) ([9085d51](https://github.com/rivet-gg/rivet/commit/9085d511df3c72aef3a416abac502a159d50ae7b))
* add opengb to bootstrap ([#844](https://github.com/rivet-gg/rivet/issues/844)) ([ebd3c7b](https://github.com/rivet-gg/rivet/commit/ebd3c7bd6fdd93b0b9af7e3658c0636e23abaf57))
* add operation to list all clusters ([#717](https://github.com/rivet-gg/rivet/issues/717)) ([1f4b169](https://github.com/rivet-gg/rivet/commit/1f4b1699f9f1c7f262c1c62a7132b459755b5975))
* add patch method to router ([#744](https://github.com/rivet-gg/rivet/issues/744)) ([ed6596c](https://github.com/rivet-gg/rivet/commit/ed6596ca94eeb87511a33d62877d6dca10b72ecf))
* add pool filter to cluster dashboard ([#830](https://github.com/rivet-gg/rivet/issues/830)) ([5436461](https://github.com/rivet-gg/rivet/commit/5436461314e7813cfb5b5185fe891e926b12ea92))
* add provider api token to all linode calls ([#613](https://github.com/rivet-gg/rivet/issues/613)) ([3882047](https://github.com/rivet-gg/rivet/commit/3882047368856177a7ae9685bc672f936338bd75))
* add provisioning dashboard ([#733](https://github.com/rivet-gg/rivet/issues/733)) ([a1f9dcc](https://github.com/rivet-gg/rivet/commit/a1f9dcc5372ae3cdbfcdd47a72952a0e09ab9118))
* add ray id to error body ([#833](https://github.com/rivet-gg/rivet/issues/833)) ([c115d6f](https://github.com/rivet-gg/rivet/commit/c115d6f484e5a0ec5c6aed0f5177c3255f5ee27a))
* add region list/resolve per game ([#633](https://github.com/rivet-gg/rivet/issues/633)) ([92275d8](https://github.com/rivet-gg/rivet/commit/92275d8037da694f0e36ca1ca682bdf6af655980))
* Add script to run cargo clean ([#700](https://github.com/rivet-gg/rivet/issues/700)) ([0f653e2](https://github.com/rivet-gg/rivet/commit/0f653e2596dd6c1ee307ddcda9b6ebef7f93bff4))
* add toggle for load tests ([#583](https://github.com/rivet-gg/rivet/issues/583)) ([a78d682](https://github.com/rivet-gg/rivet/commit/a78d6826dcb0611275315edf42054cd8b9b36779))
* add vector http source ([#800](https://github.com/rivet-gg/rivet/issues/800)) ([f4f2734](https://github.com/rivet-gg/rivet/commit/f4f27343ea4b472eee3b1867aefe887b00d25b44))
* **api-admin:** add server destroy endpoint ([#838](https://github.com/rivet-gg/rivet/issues/838)) ([4ff616b](https://github.com/rivet-gg/rivet/commit/4ff616bff3c5a3e4618065dbbf424236da64aee3))
* **bolt:** list datacenter CLI command ([#728](https://github.com/rivet-gg/rivet/issues/728)) ([c4a88de](https://github.com/rivet-gg/rivet/commit/c4a88de2469cd56b5dc26eb6e9e949bd25c33643))
* **bolt:** update datacenters from CLI ([#727](https://github.com/rivet-gg/rivet/issues/727)) ([083cd19](https://github.com/rivet-gg/rivet/commit/083cd1909ab046017cac92b42bbea85a7ca6c99a))
* configurable drain ts per pool ([#684](https://github.com/rivet-gg/rivet/issues/684)) ([f88c457](https://github.com/rivet-gg/rivet/commit/f88c457b1e4fedce38a2b99fcff1c63601b111b6))
* dynamic TLS generation ([#635](https://github.com/rivet-gg/rivet/issues/635)) ([66e49dd](https://github.com/rivet-gg/rivet/commit/66e49dd6cdcfe1aad044d36fa85b1d331dcf3cb3))
* **grafana:** rivet logs dashboard ([#724](https://github.com/rivet-gg/rivet/issues/724)) ([9a43f3a](https://github.com/rivet-gg/rivet/commit/9a43f3ab32ec115bb53b7e936c6b4c0a50be2061))
* **infra:** add ability to provision dev tunnel ([#692](https://github.com/rivet-gg/rivet/issues/692)) ([659f8a1](https://github.com/rivet-gg/rivet/commit/659f8a1110835af93b13bd69adf995b85f31f565))
* **Infra:** Loops welcome email ([b2e4006](https://github.com/rivet-gg/rivet/commit/b2e4006af0569f0167d8cef47fa4c8dbdfec8163))
* **nix:** skip building bolt in nix with NIX_SKIP_BOLT ([#664](https://github.com/rivet-gg/rivet/issues/664)) ([8e16a94](https://github.com/rivet-gg/rivet/commit/8e16a944f878dec5d8be0b87c94ca66b881c679e))
* **svc:** resolve cluster name id op ([#751](https://github.com/rivet-gg/rivet/issues/751)) ([58200ec](https://github.com/rivet-gg/rivet/commit/58200ec4b6c8f7951c9fa5074f5ed2d6a2d9334c))


### Bug Fixes

* add last upload id ([#745](https://github.com/rivet-gg/rivet/issues/745)) ([d10d917](https://github.com/rivet-gg/rivet/commit/d10d917721df422c225388617f5220c8701764c2))
* add min count to autoscaler ([#826](https://github.com/rivet-gg/rivet/issues/826)) ([9fe12a1](https://github.com/rivet-gg/rivet/commit/9fe12a1f3949e864420e6b1944ea9a8818cb0d58))
* add patch to CORS ([#848](https://github.com/rivet-gg/rivet/issues/848)) ([09f3ddc](https://github.com/rivet-gg/rivet/commit/09f3ddc93ca5544317e71a98b607b018b9982205))
* add region to dns for path routing ([#574](https://github.com/rivet-gg/rivet/issues/574)) ([e10ad25](https://github.com/rivet-gg/rivet/commit/e10ad25f7ff54666faeb152d642b486ba64a4180))
* add transacitons ([#689](https://github.com/rivet-gg/rivet/issues/689)) ([f55b7e6](https://github.com/rivet-gg/rivet/commit/f55b7e694b15d2709bf6d98b92c9eb15c11ccd0b))
* add transactions and locks ([#696](https://github.com/rivet-gg/rivet/issues/696)) ([477ade5](https://github.com/rivet-gg/rivet/commit/477ade596ec38c806c83f9e86c694a9e985468b3))
* api admin hub endpoint is incorrect ([#660](https://github.com/rivet-gg/rivet/issues/660)) ([0aff347](https://github.com/rivet-gg/rivet/commit/0aff347c78c8c3a619e3335a0df7685f930d6a12))
* **api-status:** auto-delete lobby after testing connection ([#770](https://github.com/rivet-gg/rivet/issues/770)) ([9803f39](https://github.com/rivet-gg/rivet/commit/9803f39dd346e3f3ab8a73944b404cf4f3690856))
* **ats:** don't send requests to ats nodes without install_complete_ts ([#807](https://github.com/rivet-gg/rivet/issues/807)) ([618a429](https://github.com/rivet-gg/rivet/commit/618a42942d1de4617e5b6ca9867f11db46e08362))
* **bolt:** copy & install git in docker for cluster build.rs ([#769](https://github.com/rivet-gg/rivet/issues/769)) ([12bf1d4](https://github.com/rivet-gg/rivet/commit/12bf1d411a01d9d7f33a936270db1ba7487245c7))
* **bolt:** correctly check for existing env var ([#705](https://github.com/rivet-gg/rivet/issues/705)) ([ca4e48d](https://github.com/rivet-gg/rivet/commit/ca4e48d3ea732a3ff103aa68ba7306dd6a2bd7a2))
* **bolt:** dont fully parse config when pulling ([#816](https://github.com/rivet-gg/rivet/issues/816)) ([d22b08b](https://github.com/rivet-gg/rivet/commit/d22b08b2d2344c82e60c933824bf1f8108a94142))
* **bolt:** uncomment provisioning check ([#749](https://github.com/rivet-gg/rivet/issues/749)) ([f25bead](https://github.com/rivet-gg/rivet/commit/f25bead2ad1ba62e008d74da22a5afc2477b9ca7))
* **bolt:** update rust test package_id parsing ([#622](https://github.com/rivet-gg/rivet/issues/622)) ([3d987ab](https://github.com/rivet-gg/rivet/commit/3d987ab01c16927d1fccf17285df822af2bb5497))
* Change sdks linguist-vendored to linguist-generated ([#662](https://github.com/rivet-gg/rivet/issues/662)) ([602749f](https://github.com/rivet-gg/rivet/commit/602749f8aeaa2c9b6a8415155816c23dab4540bc))
* change test relative path ([#754](https://github.com/rivet-gg/rivet/issues/754)) ([daf1d07](https://github.com/rivet-gg/rivet/commit/daf1d07e69499008bb5c60d426ebc61cf6e049ff))
* check for draining before installing/creating dns ([#773](https://github.com/rivet-gg/rivet/issues/773)) ([cbe450b](https://github.com/rivet-gg/rivet/commit/cbe450bacbcd78e0f79c7c561d6899e695cecab6))
* **chirp:** add bypass for recursive messages ([#708](https://github.com/rivet-gg/rivet/issues/708)) ([566088f](https://github.com/rivet-gg/rivet/commit/566088fbef3230405de5dc961b7a5b26a67c6861))
* CI regression ([#713](https://github.com/rivet-gg/rivet/issues/713)) ([636f0d3](https://github.com/rivet-gg/rivet/commit/636f0d3d2715c112d7d78f8907b51e852843b07b))
* claims ([#672](https://github.com/rivet-gg/rivet/issues/672)) ([d61e290](https://github.com/rivet-gg/rivet/commit/d61e2908cc3f3cb5d2921efec954771870c1d03f))
* clean up nomad jobs per test ([#596](https://github.com/rivet-gg/rivet/issues/596)) ([6d7f0ee](https://github.com/rivet-gg/rivet/commit/6d7f0ee5338c516db9d9481c2213a18fed66005d))
* Cleanup API definitions, module imports ([#534](https://github.com/rivet-gg/rivet/issues/534)) ([0e0660a](https://github.com/rivet-gg/rivet/commit/0e0660a5145a80283f9d7ab76b9eda4d0683247e))
* **cluster:** delete dns record after failure to create ([#827](https://github.com/rivet-gg/rivet/issues/827)) ([35fc6fe](https://github.com/rivet-gg/rivet/commit/35fc6fedf175d0c04b58471384cf83554c09c5ec))
* **cluster:** don't taint servers that have cloud_destroy_ts ([#839](https://github.com/rivet-gg/rivet/issues/839)) ([e5256f1](https://github.com/rivet-gg/rivet/commit/e5256f1e4ee5466c0a961fe691df7dc9d369413f))
* **cluster:** gg dns records leak if server destroyed before install complete ([#842](https://github.com/rivet-gg/rivet/issues/842)) ([e63f242](https://github.com/rivet-gg/rivet/commit/e63f2422d7ac65c1bc29d0eb63ab72e7a1f1ce2d))
* **cluster:** handle failed tls issuing gracefully ([#825](https://github.com/rivet-gg/rivet/issues/825)) ([9aa424b](https://github.com/rivet-gg/rivet/commit/9aa424b349f22ee4acfeda6acd74733ce904877b))
* **cluter:** disable prebake images ([#813](https://github.com/rivet-gg/rivet/issues/813)) ([cdb6133](https://github.com/rivet-gg/rivet/commit/cdb61335c41e6147718213f8f61e0fa63f9f0c7e))
* contention bugs ([#707](https://github.com/rivet-gg/rivet/issues/707)) ([d8a5d33](https://github.com/rivet-gg/rivet/commit/d8a5d3342708b03648f66c3a39f4fd0c56b4fa2d))
* datacenter taint draining too soon, datacenter update not updating drain timeout ([#763](https://github.com/rivet-gg/rivet/issues/763)) ([55073a4](https://github.com/rivet-gg/rivet/commit/55073a42ccd1169fab29912dd6c51290897ad9b1))
* default build creation ([#582](https://github.com/rivet-gg/rivet/issues/582)) ([1ec0ba5](https://github.com/rivet-gg/rivet/commit/1ec0ba5faab39376c517e642cb349b04e3cd6872))
* delegate more funcionality to dc-scale ([#674](https://github.com/rivet-gg/rivet/issues/674)) ([a5be980](https://github.com/rivet-gg/rivet/commit/a5be9801e2e82f51c6d30108c656441d78b4acfd))
* **detect-secrets:** pin detect secrets version ([#786](https://github.com/rivet-gg/rivet/issues/786)) ([9db9d3c](https://github.com/rivet-gg/rivet/commit/9db9d3cf1f8550741048b813a2ae6a2be09ab5a4))
* docs ([#667](https://github.com/rivet-gg/rivet/issues/667)) ([c5b33fa](https://github.com/rivet-gg/rivet/commit/c5b33fa14b83e35cd48ec567dacf06ae81a6b989))
* encode query parameters in migrations ([#579](https://github.com/rivet-gg/rivet/issues/579)) ([17ba1d1](https://github.com/rivet-gg/rivet/commit/17ba1d1e0036138f351b4790ec96cf0d41049c94))
* expand prebake image variant system ([#628](https://github.com/rivet-gg/rivet/issues/628)) ([af41308](https://github.com/rivet-gg/rivet/commit/af4130897e971b4fada9868de918e93715e772f9))
* feature flag more tests ([#581](https://github.com/rivet-gg/rivet/issues/581)) ([be0e3e9](https://github.com/rivet-gg/rivet/commit/be0e3e9bfa288b6fdb6a94a74021e6b18605ae27))
* **fern:** remove dupe fern gen from bad merge ([#725](https://github.com/rivet-gg/rivet/issues/725)) ([982d388](https://github.com/rivet-gg/rivet/commit/982d388de780d35341e7a7f86172faf3f1b726a0))
* Fix nix build of bolt on macOS (Darwin) ([#589](https://github.com/rivet-gg/rivet/issues/589)) ([3343b06](https://github.com/rivet-gg/rivet/commit/3343b065c44a8885d66952be7e16eb23fa1f795c))
* fix user relationship test ([#616](https://github.com/rivet-gg/rivet/issues/616)) ([4edd90c](https://github.com/rivet-gg/rivet/commit/4edd90ce222331d8357bb9d72f9418f5fec3f9f6))
* force reload tls certs ([#736](https://github.com/rivet-gg/rivet/issues/736)) ([599cb8b](https://github.com/rivet-gg/rivet/commit/599cb8bb34479dcf1ef98041dade84e313501ed7))
* game guard ingress routes getting cobbled ([#569](https://github.com/rivet-gg/rivet/issues/569)) ([bd3a73f](https://github.com/rivet-gg/rivet/commit/bd3a73f6016ad4dae0c8c738293776ca5bdbc7ed))
* game, ip, and job tests ([#566](https://github.com/rivet-gg/rivet/issues/566)) ([1607c40](https://github.com/rivet-gg/rivet/commit/1607c407c3f19861dba16c51246e68c195542ea9))
* get all api tests passing or disabled ([#565](https://github.com/rivet-gg/rivet/issues/565)) ([431bfa5](https://github.com/rivet-gg/rivet/commit/431bfa59459f4369b63485fa4215756611150a8d))
* get mm tests working again with provisioning ([#711](https://github.com/rivet-gg/rivet/issues/711)) ([0b27dc2](https://github.com/rivet-gg/rivet/commit/0b27dc293f58c6fe7cb2e974fbfb377da56efcb9))
* get tests working with new target ([#737](https://github.com/rivet-gg/rivet/issues/737)) ([3d3e37a](https://github.com/rivet-gg/rivet/commit/3d3e37a630ffa6abac7b1f765a87a2df68c4d2c8))
* get todo tests working ([#573](https://github.com/rivet-gg/rivet/issues/573)) ([38ed2da](https://github.com/rivet-gg/rivet/commit/38ed2da1561f1572151b87994ba4c7a9f176be75))
* get upload tests working ([#572](https://github.com/rivet-gg/rivet/issues/572)) ([ace12d9](https://github.com/rivet-gg/rivet/commit/ace12d9a26313c40f2dac1ae21a20ea5a6e61b5a))
* gracfully delete secondary dns record ([#828](https://github.com/rivet-gg/rivet/issues/828)) ([94cc2ae](https://github.com/rivet-gg/rivet/commit/94cc2ae4c51d043bf049540ee4b1fec117a8340f))
* **grafana:** add back default prometheus dashboards ([#771](https://github.com/rivet-gg/rivet/issues/771)) ([30f41ee](https://github.com/rivet-gg/rivet/commit/30f41eeb7e56a0b4776c316f20adc8e8c978fd9c))
* **grafana:** fix circular dependency between grafana &lt;-&gt; cockroachdb_managed ([#760](https://github.com/rivet-gg/rivet/issues/760)) ([46e3bf0](https://github.com/rivet-gg/rivet/commit/46e3bf05bf78578cfa364331f1a6845689a8171c))
* **grafana:** fix pool_type query on cluster nomad panels ([#840](https://github.com/rivet-gg/rivet/issues/840)) ([d99d466](https://github.com/rivet-gg/rivet/commit/d99d466016942fd613fb260157d36b4d1073e211))
* hotfix check ci ([#719](https://github.com/rivet-gg/rivet/issues/719)) ([974b7f4](https://github.com/rivet-gg/rivet/commit/974b7f42dad3ee10f7095325a15293f0283901bb))
* increase default api-route resources for distributed ([#559](https://github.com/rivet-gg/rivet/issues/559)) ([dc6cd79](https://github.com/rivet-gg/rivet/commit/dc6cd79d254b76d326e3a09215e6853ee40df12d))
* **infra:** gg tls certs timer & precreate tls dir ([#812](https://github.com/rivet-gg/rivet/issues/812)) ([b4b707e](https://github.com/rivet-gg/rivet/commit/b4b707ec6863dfe297a19ef6a1b573436f88417d))
* **infra:** remove high cardinality prometheus metrics ([#835](https://github.com/rivet-gg/rivet/issues/835)) ([e554984](https://github.com/rivet-gg/rivet/commit/e5549844cd032a4164841376d998806894c2e68d))
* **infra:** upgrade karpenter to 0.32 & disable compaction ([#834](https://github.com/rivet-gg/rivet/issues/834)) ([0976245](https://github.com/rivet-gg/rivet/commit/09762453914a679a4a60fbbd18513762f31193f2))
* ip-info test ([#631](https://github.com/rivet-gg/rivet/issues/631)) ([5fc1e16](https://github.com/rivet-gg/rivet/commit/5fc1e169b55d41c822a533f2e0b1432a10be0e24))
* **job-run:** add index for run_meta_nomad.node_id ([#810](https://github.com/rivet-gg/rivet/issues/810)) ([4559152](https://github.com/rivet-gg/rivet/commit/4559152df1e0a5288b842bc6562970e211e7da5f))
* **job-run:** correctly clean up leaked proxied ports ([#832](https://github.com/rivet-gg/rivet/issues/832)) ([824936f](https://github.com/rivet-gg/rivet/commit/824936f0aed81e005c50376b6449f927e1861c54))
* **job-run:** don't write job proxied port if job already stopped ([#841](https://github.com/rivet-gg/rivet/issues/841)) ([4466d82](https://github.com/rivet-gg/rivet/commit/4466d823282df4f2d99e9afe99a09a0499214c3d))
* **job-run:** fix leaking jobs with wrong param order ([#815](https://github.com/rivet-gg/rivet/issues/815)) ([6350c72](https://github.com/rivet-gg/rivet/commit/6350c72f939a9f60cf6fc15c796c9553a150b42a))
* **job:** gc was not stopping jobs which failed to stop on nomad ([#617](https://github.com/rivet-gg/rivet/issues/617)) ([67ab5eb](https://github.com/rivet-gg/rivet/commit/67ab5eba8cd383b7f42764985c59d843e7754754))
* **k8s_infra:** resolve invalid tf types ([#742](https://github.com/rivet-gg/rivet/issues/742)) ([565b044](https://github.com/rivet-gg/rivet/commit/565b0440577ef33e2e30b5aef2793b8cad4dc6bf))
* leaked dns records ([#765](https://github.com/rivet-gg/rivet/issues/765)) ([163beaf](https://github.com/rivet-gg/rivet/commit/163beaf26609c8b89709da9e05971f8694bcc3a1))
* make default cluster opt in ([#632](https://github.com/rivet-gg/rivet/issues/632)) ([c98e6aa](https://github.com/rivet-gg/rivet/commit/c98e6aa066eff72d8f91ce4896ddfece4ae2586e))
* make nsfw check verbose error optional ([#746](https://github.com/rivet-gg/rivet/issues/746)) ([3fb5195](https://github.com/rivet-gg/rivet/commit/3fb5195b4ae9ef3c17ee1e8f1dd95c9428437a80))
* mm fixes ([#731](https://github.com/rivet-gg/rivet/issues/731)) ([c987736](https://github.com/rivet-gg/rivet/commit/c987736e9614301b9e9c5e9f918dd0917496044b))
* mm tests ([#570](https://github.com/rivet-gg/rivet/issues/570)) ([c99a410](https://github.com/rivet-gg/rivet/commit/c99a410f56a593d46b7212865a1ec4b0b605f0fb))
* **mm:** broken cache ([#806](https://github.com/rivet-gg/rivet/issues/806)) ([12ac484](https://github.com/rivet-gg/rivet/commit/12ac484e78eb32c881330c067230c6a99dba77c8))
* **mm:** only add to available spots if lobby is running ([#843](https://github.com/rivet-gg/rivet/issues/843)) ([9b15294](https://github.com/rivet-gg/rivet/commit/9b1529498cc5caef4f42329f5f404e4858d05259))
* move crdb user grants to post migration query ([#757](https://github.com/rivet-gg/rivet/issues/757)) ([fbb474d](https://github.com/rivet-gg/rivet/commit/fbb474d2de623572c981eebb07749ea8f0b58272))
* move grafana to its own helm chart ([#741](https://github.com/rivet-gg/rivet/issues/741)) ([1be990b](https://github.com/rivet-gg/rivet/commit/1be990b956c838c1efe4f6d5b6ad44a8c59e5f6d))
* node draining ([#721](https://github.com/rivet-gg/rivet/issues/721)) ([2432a40](https://github.com/rivet-gg/rivet/commit/2432a40d1ea3777ca9b64396a2d11852cbb4c9d4))
* **nomad:** increase storage size to recommended capacity ([#818](https://github.com/rivet-gg/rivet/issues/818)) ([9f78ba5](https://github.com/rivet-gg/rivet/commit/9f78ba5d12f9b317415673c92871985990a986cb))
* only generate path proxied port for https routes ([#587](https://github.com/rivet-gg/rivet/issues/587)) ([29985ce](https://github.com/rivet-gg/rivet/commit/29985cec4e7683bd1fca1d709fd3dd4b81b6401d))
* only select primary hostname in mm endpoints ([#577](https://github.com/rivet-gg/rivet/issues/577)) ([3d8e55d](https://github.com/rivet-gg/rivet/commit/3d8e55d00608508c410283f3e1c9f2bf371886e7))
* **opengb:** add dedicated error for neon projects exceeded ([#847](https://github.com/rivet-gg/rivet/issues/847)) ([95b7711](https://github.com/rivet-gg/rivet/commit/95b7711193309fbc2b5ebeecbf71a04d3f360fc7))
* pass tags to lobby create ([#619](https://github.com/rivet-gg/rivet/issues/619)) ([fd7d90c](https://github.com/rivet-gg/rivet/commit/fd7d90cb92f954adc90ad236f922915c6c19c900))
* patch signal endpoint with nomad client ([#712](https://github.com/rivet-gg/rivet/issues/712)) ([2891b0f](https://github.com/rivet-gg/rivet/commit/2891b0f09885170a6197da7b070de7444668383a))
* reenable better stack ([#669](https://github.com/rivet-gg/rivet/issues/669)) ([31d0e43](https://github.com/rivet-gg/rivet/commit/31d0e4343602fb69e4f0d77ec936ac75ac1728f6))
* remove /join regression ([#687](https://github.com/rivet-gg/rivet/issues/687)) ([0b4af4c](https://github.com/rivet-gg/rivet/commit/0b4af4c2989e0cc46c03fcdc92a154a6f01b9661))
* remove absolute path from http vector sink ([#851](https://github.com/rivet-gg/rivet/issues/851)) ([58c21fc](https://github.com/rivet-gg/rivet/commit/58c21fc017853e686d2279161ce61c7e0148235b))
* remove duplicate trace in op ctx ([#845](https://github.com/rivet-gg/rivet/issues/845)) ([dc9812c](https://github.com/rivet-gg/rivet/commit/dc9812c28378195b9679933f13f8ae46aa4c3f88))
* remove erronious dep on linode & cloudflare tokens ([#649](https://github.com/rivet-gg/rivet/issues/649)) ([259abd8](https://github.com/rivet-gg/rivet/commit/259abd85bc621b73c8926b439b0167490aade9e8))
* remove hardcoded eks role ([#586](https://github.com/rivet-gg/rivet/issues/586)) ([f1546c6](https://github.com/rivet-gg/rivet/commit/f1546c69342216a985c0845b8c8592e09f3ecc15))
* Remove old module code ([#533](https://github.com/rivet-gg/rivet/issues/533)) ([689d203](https://github.com/rivet-gg/rivet/commit/689d203326dea34df116bbd4d5ac9ae01f1fa995))
* remove trace from ops ([#780](https://github.com/rivet-gg/rivet/issues/780)) ([d4b80f6](https://github.com/rivet-gg/rivet/commit/d4b80f69dbb1f18d10cef603398f5b381586719e))
* rename api-route -&gt; api-traefik-provider ([#697](https://github.com/rivet-gg/rivet/issues/697)) ([3bf5a1f](https://github.com/rivet-gg/rivet/commit/3bf5a1f0e3872c1f8a8093ba386c9b48d121c451))
* require tunnel before rivet hook ([#714](https://github.com/rivet-gg/rivet/issues/714)) ([22f962f](https://github.com/rivet-gg/rivet/commit/22f962f9cecba0635275313081da3ac76d93098d))
* resolve minio url within k8s when using loopback cluster ip ([#580](https://github.com/rivet-gg/rivet/issues/580)) ([9bd3c83](https://github.com/rivet-gg/rivet/commit/9bd3c83c4822a122fdefa4c0275aebb932df2e21))
* revert [#800](https://github.com/rivet-gg/rivet/issues/800), add http vector filter ([#821](https://github.com/rivet-gg/rivet/issues/821)) ([b154bb6](https://github.com/rivet-gg/rivet/commit/b154bb6f18ff79803109b9db16c6b5bbdb74d65d))
* route and access token tests ([#578](https://github.com/rivet-gg/rivet/issues/578)) ([4d8816a](https://github.com/rivet-gg/rivet/commit/4d8816a02b0cd1697cdecd709909371790fce38d))
* run all tests in one pod ([#615](https://github.com/rivet-gg/rivet/issues/615)) ([3db1a8c](https://github.com/rivet-gg/rivet/commit/3db1a8c3d6a28f1339e50bfd814ad3065497c098))
* server sql ([#715](https://github.com/rivet-gg/rivet/issues/715)) ([7c0418d](https://github.com/rivet-gg/rivet/commit/7c0418d8e03028949a4cdef9d4a68c61d4acc52f))
* standardize token ttl ([#686](https://github.com/rivet-gg/rivet/issues/686)) ([f17d652](https://github.com/rivet-gg/rivet/commit/f17d65283b3cb89df7d29a6930f8e0dcf8759ce2))
* start dns creation after installation ([#829](https://github.com/rivet-gg/rivet/issues/829)) ([e4e7e21](https://github.com/rivet-gg/rivet/commit/e4e7e2152f89c456b66c573856cb094e1404cb80))
* **svc:** change cluster name_id to be unique ([#752](https://github.com/rivet-gg/rivet/issues/752)) ([cea1fe7](https://github.com/rivet-gg/rivet/commit/cea1fe718440013d6ad8d5dcb2e33225bfa2c9aa))
* taint logic for job nodes with no nomad node ([#774](https://github.com/rivet-gg/rivet/issues/774)) ([97f6b72](https://github.com/rivet-gg/rivet/commit/97f6b720715be1b8d1078d3a1d5315f0297f70da))
* team tests ([#571](https://github.com/rivet-gg/rivet/issues/571)) ([3265c66](https://github.com/rivet-gg/rivet/commit/3265c6615897b674723f0d88c4f2b1f21d8d0435))
* test isolation and install script hashing ([#671](https://github.com/rivet-gg/rivet/issues/671)) ([495a7a5](https://github.com/rivet-gg/rivet/commit/495a7a561b20dd411bb00066a1fe574341a87bc8))
* tls install script not running on first boot ([#764](https://github.com/rivet-gg/rivet/issues/764)) ([c13a3ed](https://github.com/rivet-gg/rivet/commit/c13a3ed01cc951600771e41c888b802b72d0e8d9))
* **tunnel:** add legacy route for api-route for gg nodes ([#767](https://github.com/rivet-gg/rivet/issues/767)) ([f2e05ab](https://github.com/rivet-gg/rivet/commit/f2e05ab24a3ba3773ae3d6330ff096c7c63ccc84))
* universal region backwards compatibility regression ([#792](https://github.com/rivet-gg/rivet/issues/792)) ([44d4c0d](https://github.com/rivet-gg/rivet/commit/44d4c0d3df9023a65dd5fc44d7ddfcf460c2601d))
* update rust nix pkg ([#648](https://github.com/rivet-gg/rivet/issues/648)) ([91792d0](https://github.com/rivet-gg/rivet/commit/91792d07d77fde11a285416536d6c058ef56d882))
* **user-presence:** broken redis query ([#802](https://github.com/rivet-gg/rivet/issues/802)) ([a899774](https://github.com/rivet-gg/rivet/commit/a899774a28260acd220b8fd324cec02feee1fc83))
* verify different tags give different lobby ([#620](https://github.com/rivet-gg/rivet/issues/620)) ([8228371](https://github.com/rivet-gg/rivet/commit/82283712fffea2d60b13134a7abf3a32fa83bf4f))


### Documentation

* add api scope to dev tunnel docs ([#747](https://github.com/rivet-gg/rivet/issues/747)) ([86a45f7](https://github.com/rivet-gg/rivet/commit/86a45f7396b5aa62d6aba62669f6ae781e7df5dc))
* Add doc about creating new endpoints ([#645](https://github.com/rivet-gg/rivet/issues/645)) ([f8f4ccc](https://github.com/rivet-gg/rivet/commit/f8f4ccc621c769c1ca5be22921a19af90786de6f))
* Fern installation instructions for script ([#643](https://github.com/rivet-gg/rivet/issues/643)) ([e07ddb3](https://github.com/rivet-gg/rivet/commit/e07ddb38ff9dce2cd5601a2e51c3b81589511b5d))
* update debugging loki command ([#852](https://github.com/rivet-gg/rivet/issues/852)) ([ef20e84](https://github.com/rivet-gg/rivet/commit/ef20e84c0b44df1628560f54d961bdb854351972))
* updating readme pricing information ([#850](https://github.com/rivet-gg/rivet/issues/850)) ([21d3a4e](https://github.com/rivet-gg/rivet/commit/21d3a4e3d8c7eb3aab07107dfe59fa6fda90ce03))


### Continuous Integration

* Disable Prettier checking on changelog for now ([#563](https://github.com/rivet-gg/rivet/issues/563)) ([8bfad8f](https://github.com/rivet-gg/rivet/commit/8bfad8f6d53810b2acc4cb86ccb95ab16e5f47ea))
* Fix release please not adding all items to changelog ([#560](https://github.com/rivet-gg/rivet/issues/560)) ([7191325](https://github.com/rivet-gg/rivet/commit/7191325a6782a0af9c584aa1933db922b552b086))


### Chores

* Add Cargo.lock to generated list ([#710](https://github.com/rivet-gg/rivet/issues/710)) ([ec1c842](https://github.com/rivet-gg/rivet/commit/ec1c84239fff925737138f15c232aa8b30e4e945))
* add comments, region consistency ([#685](https://github.com/rivet-gg/rivet/issues/685)) ([9fe643f](https://github.com/rivet-gg/rivet/commit/9fe643fa7b42bdf1bd22db98f9724d0a12270d58))
* add datacenter location get test ([#673](https://github.com/rivet-gg/rivet/issues/673)) ([79ac6e2](https://github.com/rivet-gg/rivet/commit/79ac6e2a35fb6c8bad4b42c5bb7ff870809fda6f))
* add forwarding script for vector ([#836](https://github.com/rivet-gg/rivet/issues/836)) ([ae7299d](https://github.com/rivet-gg/rivet/commit/ae7299dfe20743780ea72c4b84fde24d18cec1ed))
* add plugins to readme ([#781](https://github.com/rivet-gg/rivet/issues/781)) ([354ab1d](https://github.com/rivet-gg/rivet/commit/354ab1d95b299b9fc06565d8968a348ce17f1bfa))
* add target directory in dockerfile ([#755](https://github.com/rivet-gg/rivet/issues/755)) ([27ab366](https://github.com/rivet-gg/rivet/commit/27ab3663abf88091c22214f568d8f079d28d04aa))
* **api:** move games/builds to game/docker/builds ([#759](https://github.com/rivet-gg/rivet/issues/759)) ([0e169ad](https://github.com/rivet-gg/rivet/commit/0e169adce50d12469dbcf1f0a0eed05a27699490))
* apply prettier formatting ([#849](https://github.com/rivet-gg/rivet/issues/849)) ([5caada5](https://github.com/rivet-gg/rivet/commit/5caada58a6bf0db93b4aae8246d67055b5bc02ce))
* **bolt:** add server filters & update admin api + cli ([#804](https://github.com/rivet-gg/rivet/issues/804)) ([e789bf0](https://github.com/rivet-gg/rivet/commit/e789bf098bbdfe1326d385da1e6a91b9cafc40be))
* **bolt:** upgrade rust to 1.77.2 ([#768](https://github.com/rivet-gg/rivet/issues/768)) ([5cc18f0](https://github.com/rivet-gg/rivet/commit/5cc18f0cc7363d49474997c896cc7472f431ace0))
* change devcontainer user off root ([#743](https://github.com/rivet-gg/rivet/issues/743)) ([af3566a](https://github.com/rivet-gg/rivet/commit/af3566a56ff868497a01c89832f2dedc54b4bf01))
* cherry pick billing feature ([#597](https://github.com/rivet-gg/rivet/issues/597)) ([afe4dd0](https://github.com/rivet-gg/rivet/commit/afe4dd019eb175b8fec57a76f0a8de67714c3a8c))
* cherry pick req extentions ([#738](https://github.com/rivet-gg/rivet/issues/738)) ([a014955](https://github.com/rivet-gg/rivet/commit/a014955ce07b50d61454d49c732fc644c58bdadd))
* clean up dev docs, update readme ([#661](https://github.com/rivet-gg/rivet/issues/661)) ([e306a77](https://github.com/rivet-gg/rivet/commit/e306a77e0526f758bc822880e36e0cf290875ce6))
* clean up ip types ([#709](https://github.com/rivet-gg/rivet/issues/709)) ([64eefd9](https://github.com/rivet-gg/rivet/commit/64eefd998f399daf9567d4cd4c1dc0f1f07e5653))
* clean up server install scripts ([#682](https://github.com/rivet-gg/rivet/issues/682)) ([2564c12](https://github.com/rivet-gg/rivet/commit/2564c129fe5557d7074ce95452071d9ffddb86e5))
* cleanup ([#670](https://github.com/rivet-gg/rivet/issues/670)) ([1c2666c](https://github.com/rivet-gg/rivet/commit/1c2666c2ad0393ba76f4954bc02dac373a33757e))
* cleanup hash code ([#639](https://github.com/rivet-gg/rivet/issues/639)) ([fc17cee](https://github.com/rivet-gg/rivet/commit/fc17cee78df010e21627d993c91656ecc2b40fef))
* clippy fix pass ([#790](https://github.com/rivet-gg/rivet/issues/790)) ([4e95737](https://github.com/rivet-gg/rivet/commit/4e957374130e306f8429deafa1ecc42a6f77dc08))
* **cluster:** increase storage reserved for system on ats ([#723](https://github.com/rivet-gg/rivet/issues/723)) ([0945af7](https://github.com/rivet-gg/rivet/commit/0945af76dbf6193251934daaf96a6d5e0e36f12e))
* **dev:** move rust-anlayzer CARGO_TARGET_DIR to separate dir ([#680](https://github.com/rivet-gg/rivet/issues/680)) ([abe64a8](https://github.com/rivet-gg/rivet/commit/abe64a878022d3a20e9b06d684b2a173dc64eebc))
* **dev:** respect CARGO_TARGET_DIR in bolt  & use non-mounted target in dev container ([#675](https://github.com/rivet-gg/rivet/issues/675)) ([eb1a6cf](https://github.com/rivet-gg/rivet/commit/eb1a6cf65ead433e5976176343f3adc1d6dc4174))
* doc drain & kill timeouts ([#646](https://github.com/rivet-gg/rivet/issues/646)) ([332f88c](https://github.com/rivet-gg/rivet/commit/332f88c480a3d5686e0278a20cbd64d560c821b0))
* dont destroy anything ([#683](https://github.com/rivet-gg/rivet/issues/683)) ([2e50434](https://github.com/rivet-gg/rivet/commit/2e50434baa7976fe0fed25bb0685918a19877682))
* fix deprecated analytics events fields ([#777](https://github.com/rivet-gg/rivet/issues/777)) ([e771f91](https://github.com/rivet-gg/rivet/commit/e771f91e75e5983916a6be2078c788e16b7e4cd6))
* fix queries and install script ([#735](https://github.com/rivet-gg/rivet/issues/735)) ([90b7fc6](https://github.com/rivet-gg/rivet/commit/90b7fc6ae0c0383f4fec1f6344b39ccc9f88b4a6))
* **grafana:** clean up provisioning dashboard ([#820](https://github.com/rivet-gg/rivet/issues/820)) ([3b1d123](https://github.com/rivet-gg/rivet/commit/3b1d1232b22b90a750675948f1c040cd2bafc9e9))
* **infra:** disable vpa for prometheus & traffic server ([#817](https://github.com/rivet-gg/rivet/issues/817)) ([5da29a4](https://github.com/rivet-gg/rivet/commit/5da29a4d718d0254c00814b80a130118387df61d))
* **infra:** increase better uptime check interval to 1m b/c we already have 4x regions ([#819](https://github.com/rivet-gg/rivet/issues/819)) ([6727bdf](https://github.com/rivet-gg/rivet/commit/6727bdf853c14626a1cd2ae1303a8f86016a31e2))
* **job:** gc orphaned jobs from mm ([#627](https://github.com/rivet-gg/rivet/issues/627)) ([a6ce505](https://github.com/rivet-gg/rivet/commit/a6ce505e8c6eedb0a319e571335dee98348cfab0))
* **k8s:** update priority classes to play nice with karpenter & preemption ([#801](https://github.com/rivet-gg/rivet/issues/801)) ([831044d](https://github.com/rivet-gg/rivet/commit/831044da00aaf398acda6de7b2a60c22dadf551c))
* misc fixes ([#706](https://github.com/rivet-gg/rivet/issues/706)) ([875b249](https://github.com/rivet-gg/rivet/commit/875b249e63b17f14c9b0efcfe8a640351b6b4fdd))
* move bolt cluster subcommand to root ([#803](https://github.com/rivet-gg/rivet/issues/803)) ([345d26d](https://github.com/rivet-gg/rivet/commit/345d26d30c03697b6e8d98e5aff81920845bb102))
* move region_config.json to configmap ([#621](https://github.com/rivet-gg/rivet/issues/621)) ([49e439e](https://github.com/rivet-gg/rivet/commit/49e439ed6ef56722ff0f9a62a20fd64e39ce0214))
* publish user-create-complete ([#539](https://github.com/rivet-gg/rivet/issues/539)) ([b2e4006](https://github.com/rivet-gg/rivet/commit/b2e4006af0569f0167d8cef47fa4c8dbdfec8163))
* **push-notification:** remove unused push notification code ([#776](https://github.com/rivet-gg/rivet/issues/776)) ([ee2893e](https://github.com/rivet-gg/rivet/commit/ee2893e2591c51f2a0a1bd5c5b5203a38561c349))
* release 24.4.0 ([#853](https://github.com/rivet-gg/rivet/issues/853)) ([ab2ee63](https://github.com/rivet-gg/rivet/commit/ab2ee6357f2ce1fbf121995b13553cb543579e5a))
* remove cluster_id from servers ([#695](https://github.com/rivet-gg/rivet/issues/695)) ([0ca61a8](https://github.com/rivet-gg/rivet/commit/0ca61a8372ad19c1f5a64b288a63721c18dee067))
* remove unnecessary files ([#668](https://github.com/rivet-gg/rivet/issues/668)) ([c5d0f81](https://github.com/rivet-gg/rivet/commit/c5d0f81ae377e3099f73a35a9e9805c10e123647))
* remove unused code ([#778](https://github.com/rivet-gg/rivet/issues/778)) ([e2f4f13](https://github.com/rivet-gg/rivet/commit/e2f4f13dc30aaebe17385247c88f6f052fbc9aa7))
* replace auto-generate public ip with 127.0.0.1 ([#650](https://github.com/rivet-gg/rivet/issues/650)) ([21d2ad1](https://github.com/rivet-gg/rivet/commit/21d2ad1f8a074cc269be8ab38443d27fe37cfa79))
* Run cleaning ([#701](https://github.com/rivet-gg/rivet/issues/701)) ([4955e28](https://github.com/rivet-gg/rivet/commit/4955e2800e4d1039cd71c503f12d6bf0d20b035d))
* run imports formatting ([#779](https://github.com/rivet-gg/rivet/issues/779)) ([1c0bbf8](https://github.com/rivet-gg/rivet/commit/1c0bbf8bfe25e1ce3c09f513f9790392a4ab60d0))
* standardize custom image list size ([#688](https://github.com/rivet-gg/rivet/issues/688)) ([8086559](https://github.com/rivet-gg/rivet/commit/80865595793d06640281e53620b0d30ae594e87e))
* update baseline secrets ([#663](https://github.com/rivet-gg/rivet/issues/663)) ([54f3135](https://github.com/rivet-gg/rivet/commit/54f3135850fef3442f0ab44133ee6ecbc980f6b3))
* update default builds ([#824](https://github.com/rivet-gg/rivet/issues/824)) ([a6d5854](https://github.com/rivet-gg/rivet/commit/a6d5854c904b6c7e008dc7ab34c0b40382ecf664))
* update devcontainer docker base image ([#739](https://github.com/rivet-gg/rivet/issues/739)) ([e91d538](https://github.com/rivet-gg/rivet/commit/e91d5385fb724b12db9520ae998a0f112e4ea9df))
* update recovery & confirmation period for better uptime ([#716](https://github.com/rivet-gg/rivet/issues/716)) ([ee7547b](https://github.com/rivet-gg/rivet/commit/ee7547bc70a8cd5914d8d41213c154eec123c8e1))
* Update sdks ([#642](https://github.com/rivet-gg/rivet/issues/642)) ([8dbcfc5](https://github.com/rivet-gg/rivet/commit/8dbcfc5fb05c2d7f4f7cbc67186ef134d99528d7))
* **vector:** filter unneeded go & prom metrics ([#837](https://github.com/rivet-gg/rivet/issues/837)) ([041ae05](https://github.com/rivet-gg/rivet/commit/041ae05bf8a198281ac04add9f2ce675d6089d19))

## [24.3.0](https://github.com/rivet-gg/rivet/compare/v24.2.0...v24.3.0) (2024-03-01)


### Features

* **bolt:** add region filter to ssh command ([#537](https://github.com/rivet-gg/rivet/issues/537)) ([af274a8](https://github.com/rivet-gg/rivet/commit/af274a8e99666e24f3f289b389246347fbb9ae1d))
* expose nomad dashboard via cloudflare tunnels ([#543](https://github.com/rivet-gg/rivet/issues/543)) ([3a574c0](https://github.com/rivet-gg/rivet/commit/3a574c03dfad3d7e0bb8a733576b1220608f2ea1))
* **Main:** Added Devcontainer files ([9bb97db](https://github.com/rivet-gg/rivet/commit/9bb97db1e3b211830eada237eca3b6fa210ba7b8))
* **mm:** add config to opt-in individual games for host networking & root containers ([#549](https://github.com/rivet-gg/rivet/issues/549)) ([be9ddd6](https://github.com/rivet-gg/rivet/commit/be9ddd6328a06bf3057d78ed94d9bd7c66c41284))


### Bug Fixes

* add checksum annotations to cloudflared deployment ([#542](https://github.com/rivet-gg/rivet/issues/542)) ([f2d847b](https://github.com/rivet-gg/rivet/commit/f2d847be17aa7b23d060292ec0aba6c213717a37))
* **bolt:** clarify 1password service token warning ([#541](https://github.com/rivet-gg/rivet/issues/541)) ([eb2e7d5](https://github.com/rivet-gg/rivet/commit/eb2e7d58c5b8f6e07bfa7740d15ae5da25f68987))
* correct hcaptcha length ([#548](https://github.com/rivet-gg/rivet/issues/548)) ([748aaa8](https://github.com/rivet-gg/rivet/commit/748aaa8d38a724b5f5f3bac0d7993cb7ace50045))
* inaccessible admin routes ([#555](https://github.com/rivet-gg/rivet/issues/555)) ([9896b09](https://github.com/rivet-gg/rivet/commit/9896b09821d86f01cf6729841764195eabb6b3dd))
* revert to redis-rs v0.23.3 with panic patch ([#552](https://github.com/rivet-gg/rivet/issues/552)) ([3780eaa](https://github.com/rivet-gg/rivet/commit/3780eaa2fa6fa5f2840411193e617b9b77984b43))
* updated docs error url ([#544](https://github.com/rivet-gg/rivet/issues/544)) ([7099658](https://github.com/rivet-gg/rivet/commit/70996584bee4678d3d42afc49ed3ed3053b9c44c))

## [24.2.1] - Unreleased

### Changed

-   Reduced minimal infrastructure required to get Rivet running:
    -   Made K8s Dashboard disabled by default
    -   Made Prometheus and friends (Vector, Loki, Promtail) disabled by default
    -   Made Clickhouse disabled by default
    -   Made NSFW Check API disabled by default
    -   Made NSFW Check API disabled by default
    -   Made Image Resizing (via Imagor) disabled by default

## [24.2.1] - Unreleased

### Changed

-   Reduced minimal infrastructure required to get Rivet running:
    -   Made K8s Dashboard disabled by default
    -   Made Prometheus and friends (Vector, Loki, Promtail) disabled by default
    -   Made Clickhouse disabled by default
    -   Made NSFW Check API disabled by default
    -   Made NSFW Check API disabled by default
    -   Made Image Resizing (via Imagor) disabled by default

## [24.2.0] - 2024-02-22

### Added

-   **Infra** Added Better Uptime monitor
-   **Bolt** Add Docker `RUN` cache to distributed deploys to improve deploy speeds
-   **Infra** Prometheus VPA
-   **Infra** Apache Traffic Server VPA
-   **api-cloud** Admins can view all teams & games in a cluster
-   Added automatic deploy CI for staging
-   **Infra** Added compactor and GC to Loki
-   **api-status** Test individual Game Guard nodes to ensure all nodes have the correct configuration
-   Generate separate SDKs for `runtime` (lightweight, essentials for running a game) and `full` (heavy, includes cloud APIs)
-   Metrics for cache operations as well as a Grafana dashboard
-   **Bolt** Added namespace config and secrets sync with `bolt config pull` and `bolt config push` via 1Password
-   `GROUP_DEACTIVATED` error now shows reasons for deactivation. Added docs for deactivation reasons
-   `/health/essential` endpoint to test connectivity to all essential services
-   Added error when trying to deploy a distributed cluster on a non-linux-x86 machine (not supported)

### Changed

-   **api-status** More comprehensive status check that both creates a lobby & connects to it
-   More details in `CLAIMS_MISSING_ENTITLEMENT` error
-   **API** Added 120s timeout to reading request body and writing response to all requests going through Traefik
-   **Infra** Update Promtail logs to match k8s semantics
-   **Infra** Added `Cache-Control: no-cache` to 400 responses from CDN
-   **[BREAKING]** **Infra** Removed config-less hCaptcha. You are now required to provide a site key and
    secret key for the hCaptcha config in your game version matchmaker config for all future versions (old
    version will remain operational using our own hCaptcha site key).
-   **Internal** Updated source hash calculation to use `git diff` and `git rev-parse HEAD`
-   **API** Removed `x-fern-*` headers from generated TypeScript clients
-   Implemented liveness probe to check connectivity to essential services
-   Remove public-facing health check endpoints
-   **API** Removed ability to choose a name id when creating a game. One will be generated based on the given display name
-   **Infra** Reduced allocated cache size on ATS nodes to prevent disks exhaustion

### Fixed

-   **Bolt** Prompt prod won't prompt if does not have user control
-   **Bolt** Exclude copying bloat from `infra/tf/` to distributed Docker builds
-   Invalid JWT tokens now return explicit `TOKEN_INVALID` error instead of 500
-   **Infra** Remove debug logging from traefik-tunnel
-   Game lobby logs now ship even when the lobby fails immediately
-   Fixed `CLAIMS_MISSING_ENTITLEMENT` not formatting correctly (reason given was `?`)
-   Added role ARN to exec commands in `k8s-cluster-aws` tf provider to properly authenticate
-   Change email attached to Stripe on group ownership change
-   Enable `keep-alive` on `redis` crate
-   Update `redis` crate to mitigate panic on connection failure during `AUTH`
-   Wrong grace period for GG config to update after `mm::msg::lobby_ready`

### Security

-   Resolve [RUSTSEC-2024-0003](https://rustsec.org/advisories/RUSTSEC-2024-0003)

## [24.1.0] - 2024-01-23

### Added

-   **Infra** New `job-runner` crate responsible for managing the OCI bundle runtime & log shipping on the machine
-   **Infra** Jobs now log an explicit rate message when logs are rate limited & truncated
-   **Infra** `infra-artifacts` Terraform plan & S3 bucket used for automating building & uploading internal binaries, etc.
-   **Infra** Aiven Redis provider
-   **Bolt** `bolt secret set <path> <value>` command
-   **Bolt** `bolt.confirm_commands` to namespace to confirm before running commands on a namespace
-   `watch-requests` load test
-   `mm-sustain` load test
-   **Infra** Automatic server provisioning system ([Read more](/docs/packages/cluster/SERVER_PROVISIONING.md)).

### Changed

-   **Matchmaker** Allow excluding `matchmaker.regions` in order to enable all regions
-   **Matchmaker** Lowered internal overhead of log shipping for lobbies
-   **Matchmaker** Game mode names are now more lenient to include capital letters & underscores
-   **API** Return `API_REQUEST_TIMEOUT` error after 50s (see `docs/infrastructure/TIMEOUTS.md` for context)
-   **API** Move generated client APIs to sdks/
-   **API** Lower long poll timeout from 60s -> 40s
-   **Bolt** Moved additional project roots to Bolt.toml
-   **types** Support multiple project roots for reusing Protobuf types
-   **Infra** Switch from AWS ELB to NLB to work around surge queue length limitation
-   **Infra** Loki resources are now configurable
-   **pools** Allow infinite Redis reconnection attempts
-   **pools** Set Redis client names
-   **pools** Ping Redis every 15 seconds
-   **pools** Enable `test_before_acquire` on SQLx
-   **pools** Decrease SQLx `idle_timeout` to 3 minutes
-   **pools** Set ClickHouse `idle_timeout` to 15 seconds
-   **api-helper** Box path futures for faster compile times
-   Upgrade `async-nats`
-   `test-mm-lobby-echo` now handles `SIGTERM` and exits immediately, allows for less resource consumption while testing lobbies
-   **mm** Dynamically sleep based on lobby's `create_ts` for Treafik config to update
-   **Infra** Update Traefik tunnel client & server to v3.0.0-beta5
-   **Infra** Update Traefik load balancer to v2.10.7

### Security

-   Resolve [RUSTSEC-2023-0044](https://rustsec.org/advisories/RUSTSEC-2023-0074)

### Fixed

-   **Infra** runc rootfs is now a writable file system
-   **Matchmaker** Logs not shipping if lobby exits immediately
-   **Matchmaker** Returning `lnd-atl` instead of `dev-lcl` as the mocked mocked region ID in the region list
-   **API** 520 error when long polling
-   **api-cloud** Returning wrong domain for `domains.cdn`
-   **Infra** Fix Prometheus storage retention conversion between mebibytes and megabytes
-   **Infra** Fix typo in Game Guard Traefik config not exposing API endpoint
-   **Infra** Kill signal for servers was `SIGINT` instead of `SIGTERM`
-   **Infra** NATS cluster not getting enabled
-   **Infra** Redis Kubernetes error when using non-Kubernetes provider
-   **api-helper** Remove excess logging
-   `user_identity.identities` not getting purged on create & delete
-   **Bolt** Error when applying Terraform when a plan is no longer required
-   **api-helper** Instrument path futures
-   **Infra** CNI ports not being removed from the `nat` iptable, therefore occasionally causing failed connections
-   **Infra** Disable `nativeLB` for Traefik tunnel
-   **Infra** Update default Nomad storage to 64Gi
-   **Infra** Tunnel now exposes each Nomad server individually so the Nomad client can handle failover natively instead of relying on Traefik
-   **Infra** Traefik tunnel not respecting configured replicas
-   **Bolt** ClickHouse password generation now includes required special characters

## [23.2.0-rc.1] - 2023-12-01

### Added

-   **Infra** Lobby tagging system for filtering lobbies in `/find`
-   **Infra** Dynamically configurable max player count in `/find` and `/create`
-   **Bolt** Added `bolt admin login` to allow for logging in without an email provider setup. Automatically turns the user into an admin for immediate access to the developer dashboard.
-   **Bolt** Fixed `bolt db migrate create`
-   **Infra** Added `user-admin-set` service for creating an admin user
-   **api-cloud** `/bootstrap` properties for `access` and `login_methods`

### Changed

-   **Bolt** Removed `bolt admin team-dev create`. You can use `bolt admin login` and the hub to create a new dev team
-   **Infra** Turnstile `CAPTCHA_CAPTCHA_REQUIRED` responses now include a site key
-   **Infra** Turnstile is no longer configurable by domain (instead configured by Turnstile itself)
-   **Infra** Job log aggregating to use Vector under the hood to insert directly into ClickHouse
-   **Matchmaker** Players automatically remove after extended periods of time to account for network failures

### Fixed

-   **Infra** Job logs occasionally returning duplicate log lines
-   **Matchmaker** /list returning no lobbies unless `include_state` query parameter is `true`
-   **Matchmaker** Players remove correctly when the player fails to be inserted into the Cockroach database and only exists in Redis
-   **Chirp** `tail_all` default timeouts are now lower than `api-helper` timeout
-   **api-kv** Batch operation timeouts are now lower than `api-helper` timeout

## [23.1.0] - 2023-10-30

### Added

-   **Bolt** Development cluster can now be booted without any external services (i.e. no Linode & Cloudflare account required, does not require LetsEncrypt cert)
-   **Infra** Autoscale non-singleton services based on CPU & memory
-   **Infra** Support for running ClickHouse on ClickHouse Cloud
-   **Infra** Support for running CockroachDB on Cockroach Cloud
-   **Infra** Support for running Redis on AWS ElastiCache & MemoryDB
-   **Infra** Dynamically provisioned core cluster using Karpenter
-   **Infra** Dual-stack CNI configuration for game containers
-   **Infra** job iptables firewall to job pool that whitelists inbound traffic from Game Guard to the container
-   **Infra** job iptables rules to configure minimize delay TOS for traffic without a TOS
-   **Infra** job iptables rules to configure maximize throughput TOS for traffic from ATS
-   **Infra** job Linux traffic control filters to prioritize game traffic over other background traffic
-   **Infra** Prewarm the Traffic Server cache when a game version is published for faster cold start times on the first booted lobby in each region
-   **Infra** Envoy Maglev load balancing for traffic to edge Traffic Server instances to maximize cache hits
-   **Bolt** Timeout for tests
-   **Bolt** New summary view of test progress
-   **Bolt** `config show` command
-   **Bolt** `ssh pool --all <COMMAND>` command
-   **Bolt** Validation that the correct pools exist in th enamespace
-   **Bolt** Validation that the matchmaker delivery method is configured correctly depending on wether ATS servers exist
-   **Dev** Bolt automatically builds with Nix shell
-   **Bolt** `--no-purge` flag to `test` to prevent purging Nomad jobs
-   **Matchmaker** Expose hardware metrics to container with `RIVET_CPU`, `RIVET_MEMORY`, and `RIVET_MEMORY_OVERSUBSCRIBE`
-   **api-cloud** `GET /cloud/bootstrapp` to provide initial config data to the hub
-   **api-cloud** Dynamically send Turnstile site key to hub
-   **Infra** Rate limit on creating new SQL connections to prevent stampeding connections

### Changed

-   Cleaned up onboarding experience for open source users, see _docs/getting_started/DEVELOPMENT.md_
-   **Infra** Moved default API routes from `{service}.api.rivet.gg/v1` to `api.rivet.gg/{service}`
-   **Infra** Removed version flat from API request paths
-   **Bolt** Tests are built in batch and binaries are ran in parallel in order to speed up test times
-   **Bolt** Run tests inside of Kubernetes pod inside cluster, removing the need for port forwarding for tests
-   **Bolt** Remove `disable_cargo_workspace` flag since it is seldom used
-   **Bolt** Remove `skip_dependencies`, `force_build`, and `skip_generate` on `bolt up` and `bolt test` commands that are no longer relevant
-   **api-route** Split up routes in to `/traefik/config/core` and `/traefik/config/game-guard`
-   **Imagor** CORS now mirror the default CORS configured for S3
-   **Dev** `git lfs install` automatically runs in `shellHook`
-   **Dev** Removed `setup.sh` in lieu of `shellHook`
-   Replaced `cdn.rivet.gg` domains with presigned requests directly to the S3 provider
-   **api-matchmaker** Gracefully disable automatic region selection when coords not obtainable
-   **Infra** Disabling DNS uses `X-Forwarded-For` header for the client IP
-   **Infra** Pool connections are now created in parallel for faster tests & service start times
-   **Infra** Connections from edge <-> core services are now done over mTLS with Treafik instead of cloudflared
-   **Infra** ClickHouse database connections now use TLS
-   **Infra** CockroachDB database connections now use TLS
-   **Infra** Redis database connections now use TLS
-   **Infra** Redis now uses Redis Cluster for everything
-   **Infra** Cloudflare certificate authority from DigitCert to Lets Encrypt
-   **Infra** Removed 1.1.1.1 & 1.0.0.1 as resolvers from Nomad jobs due to reliability issues
-   **Infra** Added IPv6 DNS resolvers to Nomad jobs
-   **Infra** CNI network for jobs from bridge to ptp for isolation & performance
-   **Infra** Remove requirement of `Content-Type: application/x-tar` for builds because of new compression types
-   **Matchmaker** Expose API origin to `RIVET_API_ENDPOINT` env var to lobby containers
-   **[BREAKING]** **Infra** Removed undocumented environment variables exposed by Nomad (i.e. anything prefixed with `NOMAD_`)

### Fixed

-   `LC_ALL: cannot change locale` error from glibc
-   **Dev** Bolt uses `write_if_different` for auto-generated files to prevent cache purging

## [23.1.0-rc4] - 2023-09-02

### Changed

-   Revert Fern TypeScript generator to 0.5.6 to fix bundled export

## [23.1.0-rc3] - 2023-09-02

### Changed

-   Don't publish internal Fern package on tag to prevent duplicate pushes

## [23.1.0-rc2] - 2023-09-02

### Changed

-   Update to Fern 0.15.0-rc7
-   Update Fern TypeScript, Java, and Go generators

## [23.1.0-rc1] - 2023-09-02

### Added

-   **Matchmaker** Support custom lobbies
-   **Matchmaker** Support lobby state
-   **Matchmaker** Support external verification
-   **Library** Support Java library
-   **Library** Support Go library
-   **Cloud** Support multipart uploads for builds
-   **Infra** Support configuring multiple S3 providers
-   **Infra** Support multipart uploads
-   **Infra** Replace Promtail-based log shipping with native Loki Docker driver
-   **Infra** Local Traefik Cloudflare proxy daemon for connecting to Cloudflare Access services
-   **Infra** Upload service builds to default S3 provider instead of hardcoded bucket
-   **Infra** Enable Apache Traffic Server pull-through cache for Docker builds
-   **Bolt** Support for connecting to Redis databases with `bolt redis sh`
-   **Bolt** Confirmation before running any command in the production namespace
-   **Bolt** `--start-at` flag for all infra commands
-   **Bolt** Explicit database dependencies in services to reduce excess database pools

### Changed

-   **Infra** Update CNI plugins to 1.3.0
-   **Infra** Update ClickHouse to 23.7.2.25
-   **Infra** Update Cockroach to 23.1.7
-   **Infra** Update Consul Exporter to 1.9.0
-   **Infra** Update Consul to 1.16.0
-   **Infra** Update Imagor to 1.4.7
-   **Infra** Update NATS server to 2.9.20
-   **Infra** Update Node Exporter server to 1.6.0
-   **Infra** Update Nomad to 1.6.0
-   **Infra** Update Prometheus server to 2.46.0
-   **Infra** Update Redis Exporter to 1.52.0
-   **Infra** Update Redis to 7.0.12
-   **Infra** Update Traefik to 2.10.4
-   **Bolt** PostHog events are now captured in a background task
-   **Bolt** Auto-install rsync on Salt Master
-   **Bolt** Recursively add dependencies from overridden services when using additional roots
-   **KV** Significantly rate limit of all endpoints

### Security

-   Resolve [RUSTSEC-2023-0044](https://rustsec.org/advisories/RUSTSEC-2023-0044)
-   Resolve [RUSTSEC-2022-0093](https://rustsec.org/advisories/RUSTSEC-2022-0093)
-   Resolve [RUSTSEC-2023-0053](https://rustsec.org/advisories/RUSTSEC-2023-0053)

### Fixed

-   **Portal** Skip captcha if no Turnstile key provided
-   **Infra** Missing dpenedency on mounting volume before setting permissions of /var/\* for Cockroach, ClickHouse, Prometheus, and Traffic Server
-   **Chrip** Empty message parameters now have placeholder so NATS doesn't throw an error
-   **Chrip** Messages with no parameters no longer have a trailing dot
-   **Bolt** Correctly resolve project root when building services natively
-   **Bolt** Correctly determine executable path for `ExecServiceDriver::UploadedBinaryArtifact` with different Cargo names
