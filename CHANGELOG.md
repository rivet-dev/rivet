# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Calendar Versioning](https://calver.org/).

## [25.6.0](https://github.com/rivet-gg/rivet/compare/v25.5.2...v25.6.0) (2025-09-02)


### Features

* add driver context access in createVars function ([#2841](https://github.com/rivet-gg/rivet/issues/2841)) ([0309f87](https://github.com/rivet-gg/rivet/commit/0309f873471d8a0a8fc7e4d666f826a26da10c66))
* expose GET /actors/usage, GET /actors/logs, GET /actors/logs/export, GET /routes/history ([#2716](https://github.com/rivet-gg/rivet/issues/2716)) ([69144d3](https://github.com/rivet-gg/rivet/commit/69144d396852e80788209cc8e9830406c943c0ac))
* implement `clickhouse-user-query` ([#2554](https://github.com/rivet-gg/rivet/issues/2554)) ([4ceeadd](https://github.com/rivet-gg/rivet/commit/4ceeaddbf407bfa8aa95f30d714772606ff66e77))
* **site:** add brand dropdown ([#2812](https://github.com/rivet-gg/rivet/issues/2812)) ([6e7a696](https://github.com/rivet-gg/rivet/commit/6e7a6961b5a3bf4228daec1822441199fba23939))
* **site:** add newsleter ([#2803](https://github.com/rivet-gg/rivet/issues/2803)) ([88d29e6](https://github.com/rivet-gg/rivet/commit/88d29e6f3da15a786d49b23aada68ecad030dcce))
* **site:** add next.js docs ([#2834](https://github.com/rivet-gg/rivet/issues/2834)) ([7dff920](https://github.com/rivet-gg/rivet/commit/7dff92004fe893f00daf5fe597d4c16514deab6a))
* **site:** bring back docs navigation to mobile ([#2814](https://github.com/rivet-gg/rivet/issues/2814)) ([bbc72f8](https://github.com/rivet-gg/rivet/commit/bbc72f8b9e9325e88872074d5cbc0352c4e8d272))
* **site:** set user's email during newsletter form submission ([#2810](https://github.com/rivet-gg/rivet/issues/2810)) ([c235c8c](https://github.com/rivet-gg/rivet/commit/c235c8c439d19ec8bced4f00114788f244475bc9))


### Bug Fixes

* 404 and descriptions ([#2830](https://github.com/rivet-gg/rivet/issues/2830)) ([ab281d6](https://github.com/rivet-gg/rivet/commit/ab281d6be5a3e38d98f0999bd9c5b33c0ef969a4))
* 404 redirects ([#2820](https://github.com/rivet-gg/rivet/issues/2820)) ([2a470a3](https://github.com/rivet-gg/rivet/commit/2a470a395a721694316ae9944478bc80a2e9967f))
* 404s ([#2835](https://github.com/rivet-gg/rivet/issues/2835)) ([239d76c](https://github.com/rivet-gg/rivet/commit/239d76c3392cafd841884c97f9b093c34dd430e7))
* Add redirects for 404 documentation URLs ([#2816](https://github.com/rivet-gg/rivet/issues/2816)) ([3ada1b2](https://github.com/rivet-gg/rivet/commit/3ada1b2ac330b8f9a6da69c8a3a4783b7298d109))
* additional 404 redirects ([#2818](https://github.com/rivet-gg/rivet/issues/2818)) ([d0f94ca](https://github.com/rivet-gg/rivet/commit/d0f94cacbfd6748507cb7d8cb5a9978ad2c06397))
* canonical URLs ([#2822](https://github.com/rivet-gg/rivet/issues/2822)) ([b9d6013](https://github.com/rivet-gg/rivet/commit/b9d6013b275240e9a77eaecd2c2268514586ed60))
* **dev-full:** update to use new ports ([#2717](https://github.com/rivet-gg/rivet/issues/2717)) ([74e2654](https://github.com/rivet-gg/rivet/commit/74e26546228d2c1d35b615228a43301b210909cb))
* gracefully handle prom failure for pb topo ([#2731](https://github.com/rivet-gg/rivet/issues/2731)) ([2840095](https://github.com/rivet-gg/rivet/commit/28400957ef3370d9a85001b36bf22801fb69854e))
* **hub:** add missing dalas icon ([#2838](https://github.com/rivet-gg/rivet/issues/2838)) ([b78c3d6](https://github.com/rivet-gg/rivet/commit/b78c3d673674f6a25139bd76ed482ebcd18e736a))
* **hub:** bring back monospace font ([#2781](https://github.com/rivet-gg/rivet/issues/2781)) ([865550c](https://github.com/rivet-gg/rivet/commit/865550cf6264e22b591781ea087368c07d713f5f))
* links and orphan pages ([#2824](https://github.com/rivet-gg/rivet/issues/2824)) ([d25aced](https://github.com/rivet-gg/rivet/commit/d25aced8351802d0e2ef9158c4ae4efb3b6576ac))
* load aurrent actor if its outside of current pagination window ([#2772](https://github.com/rivet-gg/rivet/issues/2772)) ([23c76be](https://github.com/rivet-gg/rivet/commit/23c76be85f3587c4e6eda9c9b147f349fc7d9481))
* **pegboard:** include namespace in actor log query ([#2712](https://github.com/rivet-gg/rivet/issues/2712)) ([5cc7d50](https://github.com/rivet-gg/rivet/commit/5cc7d506e002c8433d7157e25b972f7e99b44c98))
* SEO improvements ([#2827](https://github.com/rivet-gg/rivet/issues/2827)) ([32ac342](https://github.com/rivet-gg/rivet/commit/32ac342f6e6c41155ef815bfd78562a1b41ddde6))
* **site:** fix initial animation state in sidebar ([#2797](https://github.com/rivet-gg/rivet/issues/2797)) ([4221722](https://github.com/rivet-gg/rivet/commit/42217221fc25b2180493fc3bfdbbebae8dd9480a))
* **site:** fix llms.txt link ([#2787](https://github.com/rivet-gg/rivet/issues/2787)) ([bc24ee5](https://github.com/rivet-gg/rivet/commit/bc24ee58ceaeb87eb66bf29bd1ee5076d456f12c))
* **toolchain:** dont upgrade for deploys by default ([#2743](https://github.com/rivet-gg/rivet/issues/2743)) ([135d982](https://github.com/rivet-gg/rivet/commit/135d982fc60c81cd5b231bde96b2d09247636977))
* **toolchain:** fix compat with deploying using podman ([#2571](https://github.com/rivet-gg/rivet/issues/2571)) ([716868c](https://github.com/rivet-gg/rivet/commit/716868c4c3d6ed4e3fc9e68f1ddd29234bb72c73))
* **toolchain:** mark external deps ([#2713](https://github.com/rivet-gg/rivet/issues/2713)) ([4ca8675](https://github.com/rivet-gg/rivet/commit/4ca8675a3aa10a9709a3ed7c7c2768f70007dbb3))
* update Claude URL from /chat to /new ([#2790](https://github.com/rivet-gg/rivet/issues/2790)) ([24e4666](https://github.com/rivet-gg/rivet/commit/24e46666c1f66a2c029f8d6cda48b81e52b8638f))
* upgrade actors by build name ([#2741](https://github.com/rivet-gg/rivet/issues/2741)) ([9eddbd3](https://github.com/rivet-gg/rivet/commit/9eddbd3f3c409edb594bb5ac5a2d81c37a3906d1))
* **workflows:** fix race condition of workflow waking before commit ([#2748](https://github.com/rivet-gg/rivet/issues/2748)) ([3493a1d](https://github.com/rivet-gg/rivet/commit/3493a1d4b294cd43575ef80fcf8b30009ae413db))
* **workflows:** fix signal publish/listen race condition ([#2742](https://github.com/rivet-gg/rivet/issues/2742)) ([cb1d4d7](https://github.com/rivet-gg/rivet/commit/cb1d4d7a5149621fc1f65dea74fb97f401016101))


### Documentation

* add runInBackground method and update actor lifecycle docs ([#2842](https://github.com/rivet-gg/rivet/issues/2842)) ([89519b9](https://github.com/rivet-gg/rivet/commit/89519b910fe2a1921e06e097b42fccbae63a602c))
* update WebSocket handler docs and client type references ([#2791](https://github.com/rivet-gg/rivet/issues/2791)) ([0b88ac3](https://github.com/rivet-gg/rivet/commit/0b88ac3c3543158dae60a2f925da75842e313a7e))


### Code Refactoring

* **docs:** update react/js/rust client docs ([#2832](https://github.com/rivet-gg/rivet/issues/2832)) ([672cc6b](https://github.com/rivet-gg/rivet/commit/672cc6bb414f104df2320464cfdbff73ec81ec68))
* **hub:** hide rivetkit related tabs from ui ([#2724](https://github.com/rivet-gg/rivet/issues/2724)) ([d8d3e8e](https://github.com/rivet-gg/rivet/commit/d8d3e8e4516ad40e02b87086dfbc34d51e1f44ab))


### Chores

* a mess of merging everything together ([#2720](https://github.com/rivet-gg/rivet/issues/2720)) ([4646b83](https://github.com/rivet-gg/rivet/commit/4646b835e137327d7ca61ccdd64e13b5791c2b19))
* add architecture docs ([#2798](https://github.com/rivet-gg/rivet/issues/2798)) ([e636233](https://github.com/rivet-gg/rivet/commit/e6362338541a7a9f89f27fbfc1a8c56942b1b23e))
* add copy as markdown, preserve sidebar dropdown state, landing page tweaks ([#2763](https://github.com/rivet-gg/rivet/issues/2763)) ([d0fc1c8](https://github.com/rivet-gg/rivet/commit/d0fc1c8a68dd3719b3ea47255309363056992d86))
* add enterprise flags for rivet cloud ([#2799](https://github.com/rivet-gg/rivet/issues/2799)) ([94704d4](https://github.com/rivet-gg/rivet/commit/94704d4e66cbe9d0fc5fd4aa8ec4b7423bf9a922))
* add mobile nav & code snippets layout ([#2753](https://github.com/rivet-gg/rivet/issues/2753)) ([21fb11e](https://github.com/rivet-gg/rivet/commit/21fb11e22fc6ffe4e5dbfde3e96f0f735d5f4e44))
* add quickstart index page ([#2759](https://github.com/rivet-gg/rivet/issues/2759)) ([6cfd757](https://github.com/rivet-gg/rivet/commit/6cfd7576099dcdcbd9cabe0f51442ea3e511cc34))
* add sitemap ([#2737](https://github.com/rivet-gg/rivet/issues/2737)) ([d083bf5](https://github.com/rivet-gg/rivet/commit/d083bf554f0ad1f72872bb1af2f3888170c4842b))
* add talk to an engineer page ([#2756](https://github.com/rivet-gg/rivet/issues/2756)) ([c827b91](https://github.com/rivet-gg/rivet/commit/c827b91173932ecb08f8d61f81774bbe26c7914b))
* add typesense search ([#2764](https://github.com/rivet-gg/rivet/issues/2764)) ([6705af9](https://github.com/rivet-gg/rivet/commit/6705af9525e640e14978beefac2140098112ea39))
* auto-generate readme examples ([#2792](https://github.com/rivet-gg/rivet/issues/2792)) ([7f55187](https://github.com/rivet-gg/rivet/commit/7f55187b10c58479d504d51d29c792e4c228f8d3))
* change open in chatgpt/claude to use url instead of full markdown text ([#2766](https://github.com/rivet-gg/rivet/issues/2766)) ([0097f67](https://github.com/rivet-gg/rivet/commit/0097f678c6fbf3a26fcbb1c625b61de31a023cc5))
* clean up mdx components ([#2769](https://github.com/rivet-gg/rivet/issues/2769)) ([a86abce](https://github.com/rivet-gg/rivet/commit/a86abce8fe8d21e782017a966aedddc0f283e0cd))
* clean up nav ([#2776](https://github.com/rivet-gg/rivet/issues/2776)) ([97e7f7e](https://github.com/rivet-gg/rivet/commit/97e7f7e4a093b6340aeb0da6e5aaee55289f2663))
* clean up nav styling ([#2751](https://github.com/rivet-gg/rivet/issues/2751)) ([9b48931](https://github.com/rivet-gg/rivet/commit/9b48931503a83e06d649d415cdbf25fe529b6635))
* clean up remaining docs ([#2770](https://github.com/rivet-gg/rivet/issues/2770)) ([497c855](https://github.com/rivet-gg/rivet/commit/497c8550edeb8af6e9c544a9c131cd39ad278c40))
* dim platform icons ([#2736](https://github.com/rivet-gg/rivet/issues/2736)) ([8ad23b1](https://github.com/rivet-gg/rivet/commit/8ad23b1585f589dc56f4248dfbf063b221722574))
* document fetch & ws handler ([#2767](https://github.com/rivet-gg/rivet/issues/2767)) ([834b7d1](https://github.com/rivet-gg/rivet/commit/834b7d1be423f9404c0c3d776bee1b05ea953ae7))
* fix active active page in router ([#2762](https://github.com/rivet-gg/rivet/issues/2762)) ([a87caf8](https://github.com/rivet-gg/rivet/commit/a87caf864b75408e003e7a87db4b7992ecee3551))
* fix building site for cf ([#2757](https://github.com/rivet-gg/rivet/issues/2757)) ([8710efb](https://github.com/rivet-gg/rivet/commit/8710efb96443e45c3e3f650794b024a1e86c105a))
* fix mobile layout on landing page ([#2754](https://github.com/rivet-gg/rivet/issues/2754)) ([714a597](https://github.com/rivet-gg/rivet/commit/714a597d478cb3509fc6fcbb6cd750144b2ef297))
* fix quickstart & update hotsing providers links ([#2777](https://github.com/rivet-gg/rivet/issues/2777)) ([a0ac16e](https://github.com/rivet-gg/rivet/commit/a0ac16e979e7aba945a24e12eeff622738f0324d))
* fix railway link ([#2785](https://github.com/rivet-gg/rivet/issues/2785)) ([041c519](https://github.com/rivet-gg/rivet/commit/041c519eb04ff127eee35ccc9c0abaeb7ea41998))
* fix steps styling ([#2755](https://github.com/rivet-gg/rivet/issues/2755)) ([1e2384a](https://github.com/rivet-gg/rivet/commit/1e2384ae754f116a68e3792abc41dbe0fea86772))
* fix vercel routing ([#2760](https://github.com/rivet-gg/rivet/issues/2760)) ([2122969](https://github.com/rivet-gg/rivet/commit/212296916702dc1a347daf0f74a913b0e8753493))
* force update hub ([0449ead](https://github.com/rivet-gg/rivet/commit/0449eadac329206f2672126e47586d123407a4b8))
* improve fetch & websocket handler ([#2771](https://github.com/rivet-gg/rivet/issues/2771)) ([8c96b2e](https://github.com/rivet-gg/rivet/commit/8c96b2e7e63c6ef33f809d7d6b640071e5162e05))
* link to websocket examples ([#2779](https://github.com/rivet-gg/rivet/issues/2779)) ([5aa1a77](https://github.com/rivet-gg/rivet/commit/5aa1a77e1da9c6dcf916aa033e0d02e80cdbc996))
* make site full width ([#2752](https://github.com/rivet-gg/rivet/issues/2752)) ([51278d1](https://github.com/rivet-gg/rivet/commit/51278d1ba8efe4c4594d426663750ab35c526fcc))
* mark LLM lists and docs as generated files ([#2788](https://github.com/rivet-gg/rivet/issues/2788)) ([a4ed08d](https://github.com/rivet-gg/rivet/commit/a4ed08de62d2087aa9327c2b772f6c8a86cb7595))
* merge v2 ([43682f7](https://github.com/rivet-gg/rivet/commit/43682f794f42b37227694708dd3cf29567a9d907))
* merge v25.6.0 ([#2847](https://github.com/rivet-gg/rivet/issues/2847)) ([43682f7](https://github.com/rivet-gg/rivet/commit/43682f794f42b37227694708dd3cf29567a9d907))
* migrate redirects from file to Next.js config ([#2789](https://github.com/rivet-gg/rivet/issues/2789)) ([da00ff5](https://github.com/rivet-gg/rivet/commit/da00ff59ded66893ab68ff763cf2148c27e410b5))
* moved tailwind v2 -&gt; main config ([#2750](https://github.com/rivet-gg/rivet/issues/2750)) ([a1641eb](https://github.com/rivet-gg/rivet/commit/a1641eb3ef88cc65f69c744c35c3b0b420c956e7))
* **pegboard:** add workaround fetching image size when not using ats ([#2744](https://github.com/rivet-gg/rivet/issues/2744)) ([386da9f](https://github.com/rivet-gg/rivet/commit/386da9f61cefd6ef1c322e29022a368dac348ee5))
* release 25.5.3 ([a07577d](https://github.com/rivet-gg/rivet/commit/a07577d83a0ca8362f760fd108b461fd5ec6f965))
* release 25.6.0 ([287ffdb](https://github.com/rivet-gg/rivet/commit/287ffdbfb2a8d92c8980807ecfac4998ab1045e1))
* **release:** update version to 25.5.3 ([2380933](https://github.com/rivet-gg/rivet/commit/2380933a8a9730304e1347d5b936ff567db2ba16))
* **site:** add examples to landing page ([#2747](https://github.com/rivet-gg/rivet/issues/2747)) ([cb555da](https://github.com/rivet-gg/rivet/commit/cb555da48787648069726afc81a4a8e21972cc4e))
* **site:** doc new useActor properties ([#2808](https://github.com/rivet-gg/rivet/issues/2808)) ([db39a0d](https://github.com/rivet-gg/rivet/commit/db39a0d417f64055584c567c9f1ac6b9990d4780))
* **site:** update docs nav scrolling ([#2796](https://github.com/rivet-gg/rivet/issues/2796)) ([7e80a18](https://github.com/rivet-gg/rivet/commit/7e80a18bfae0e1bfa579581b23bf8c49c0502a9a))
* **site:** update docs on new lifecycle hooks ([#2843](https://github.com/rivet-gg/rivet/issues/2843)) ([f879623](https://github.com/rivet-gg/rivet/commit/f879623b871e4acafaffd31817b9386fb84ddce1))
* **site:** update react docs ([#2806](https://github.com/rivet-gg/rivet/issues/2806)) ([68cef01](https://github.com/rivet-gg/rivet/commit/68cef015f1e2e780850fa9ac3290aa8165956c07))
* tweak nav ([#2749](https://github.com/rivet-gg/rivet/issues/2749)) ([233cee9](https://github.com/rivet-gg/rivet/commit/233cee96ee420fcafca07f4330935d84441613a9))
* update actor-core repo path ([b3cb221](https://github.com/rivet-gg/rivet/commit/b3cb221b65d9549596cc4676565226394b6cc802))
* update clickhouse user query to dynamically bind subproperties ([#2715](https://github.com/rivet-gg/rivet/issues/2715)) ([ce74063](https://github.com/rivet-gg/rivet/commit/ce74063e925b04eb2c46f8568c8721efa97d548d))
* update cta ([#2738](https://github.com/rivet-gg/rivet/issues/2738)) ([443e63f](https://github.com/rivet-gg/rivet/commit/443e63ff5f34ec7b1a1150b54b9ead88cde7cd68))
* update download logs button to use export ([#2718](https://github.com/rivet-gg/rivet/issues/2718)) ([d01af0e](https://github.com/rivet-gg/rivet/commit/d01af0e8f96c0b8086f78cda970d6922b5ae5b15))
* update og image ([#2801](https://github.com/rivet-gg/rivet/issues/2801)) ([62b95a1](https://github.com/rivet-gg/rivet/commit/62b95a164adffb71ae399b3df5cca9dc62307cfd))

## [25.5.2](https://github.com/rivet-gg/rivet/compare/v25.5.1...v25.5.2) (2025-07-10)


### Features

* add skip_upgrade flag for actor deployments ([#2739](https://github.com/rivet-gg/rivet/issues/2739)) ([f3e1051](https://github.com/rivet-gg/rivet/commit/f3e1051a41dc4ed21adec3b7e2ec05beb9c06d5d))


### Bug Fixes

* fix ports ([#2729](https://github.com/rivet-gg/rivet/issues/2729)) ([e13ac80](https://github.com/rivet-gg/rivet/commit/e13ac80c95022dfa4f797a24cf2c52f35b3beb11))
* make artifact size changes not break wf ([#2726](https://github.com/rivet-gg/rivet/issues/2726)) ([17620f8](https://github.com/rivet-gg/rivet/commit/17620f80b8670c2487a60d6317830aa33c13295b))
* reduce cardinality of metrics ([#2691](https://github.com/rivet-gg/rivet/issues/2691)) ([e63f5c1](https://github.com/rivet-gg/rivet/commit/e63f5c1744b6304a7cab1952f57b2e44bb79b28d))
* **release:** fix generate api path ([c9926da](https://github.com/rivet-gg/rivet/commit/c9926da7f8901ca35b84208dbbfa6e4228d5ade7))
* **release:** fix replace install script version ([7e69fc0](https://github.com/rivet-gg/rivet/commit/7e69fc0026f4edb6d0e0e12add903b8784785bb2))
* **site:** fix generate api path ([73a05a5](https://github.com/rivet-gg/rivet/commit/73a05a5c76bb730b3e9a7d7220ca70042d095d4d))
* sqlite lock error ([#2672](https://github.com/rivet-gg/rivet/issues/2672)) ([e187b64](https://github.com/rivet-gg/rivet/commit/e187b64847f93eb79880a476da42861bc6ed06b8))


### Chores

* fix broken links ([#2734](https://github.com/rivet-gg/rivet/issues/2734)) ([a99678c](https://github.com/rivet-gg/rivet/commit/a99678ce3acd047647f2360c124f4f537636d324))
* release 25.5.2 ([d8ffbf0](https://github.com/rivet-gg/rivet/commit/d8ffbf0c8e198a650f0a5d94584ea0e85d3498a3))
* release 25.5.2 ([b210f39](https://github.com/rivet-gg/rivet/commit/b210f39177325b5b8cf4f70e18a89bf32ddb21f4))
* **release:** update version to 25.5.2 ([88d4760](https://github.com/rivet-gg/rivet/commit/88d4760834de2bd92a5d5be17eb1e67a5e019acc))
* **release:** update version to 25.5.2 ([5e03f9e](https://github.com/rivet-gg/rivet/commit/5e03f9eff94c706839889eaa1ec5a6e917d476fc))
* update docs for rivetkit ([#2732](https://github.com/rivet-gg/rivet/issues/2732)) ([f0243da](https://github.com/rivet-gg/rivet/commit/f0243da833e81f3257ae3944e107450a2ffabfc9))

## [25.5.1](https://github.com/rivet-gg/rivet/compare/v25.4.1...v25.5.1) (2025-07-03)


### Features

* **actors:** expose container system metrics ([#2664](https://github.com/rivet-gg/rivet/issues/2664)) ([c61646d](https://github.com/rivet-gg/rivet/commit/c61646d8657ce5e2775770d1b81cbf95b62480ff))
* add container platform comparison blog post ([#2639](https://github.com/rivet-gg/rivet/issues/2639)) ([9035ace](https://github.com/rivet-gg/rivet/commit/9035ace299c387573d4b5c8d1ecd177b03f14790))
* add frontend-hub service to docker-compose ([#2614](https://github.com/rivet-gg/rivet/issues/2614)) ([6c48e0f](https://github.com/rivet-gg/rivet/commit/6c48e0fc3f3a3c15cad11fb0515e184ab68c59a7))
* add non-interactive mode and route management options to CLI ([#2532](https://github.com/rivet-gg/rivet/issues/2532)) ([c729f94](https://github.com/rivet-gg/rivet/commit/c729f94efce02c80d664633e7a84c68b7f689554))
* add pb usage metrics, server state ([#2503](https://github.com/rivet-gg/rivet/issues/2503)) ([961149a](https://github.com/rivet-gg/rivet/commit/961149abe80432b94a21ef210c8ca33c4923e024))
* add retries to sql macros ([#2529](https://github.com/rivet-gg/rivet/issues/2529)) ([1f680de](https://github.com/rivet-gg/rivet/commit/1f680de444fc934b611f357cfcab8bbcff9e2a4a))
* **cli:** add `--env` to `rivet shell` command ([#2434](https://github.com/rivet-gg/rivet/issues/2434)) ([a0351bd](https://github.com/rivet-gg/rivet/commit/a0351bdccb9ed90245cd3f4972eb73f50878c212))
* **cli:** add `rivet kit endpoint` command ([#2653](https://github.com/rivet-gg/rivet/issues/2653)) ([5cdfcd2](https://github.com/rivet-gg/rivet/commit/5cdfcd209de5cb78bf4d003680db77216ec73f5a))
* **cli:** support passing env name to `rivet env select &lt;name&gt;` ([#2652](https://github.com/rivet-gg/rivet/issues/2652)) ([c04bdf7](https://github.com/rivet-gg/rivet/commit/c04bdf75e8d692fe848dbb3a787998da0468b963))
* **clusters:** add margin per pool ([#2543](https://github.com/rivet-gg/rivet/issues/2543)) ([dbe2a48](https://github.com/rivet-gg/rivet/commit/dbe2a485952f8de5ad9e63eb54c3aa95dc4de63f))
* db sh for workflows ([#2570](https://github.com/rivet-gg/rivet/issues/2570)) ([b3ef5c8](https://github.com/rivet-gg/rivet/commit/b3ef5c85e5130fac2359293e88b28939483e09d6))
* **examples:** multitenant using remote builds ([#2619](https://github.com/rivet-gg/rivet/issues/2619)) ([9b6888a](https://github.com/rivet-gg/rivet/commit/9b6888a8b5c55a47d4bcc13c017ccc9d0ae538a2))
* **guard:** add http ping endpoint to edge api hosts ([#2506](https://github.com/rivet-gg/rivet/issues/2506)) ([c33d254](https://github.com/rivet-gg/rivet/commit/c33d25498a90b85d0eb61c3788e3e13385970851))
* **guard:** support streaming responses ([#2667](https://github.com/rivet-gg/rivet/issues/2667)) ([90ab6ac](https://github.com/rivet-gg/rivet/commit/90ab6ac5e1a668a24cc3beb685bd63947273a4cc))
* **hub:** add actors billing preview ([#2466](https://github.com/rivet-gg/rivet/issues/2466)) ([6728f28](https://github.com/rivet-gg/rivet/commit/6728f286aee921e8456853c68d4cdbf01be1a714))
* **hub:** add loading indicator to actors and containers ([#2486](https://github.com/rivet-gg/rivet/issues/2486)) ([480e721](https://github.com/rivet-gg/rivet/commit/480e721009f0cadab87d9608251f2e5b9c7dc8be))
* **hub:** add metrics charts ([#2695](https://github.com/rivet-gg/rivet/issues/2695)) ([ab1f738](https://github.com/rivet-gg/rivet/commit/ab1f73877eae7eef3ebd4995b174c2af67ed8cf8))
* **hub:** improve get started guide ([#2496](https://github.com/rivet-gg/rivet/issues/2496)) ([509b929](https://github.com/rivet-gg/rivet/commit/509b929db4c81245708de8efd85500c6fd293819))
* **hub:** truncate actor tags when in a table ([#2497](https://github.com/rivet-gg/rivet/issues/2497)) ([e349d6e](https://github.com/rivet-gg/rivet/commit/e349d6ed45fcdb91ba59f0a21ebd16ec30aa04a5))
* improve deploy summary with RivetKit endpoint ([#2655](https://github.com/rivet-gg/rivet/issues/2655)) ([32a93da](https://github.com/rivet-gg/rivet/commit/32a93daf10d5b20d4139eac109ff4b5a146bd09a))
* **pegboard:** add draining state to alloc metrics ([#2565](https://github.com/rivet-gg/rivet/issues/2565)) ([befcf6d](https://github.com/rivet-gg/rivet/commit/befcf6d10fddff9576744e004615244eefdcb6bb))
* **pegboard:** add local image cache and overlay fs mounts ([#2586](https://github.com/rivet-gg/rivet/issues/2586)) ([da2b9e4](https://github.com/rivet-gg/rivet/commit/da2b9e47e48b9a32a7f047a59ce7ea43ac715d38))
* **pegboard:** add support for custom host entries ([#2627](https://github.com/rivet-gg/rivet/issues/2627)) ([898a9f8](https://github.com/rivet-gg/rivet/commit/898a9f8d6ba5410b04ac486fb74cd902eed37ce9))
* **pegboard:** expose rivet server from within containers for docker compose ([#2628](https://github.com/rivet-gg/rivet/issues/2628)) ([f872ff4](https://github.com/rivet-gg/rivet/commit/f872ff45b14b65256ef9b6815302e583b7dd453b))
* **site:** add error pages ([#2604](https://github.com/rivet-gg/rivet/issues/2604)) ([aa4e63f](https://github.com/rivet-gg/rivet/commit/aa4e63f4c8cab0578ab186ada3e2208d5520d2e5))
* **toolchain:** add rivetkit deployment support ([#2636](https://github.com/rivet-gg/rivet/issues/2636)) ([15f9a8e](https://github.com/rivet-gg/rivet/commit/15f9a8e2af8e981078287559fe6840c4d7db5641))
* **toolchain:** allow passing environment to functions ([#2624](https://github.com/rivet-gg/rivet/issues/2624)) ([e875d33](https://github.com/rivet-gg/rivet/commit/e875d3351b684d5d5863fc0815aa6a285c06e6bd))
* **toolchain:** build args support for remote builds ([#2583](https://github.com/rivet-gg/rivet/issues/2583)) ([5816b77](https://github.com/rivet-gg/rivet/commit/5816b77e1431230a5a5dd953db440cd940cb0ef4))
* **toolchain:** Implement dockerignore for remote builds ([#2590](https://github.com/rivet-gg/rivet/issues/2590)) ([34d4343](https://github.com/rivet-gg/rivet/commit/34d434375995821696a0afe7241a7ea5dad695b2))
* **toolchain:** remote build logs ([#2587](https://github.com/rivet-gg/rivet/issues/2587)) ([6579f85](https://github.com/rivet-gg/rivet/commit/6579f855eac7bc914441dd4c71bfbddf3c5ed6fd))
* **toolchain:** remote build method ([#2579](https://github.com/rivet-gg/rivet/issues/2579)) ([c9b525c](https://github.com/rivet-gg/rivet/commit/c9b525c051133119e23a3cb601db62bc48f141a8))


### Bug Fixes

* add cache to server list queries ([#2620](https://github.com/rivet-gg/rivet/issues/2620)) ([e0fc4c6](https://github.com/rivet-gg/rivet/commit/e0fc4c6210871da4c3c3b72278fe51bd48419dd5))
* add create ts to server list ([#2523](https://github.com/rivet-gg/rivet/issues/2523)) ([8f39a94](https://github.com/rivet-gg/rivet/commit/8f39a94519e8537f174a8a824c54b0402e64b2b4))
* add future/fdb metrics ([#2377](https://github.com/rivet-gg/rivet/issues/2377)) ([37250d5](https://github.com/rivet-gg/rivet/commit/37250d5e3dc7897fdd31d8f21ef6597b07abb15b))
* add logs to ws ([#2412](https://github.com/rivet-gg/rivet/issues/2412)) ([0ab685a](https://github.com/rivet-gg/rivet/commit/0ab685ae5ebef7310f48f519a3789d6c708b8bd6))
* add max by to topo queries ([#2645](https://github.com/rivet-gg/rivet/issues/2645)) ([c0a9c3b](https://github.com/rivet-gg/rivet/commit/c0a9c3b83ddc3abf6040488264cf600189154fb2))
* add metrics to ops and guard ([#2429](https://github.com/rivet-gg/rivet/issues/2429)) ([9cd808a](https://github.com/rivet-gg/rivet/commit/9cd808aed6ef45a0250d2c3b167c9965e035b755))
* add otel collector tail sampler ([#2411](https://github.com/rivet-gg/rivet/issues/2411)) ([026e21b](https://github.com/rivet-gg/rivet/commit/026e21b74bbc50dddc6014a658fe00a1a755be09))
* add udp test to system test ([#2417](https://github.com/rivet-gg/rivet/issues/2417)) ([91d820a](https://github.com/rivet-gg/rivet/commit/91d820a30cc87a9de90989f2c2a75a28f821167e))
* add vector http config to edge configs ([#2553](https://github.com/rivet-gg/rivet/issues/2553)) ([e54fd50](https://github.com/rivet-gg/rivet/commit/e54fd50795e61b41d3aac522d83f3e2ce4a43fdb))
* allow custom project for status monitor ([#2415](https://github.com/rivet-gg/rivet/issues/2415)) ([9f6a5a7](https://github.com/rivet-gg/rivet/commit/9f6a5a7950f07d1110bb36fc584d539c5133446b))
* **api:** handle wrapped operation errors ([#2602](https://github.com/rivet-gg/rivet/issues/2602)) ([d8ea7be](https://github.com/rivet-gg/rivet/commit/d8ea7be79e9df4089f1c4afc1f2813a01f71fb70))
* auto-create routes for RivetKit in non-interactive mode ([#2654](https://github.com/rivet-gg/rivet/issues/2654)) ([39fcfd9](https://github.com/rivet-gg/rivet/commit/39fcfd9c27b8172155b457171de2d3158ce124f3))
* **blog:** update cloudflare docs links ([#2641](https://github.com/rivet-gg/rivet/issues/2641)) ([9488a70](https://github.com/rivet-gg/rivet/commit/9488a70006ce10443e8c247889f05fff9ddd5406))
* **cache:** add traces ([#2489](https://github.com/rivet-gg/rivet/issues/2489)) ([b316d41](https://github.com/rivet-gg/rivet/commit/b316d41f2b3ae876e499273ba3c3aecdca27456b))
* change invalid atom value ([#2600](https://github.com/rivet-gg/rivet/issues/2600)) ([117f5dd](https://github.com/rivet-gg/rivet/commit/117f5dddd908d9c8faa63185bdce52831bf5ed7d))
* **cli:** internal_port u8 -&gt; u16 ([#2424](https://github.com/rivet-gg/rivet/issues/2424)) ([6d024f5](https://github.com/rivet-gg/rivet/commit/6d024f5066e0293554a221e1f72fd01e7aa7b1e6))
* **cluster:** dc workflow does not insert row on creation ([#2610](https://github.com/rivet-gg/rivet/issues/2610)) ([91fd539](https://github.com/rivet-gg/rivet/commit/91fd53934d8bee38cf9c9d1bd3db2f06d7dd4693))
* **cluster:** fix pbi autoscaling ([#2592](https://github.com/rivet-gg/rivet/issues/2592)) ([8bc423f](https://github.com/rivet-gg/rivet/commit/8bc423f9ed381b8ca809a67bbe55ea1ad4b0b3af))
* **cluster:** fix scaling logic and dedup metrics ([#2566](https://github.com/rivet-gg/rivet/issues/2566)) ([d632ace](https://github.com/rivet-gg/rivet/commit/d632ace04294b1fcdeba8316cdac96891a504f60))
* **container-runner:** fix leaked pipes ([#2473](https://github.com/rivet-gg/rivet/issues/2473)) ([121a4a4](https://github.com/rivet-gg/rivet/commit/121a4a4953e7978902417579a437b9e0bc22c157))
* correct memory safety vs correctness guarantees in blog post ([#2534](https://github.com/rivet-gg/rivet/issues/2534)) ([4f0663b](https://github.com/rivet-gg/rivet/commit/4f0663baceffd1c757100192f48716bc816016c8))
* disambiguate cluster_id &lt;-&gt; instance_id ([#2472](https://github.com/rivet-gg/rivet/issues/2472)) ([6fccebb](https://github.com/rivet-gg/rivet/commit/6fccebb9b140d666af84eb088b4f5a14e4bf304c))
* dont include installing servers in service discovery ([#2569](https://github.com/rivet-gg/rivet/issues/2569)) ([95da029](https://github.com/rivet-gg/rivet/commit/95da029b112219957267b30c4515fe4972510142))
* fix build cache key ([#2530](https://github.com/rivet-gg/rivet/issues/2530)) ([3c7f0b3](https://github.com/rivet-gg/rivet/commit/3c7f0b388870df80117808d36b37d75df4d8ce91))
* fix client leaked pipes ([#2474](https://github.com/rivet-gg/rivet/issues/2474)) ([39220d3](https://github.com/rivet-gg/rivet/commit/39220d35a182aa99f401646169a61e96d8d89293))
* fix copy pasta typo in route propegation rip ([#2536](https://github.com/rivet-gg/rivet/issues/2536)) ([639afa9](https://github.com/rivet-gg/rivet/commit/639afa977761b38efbac9e836da788069f452610))
* fix edge dc eligible calculation ([#2567](https://github.com/rivet-gg/rivet/issues/2567)) ([72c45a0](https://github.com/rivet-gg/rivet/commit/72c45a03185bacec658d13209322a9d2f092a43a))
* fix fdb cli, wf pulling ([#2418](https://github.com/rivet-gg/rivet/issues/2418)) ([a8d5b7a](https://github.com/rivet-gg/rivet/commit/a8d5b7a1747245c9ecd5cbe46d0680a7095b3e17))
* fix metrics ([#2416](https://github.com/rivet-gg/rivet/issues/2416)) ([b2f092d](https://github.com/rivet-gg/rivet/commit/b2f092d96dc385cebb77c4e585f6764ca3ad96fc))
* fix npm package naming ([#2679](https://github.com/rivet-gg/rivet/issues/2679)) ([6c6bcb2](https://github.com/rivet-gg/rivet/commit/6c6bcb238f3d5a83c2635540cb338af5a71124a1))
* fix pb metrics standalone ([#2518](https://github.com/rivet-gg/rivet/issues/2518)) ([eeb9b08](https://github.com/rivet-gg/rivet/commit/eeb9b08ab5c587dd4d480162396e573a4208a71e))
* fix release script to use endpoint url flag instead of env var for aws ([#2685](https://github.com/rivet-gg/rivet/issues/2685)) ([0c3e948](https://github.com/rivet-gg/rivet/commit/0c3e9485f09025285352c20ccd5f314103d17cf3))
* **guard:** add cache clearing to ws retry, increase ws retries ([#2448](https://github.com/rivet-gg/rivet/issues/2448)) ([65aa627](https://github.com/rivet-gg/rivet/commit/65aa627a5b62205e8d97bcc6a2acc3c042c466f0))
* **guard:** add metrics ([#2480](https://github.com/rivet-gg/rivet/issues/2480)) ([3cee110](https://github.com/rivet-gg/rivet/commit/3cee11016f4dabd74651e294952dafd270add2bc))
* **guard:** add more tokio runtime metrics, remove labels from metrics ([#2485](https://github.com/rivet-gg/rivet/issues/2485)) ([5b84ccc](https://github.com/rivet-gg/rivet/commit/5b84ccc942d01ef77867e26d2036f145ca564b49))
* **guard:** allow routing to path-based endpoints via any hostname for dev clusters ([#2469](https://github.com/rivet-gg/rivet/issues/2469)) ([ba7a88d](https://github.com/rivet-gg/rivet/commit/ba7a88dc3f4afac48d9addeae3c7726e894a4ee9))
* **guard:** dont route to installing api nodes ([#2564](https://github.com/rivet-gg/rivet/issues/2564)) ([86c6e41](https://github.com/rivet-gg/rivet/commit/86c6e41d1fb3458ec58513349fa66c0ec2563302))
* **guard:** replace internal caches with moka ([#2481](https://github.com/rivet-gg/rivet/issues/2481)) ([c88362c](https://github.com/rivet-gg/rivet/commit/c88362c16c318bde3c905a4d03fb7270b505ccfd))
* **guard:** return error response on router error ([#2540](https://github.com/rivet-gg/rivet/issues/2540)) ([cad72fe](https://github.com/rivet-gg/rivet/commit/cad72fe7f7424a59aaef544eb57cd4311130c17a))
* hide functions in containers view ([#2594](https://github.com/rivet-gg/rivet/issues/2594)) ([258dd58](https://github.com/rivet-gg/rivet/commit/258dd583b42f4d134de3ef8863f85d155cff92a6))
* hide functions in containers view ([#2597](https://github.com/rivet-gg/rivet/issues/2597)) ([097b545](https://github.com/rivet-gg/rivet/commit/097b5451e33a60fc871ce3d782acfbec7103d5f2))
* **hub:** close modal after creating a team ([#2498](https://github.com/rivet-gg/rivet/issues/2498)) ([85a5c84](https://github.com/rivet-gg/rivet/commit/85a5c84d2702b56ec8ed4ecdef2a383c325ed7bc))
* **hub:** containers screen overflow issues ([#2545](https://github.com/rivet-gg/rivet/issues/2545)) ([5ba15dd](https://github.com/rivet-gg/rivet/commit/5ba15dd483fcae4d826f8b1d57e58effdb9e474b))
* **hub:** do not force users to go through onboarding when they already have projects created ([#2499](https://github.com/rivet-gg/rivet/issues/2499)) ([1135677](https://github.com/rivet-gg/rivet/commit/1135677382e34690722ec15abdf334b994983e28))
* **hub:** generate stripe sessions links aot ([#2494](https://github.com/rivet-gg/rivet/issues/2494)) ([e3f8c7f](https://github.com/rivet-gg/rivet/commit/e3f8c7f4d1fa13bb8b716e9287501afa964b0e35))
* **hub:** invalidate billing calculation after chagning the billing plan ([#2547](https://github.com/rivet-gg/rivet/issues/2547)) ([c3a8a1e](https://github.com/rivet-gg/rivet/commit/c3a8a1eba531bcd9c4d550a0f3eef9765ad073fa))
* **hub:** make sure kill timeout is properly displayed ([#2488](https://github.com/rivet-gg/rivet/issues/2488)) ([d8b1a65](https://github.com/rivet-gg/rivet/commit/d8b1a650a25ea9ea6d65165761b21a9708746e6b))
* **hub:** minor fixes to filters and sidebar ([#2439](https://github.com/rivet-gg/rivet/issues/2439)) ([a2aeafd](https://github.com/rivet-gg/rivet/commit/a2aeafd1ec75836cc386d35bc27bf7c8da2908e1))
* **hub:** minor polishing ([#2444](https://github.com/rivet-gg/rivet/issues/2444)) ([b2e5f8f](https://github.com/rivet-gg/rivet/commit/b2e5f8fe65a2575f7322cb801d26543f3c1e4a7e))
* **hub:** missing message in logs txt file ([#2555](https://github.com/rivet-gg/rivet/issues/2555)) ([f26f599](https://github.com/rivet-gg/rivet/commit/f26f599b0bd17d8c9e4f909958917adc0396ef33))
* **hub:** missing message in logs txt file ([#2557](https://github.com/rivet-gg/rivet/issues/2557)) ([34d6973](https://github.com/rivet-gg/rivet/commit/34d697306b8a9ab0180e9f2553a0a8a56c779ef2))
* **hub:** refactor validation for functions ([#2647](https://github.com/rivet-gg/rivet/issues/2647)) ([bc97b4a](https://github.com/rivet-gg/rivet/commit/bc97b4aeab9ba626a1f019655f9665633f2c5801))
* **hub:** two cmd+k pannels is too much ([#2491](https://github.com/rivet-gg/rivet/issues/2491)) ([dc35946](https://github.com/rivet-gg/rivet/commit/dc35946be370e360a79fda5edad05510706c899f))
* **hub:** use proper logo in footer ([#2495](https://github.com/rivet-gg/rivet/issues/2495)) ([be885f6](https://github.com/rivet-gg/rivet/commit/be885f69680eac431f5e08743b44008ff83612a0))
* improve error handling in status check ([#2563](https://github.com/rivet-gg/rivet/issues/2563)) ([6b786d0](https://github.com/rivet-gg/rivet/commit/6b786d0bf04c55518ea24974633106a302acdf32))
* increase log limits ([#2646](https://github.com/rivet-gg/rivet/issues/2646)) ([fa3f299](https://github.com/rivet-gg/rivet/commit/fa3f299689f305389b4b9f7e80c8267b70515099))
* make service discovery use a single client ([#2621](https://github.com/rivet-gg/rivet/issues/2621)) ([5a8daa4](https://github.com/rivet-gg/rivet/commit/5a8daa4da9374f5d0462f2627a84802dfae84c52))
* missing existence check ([#2599](https://github.com/rivet-gg/rivet/issues/2599)) ([ce4e2b0](https://github.com/rivet-gg/rivet/commit/ce4e2b0fd4f6bf699b95de67ec9f36869caca9e0))
* **pb:** fix actor reschedule with wrong image ([#2428](https://github.com/rivet-gg/rivet/issues/2428)) ([ad8ea6b](https://github.com/rivet-gg/rivet/commit/ad8ea6b9fba62a534a2e6905f57165072e7f990d))
* **pegboard:** continue exporting metrics even if fetching actor fails ([#2520](https://github.com/rivet-gg/rivet/issues/2520)) ([9ca20cf](https://github.com/rivet-gg/rivet/commit/9ca20cf8f2b2167cdfcbf500d9d09e4a16089c6c))
* **pegboard:** fix allocation metrics ([#2568](https://github.com/rivet-gg/rivet/issues/2568)) ([9b454d7](https://github.com/rivet-gg/rivet/commit/9b454d738c7257f9f1bc05a6db24352edb3def3e))
* **pegboard:** fix collecting metrics for actors without workflows ([#2519](https://github.com/rivet-gg/rivet/issues/2519)) ([9af8aa1](https://github.com/rivet-gg/rivet/commit/9af8aa149e9af3796b065429bb1f7730ff5db4f8))
* **pegboard:** fix container runner orphaning ([#2575](https://github.com/rivet-gg/rivet/issues/2575)) ([88ec694](https://github.com/rivet-gg/rivet/commit/88ec69445a733c80c01c548043993dedf97a12f2))
* **pegboard:** fix enabling root users ([#2572](https://github.com/rivet-gg/rivet/issues/2572)) ([edfe98f](https://github.com/rivet-gg/rivet/commit/edfe98f7f35e5c5c6eea4a6d43542d7f28bc7230))
* **pegboard:** fix hosts file not getting written correctly ([#2626](https://github.com/rivet-gg/rivet/issues/2626)) ([1a51658](https://github.com/rivet-gg/rivet/commit/1a5165809fb753dc5dab8736bbfb65868eb002cf))
* **pegboard:** fix netns path bug ([#2431](https://github.com/rivet-gg/rivet/issues/2431)) ([36252ec](https://github.com/rivet-gg/rivet/commit/36252ec8694939c3b50cdcbc96c9379816f89484))
* **pegboard:** fix prewarm logic ([#2409](https://github.com/rivet-gg/rivet/issues/2409)) ([abdda32](https://github.com/rivet-gg/rivet/commit/abdda32dce8bf514cc98f076ccd266441d2a60a3))
* **pegboard:** increase actor log ttl to 14 days ([#2544](https://github.com/rivet-gg/rivet/issues/2544)) ([925250a](https://github.com/rivet-gg/rivet/commit/925250a337ea37bf9da91547df4fb36ad09aa7ca))
* **pegboard:** increase signal timeout, add check for actor exit ([#2538](https://github.com/rivet-gg/rivet/issues/2538)) ([32b516a](https://github.com/rivet-gg/rivet/commit/32b516aa0cac6e1f8ecf59960e6e493b612cd0c2))
* **pegboard:** revise actor rescheduling algorithm, add client metrics ([#2531](https://github.com/rivet-gg/rivet/issues/2531)) ([e3b6c06](https://github.com/rivet-gg/rivet/commit/e3b6c0650f90aba981c95dec1613f69f82bb2c81))
* **pegboard:** wait for actor allocation not creation ([#2539](https://github.com/rivet-gg/rivet/issues/2539)) ([6fb6e53](https://github.com/rivet-gg/rivet/commit/6fb6e5304d8edfa15d7743dbf6bfde4588f0551c))
* remove iptables filter after CNI ([#2421](https://github.com/rivet-gg/rivet/issues/2421)) ([631965b](https://github.com/rivet-gg/rivet/commit/631965b1b219e6224c0bf616caeb94bc81492398))
* **site:** broken sales / missing support page ([#2559](https://github.com/rivet-gg/rivet/issues/2559)) ([db6b40b](https://github.com/rivet-gg/rivet/commit/db6b40b652573b4c471fa4d790c35586a6ffcb05))
* **site:** broken sales / missing support page ([#2561](https://github.com/rivet-gg/rivet/issues/2561)) ([3b81ca3](https://github.com/rivet-gg/rivet/commit/3b81ca3a51341fef691e90b0d5218eb380e43f2a))
* **studio:** items flows incorreclty on getting started screen ([#2542](https://github.com/rivet-gg/rivet/issues/2542)) ([d6bc547](https://github.com/rivet-gg/rivet/commit/d6bc54706d4549ea51e9079c0a04783c1aaee25a))
* swap status monitor dns resolver with native resolver ([#2432](https://github.com/rivet-gg/rivet/issues/2432)) ([d450f12](https://github.com/rivet-gg/rivet/commit/d450f1256077bd0528e950b9551d8c5919f5771d))
* **toolchain:** attach tags to function workers ([#2617](https://github.com/rivet-gg/rivet/issues/2617)) ([334e2c6](https://github.com/rivet-gg/rivet/commit/334e2c6d7aa4ae5ba8e38988b77d989c9b06d08a))
* **toolchain:** creating ci env no longer fails first try ([#2618](https://github.com/rivet-gg/rivet/issues/2618)) ([da726ec](https://github.com/rivet-gg/rivet/commit/da726ec51c123ecf9c36adaa7f264edcc819d04e))
* **toolchain:** fix creating route failing silently ([#2601](https://github.com/rivet-gg/rivet/issues/2601)) ([7f9bf14](https://github.com/rivet-gg/rivet/commit/7f9bf141811b5f737d7f33280ee557106708c63c))
* **toolchain:** fix fallback package manager to npm ([#2650](https://github.com/rivet-gg/rivet/issues/2650)) ([a256a69](https://github.com/rivet-gg/rivet/commit/a256a6901ea92a2187a68d1abb82a2c2f751c997))
* **toolchain:** fix symlinks &gt; 100 chars not being archived correctly ([#2651](https://github.com/rivet-gg/rivet/issues/2651)) ([8e37d4c](https://github.com/rivet-gg/rivet/commit/8e37d4cb4de2044032acb9ba80f0806f1fd65526))
* uncomment function type filter in ACTORS_FILTER ([#2533](https://github.com/rivet-gg/rivet/issues/2533)) ([f7d97c9](https://github.com/rivet-gg/rivet/commit/f7d97c9715c16a8a404e9cde8d9e04a401b907ef))
* update @rivet-gg/components path in tailwind config ([#2613](https://github.com/rivet-gg/rivet/issues/2613)) ([4033c74](https://github.com/rivet-gg/rivet/commit/4033c74fd487f8b1d6671120ee66acb2c3d70587))
* update PowerShell version pattern in install docs ([#2703](https://github.com/rivet-gg/rivet/issues/2703)) ([93e3d68](https://github.com/rivet-gg/rivet/commit/93e3d68792e99ff992b5019cbb101e70555af583))
* update version references in install documentation ([#2708](https://github.com/rivet-gg/rivet/issues/2708)) ([e7d4031](https://github.com/rivet-gg/rivet/commit/e7d4031f886425d19c549705b5309bb436155da2))
* use real errors for actor logs ([#2410](https://github.com/rivet-gg/rivet/issues/2410)) ([6134113](https://github.com/rivet-gg/rivet/commit/61341130281fbf6eb6e012757ebb3ec797e07749))


### Documentation

* add linear-agent-starter example ([#2482](https://github.com/rivet-gg/rivet/issues/2482)) ([e13e6e9](https://github.com/rivet-gg/rivet/commit/e13e6e95c56ea63bc73312fa7d01a647412ac507))
* add memory & CPU overcommit section to containers quickstart ([#2693](https://github.com/rivet-gg/rivet/issues/2693)) ([cc10787](https://github.com/rivet-gg/rivet/commit/cc10787e5e55a4c56a9e520ce0bb2282864901ef))
* clarify default port in Rivet Functions ([#2608](https://github.com/rivet-gg/rivet/issues/2608)) ([335088d](https://github.com/rivet-gg/rivet/commit/335088d0e7b38be5d029d52556aa8ad8e101b344))
* update code for functions example ([c103001](https://github.com/rivet-gg/rivet/commit/c103001087ec87b8286bed1a909b44bab6228ebc))


### Code Refactoring

* **hub:** increase level of error reporting to catch cancelled errors ([#2492](https://github.com/rivet-gg/rivet/issues/2492)) ([2cc1e6e](https://github.com/rivet-gg/rivet/commit/2cc1e6e11f2da6fe97e022a67ded35a49f76a69e))
* **hub:** make sure user can create actors when manager is present ([#2493](https://github.com/rivet-gg/rivet/issues/2493)) ([2eb6885](https://github.com/rivet-gg/rivet/commit/2eb688553bf7998e99f251556d2ceae918ef4ff8))
* **hub:** polishing ([#2420](https://github.com/rivet-gg/rivet/issues/2420)) ([677643e](https://github.com/rivet-gg/rivet/commit/677643ecea49a1363355bcde6f7261f300ce4ac4))
* **hub:** remove instance count for functions ([#2541](https://github.com/rivet-gg/rivet/issues/2541)) ([fa5deef](https://github.com/rivet-gg/rivet/commit/fa5deefd70cb473f6be397598ee7cb8947209eb9))
* **hub:** responsive actors table ([#2468](https://github.com/rivet-gg/rivet/issues/2468)) ([6549759](https://github.com/rivet-gg/rivet/commit/65497594024d3adce86d628fced9ed27eddec471))
* **hub:** sugar coat config tab ([#2490](https://github.com/rivet-gg/rivet/issues/2490)) ([97d6d62](https://github.com/rivet-gg/rivet/commit/97d6d62be4e979560269562da70704978c83f6d4))
* replace unsafe File::from_raw_fd with safe File::from ([#2479](https://github.com/rivet-gg/rivet/issues/2479)) ([5c87dae](https://github.com/rivet-gg/rivet/commit/5c87daeb6d548e25d9815054410106a9d313db78))
* use bail_with! macro for actor metrics invalid interval ([#2669](https://github.com/rivet-gg/rivet/issues/2669)) ([9e3dbb1](https://github.com/rivet-gg/rivet/commit/9e3dbb1e18514923c55287898e4f061358c494d9))


### Chores

* add --non-interactive flag to rivet deploy commands ([#2658](https://github.com/rivet-gg/rivet/issues/2658)) ([b8b80e3](https://github.com/rivet-gg/rivet/commit/b8b80e3e5f27046c3270c71b305b81ac9d1af20d))
* add ahrefs analytics ([#2675](https://github.com/rivet-gg/rivet/issues/2675)) ([9f50c70](https://github.com/rivet-gg/rivet/commit/9f50c70dd099e67bf1499ea0ce90ba8802bf4bd2))
* add ahrefs analytics to site ([#2677](https://github.com/rivet-gg/rivet/issues/2677)) ([5e9b473](https://github.com/rivet-gg/rivet/commit/5e9b473166755b91b2b4dacaa60d568a76f36ec3))
* add cargo fmt precommit hook ([#2666](https://github.com/rivet-gg/rivet/issues/2666)) ([c90883d](https://github.com/rivet-gg/rivet/commit/c90883dad9feeb04d19761235f19de059909c287))
* add Linux ARM64 and Windows build targets ([#2551](https://github.com/rivet-gg/rivet/issues/2551)) ([7ea58ca](https://github.com/rivet-gg/rivet/commit/7ea58caab715f50e06a0f6a9c7aabdc4debf362a))
* add open-source durable objects to landing page ([#2514](https://github.com/rivet-gg/rivet/issues/2514)) ([941b6ac](https://github.com/rivet-gg/rivet/commit/941b6ace404cabecfd41277917dd86d0657f4c63))
* add standalone toolchain build scripts ([#2400](https://github.com/rivet-gg/rivet/issues/2400)) ([7e3b07e](https://github.com/rivet-gg/rivet/commit/7e3b07ec2c41e00006d105b7e0e56d898e70338f))
* auto-generate dockerfile for rivetkit ([#2659](https://github.com/rivet-gg/rivet/issues/2659)) ([1ad2a2a](https://github.com/rivet-gg/rivet/commit/1ad2a2a7ebb595888b8aed8096a684fc5a0f4a69))
* cargo fmt ([#2665](https://github.com/rivet-gg/rivet/issues/2665)) ([3a8cb86](https://github.com/rivet-gg/rivet/commit/3a8cb86b0029c068e406f06ae1ce3e72d11e0fcf))
* change otel collector policies ([#2436](https://github.com/rivet-gg/rivet/issues/2436)) ([bb8ccd5](https://github.com/rivet-gg/rivet/commit/bb8ccd548353180820ade4253ac7a8d249bc7aff))
* **cli:** allow RIVET_CLI_VERSION override for installed CLI version ([#2426](https://github.com/rivet-gg/rivet/issues/2426)) ([ecdd6c8](https://github.com/rivet-gg/rivet/commit/ecdd6c8bce5d7d4ab364dae2a18ccb5b0a52e7e9))
* disable OTEL by default in guard service ([#2522](https://github.com/rivet-gg/rivet/issues/2522)) ([1e6fef3](https://github.com/rivet-gg/rivet/commit/1e6fef392aac4fcad481d3914379827fa10b6e8e))
* **examples:** multitenant deploys example ([#2527](https://github.com/rivet-gg/rivet/issues/2527)) ([183ca27](https://github.com/rivet-gg/rivet/commit/183ca276a76d93438bb25544265d53c93d9356d3))
* expose namespace in otel attributes ([#2501](https://github.com/rivet-gg/rivet/issues/2501)) ([b208feb](https://github.com/rivet-gg/rivet/commit/b208febfc829ff14295f56d0014b015515e930b5))
* fill out remaining content for landing page ([#2413](https://github.com/rivet-gg/rivet/issues/2413)) ([e0fecf1](https://github.com/rivet-gg/rivet/commit/e0fecf12fc3064626798de9c34b055d69f3476df))
* fix broadcast req error logs ([#2419](https://github.com/rivet-gg/rivet/issues/2419)) ([5f2cfaa](https://github.com/rivet-gg/rivet/commit/5f2cfaa0db51009c6029737fe4be679795732763))
* fix dc wf ([#2552](https://github.com/rivet-gg/rivet/issues/2552)) ([9e9363e](https://github.com/rivet-gg/rivet/commit/9e9363e3f519a2a29734bccf57cb3c50de4ee689))
* fix guard metrics ([#2504](https://github.com/rivet-gg/rivet/issues/2504)) ([fa2f84f](https://github.com/rivet-gg/rivet/commit/fa2f84f0ea29cbd0d1295da2ec14368380117b1b))
* fix landing page links ([#2452](https://github.com/rivet-gg/rivet/issues/2452)) ([24a03dc](https://github.com/rivet-gg/rivet/commit/24a03dc2929ad1058bec72bc0e214862a68dfd28))
* fix links in linear agent blog post ([f0ce79f](https://github.com/rivet-gg/rivet/commit/f0ce79f261c0632296b6b4c01463b479ccc207e8))
* fix peer deps ([520b83c](https://github.com/rivet-gg/rivet/commit/520b83cbd5eed8ec8462a34e584aea21db00e92c))
* fix typo ([#2516](https://github.com/rivet-gg/rivet/issues/2516)) ([9fdd868](https://github.com/rivet-gg/rivet/commit/9fdd86826def1803b2ebb5fd5166aa6237b1fe01))
* fix update version ([#2686](https://github.com/rivet-gg/rivet/issues/2686)) ([25969da](https://github.com/rivet-gg/rivet/commit/25969da2ef25432e813f9627281bfc31e1b2b371))
* fixes to pricing page ([#2459](https://github.com/rivet-gg/rivet/issues/2459)) ([a207c1f](https://github.com/rivet-gg/rivet/commit/a207c1fc5976532fd0a1346fb883f8fa79da2d89))
* fmt ([#2706](https://github.com/rivet-gg/rivet/issues/2706)) ([df3292a](https://github.com/rivet-gg/rivet/commit/df3292a7c51d0499f168549d5f4d1434356b11e9))
* functions blog ([#2581](https://github.com/rivet-gg/rivet/issues/2581)) ([d993c81](https://github.com/rivet-gg/rivet/commit/d993c81e07de784fbe4add79f4062d88d2f5aa3d))
* **guard:** fix log levels ([#2580](https://github.com/rivet-gg/rivet/issues/2580)) ([c1c57ff](https://github.com/rivet-gg/rivet/commit/c1c57ffe000ff4bbc24f5693eb13cf06e626766d))
* **hub:** update actor framework filters for rivetkit ([#2656](https://github.com/rivet-gg/rivet/issues/2656)) ([fa1f776](https://github.com/rivet-gg/rivet/commit/fa1f77683dd5a32ad58d3716b9b9e1008206d393))
* increase file watch limit for rivet guard ([#2521](https://github.com/rivet-gg/rivet/issues/2521)) ([38528db](https://github.com/rivet-gg/rivet/commit/38528dbbc7658f767d7343c7dbe1f5b5233ef964))
* new landing page ([#2369](https://github.com/rivet-gg/rivet/issues/2369)) ([cd66a6d](https://github.com/rivet-gg/rivet/commit/cd66a6d6f7ebe99aa52403b646952ea1a0b78fbe))
* new og image ([#2450](https://github.com/rivet-gg/rivet/issues/2450)) ([14c45eb](https://github.com/rivet-gg/rivet/commit/14c45ebabf0e7da3edabfcca7315f6137273272f))
* page description optimization ([#2584](https://github.com/rivet-gg/rivet/issues/2584)) ([b6524d9](https://github.com/rivet-gg/rivet/commit/b6524d90dcc762540acc6c22fa56750920784531))
* **pegboard:** fix local cache bugs ([#2596](https://github.com/rivet-gg/rivet/issues/2596)) ([496c391](https://github.com/rivet-gg/rivet/commit/496c391b8c1c5822fe042fa9ca8ab89c4caeb12e))
* **pegboard:** send artifact image size from workflow instead of fetching with HEAD ([#2612](https://github.com/rivet-gg/rivet/issues/2612)) ([4969582](https://github.com/rivet-gg/rivet/commit/49695829c9548da58d3151bce36aac6f3e4af8c8))
* pricing update ([#2455](https://github.com/rivet-gg/rivet/issues/2455)) ([efcd3fd](https://github.com/rivet-gg/rivet/commit/efcd3fdaa1618c55e57a5fa394ac10af7019a72e))
* pricing update ([#2591](https://github.com/rivet-gg/rivet/issues/2591)) ([b16d098](https://github.com/rivet-gg/rivet/commit/b16d098922fc598326288282f934b2095b6d151a))
* re-enable building windows x86 toolchain ([#2616](https://github.com/rivet-gg/rivet/issues/2616)) ([b03c6f7](https://github.com/rivet-gg/rivet/commit/b03c6f7d638e90e92b83dbeee445f9bc73188170))
* reduce actor metrics poll interval and increase data points ([#2670](https://github.com/rivet-gg/rivet/issues/2670)) ([c6c6948](https://github.com/rivet-gg/rivet/commit/c6c6948b6c7bf77dd0a4f80f34990aff8d1c87b2))
* release 25.4.2 ([3924250](https://github.com/rivet-gg/rivet/commit/392425013fac42001fdc4f5176e393ca593e68c1))
* release 25.4.2 ([dc4c2cf](https://github.com/rivet-gg/rivet/commit/dc4c2cfd908503d046635bd45cc918e4f0e9c7fa))
* release 25.5.0 ([80902bc](https://github.com/rivet-gg/rivet/commit/80902bcf3867352a5e39330cd4247bb14910c663))
* release 25.5.1 ([3671810](https://github.com/rivet-gg/rivet/commit/3671810d6e9bd65389a78c87e6ddb3230017a4e5))
* release 25.5.1 ([3528157](https://github.com/rivet-gg/rivet/commit/3528157a6988ac4865b5bbacc32247f58b38e935))
* **release:** update version to 25.4.2 ([e6a665f](https://github.com/rivet-gg/rivet/commit/e6a665fca65f8593f9e59ccb409146f4ba5d3013))
* **release:** update version to 25.4.2 ([89d12ce](https://github.com/rivet-gg/rivet/commit/89d12ce6fdf6a9f404d9dd271adc9e58904b2752))
* **release:** update version to 25.5.0 ([08cbf90](https://github.com/rivet-gg/rivet/commit/08cbf904d7493ec50981a98b64e088fe07a0715e))
* **release:** update version to 25.5.1 ([d04aeea](https://github.com/rivet-gg/rivet/commit/d04aeea8260d07f01cc0835fbb19ffbdc6430f8e))
* **release:** update version to 25.5.1 ([5f350ea](https://github.com/rivet-gg/rivet/commit/5f350ea64505fc80156e901b8252e6b6a5667cbe))
* remove isolate-v8-runner from Docker repos ([#2684](https://github.com/rivet-gg/rivet/issues/2684)) ([fc15db0](https://github.com/rivet-gg/rivet/commit/fc15db0e7178f367d85050b4fba27f014f61fbc2))
* remove old references to matchmaker on website for seo ([#2511](https://github.com/rivet-gg/rivet/issues/2511)) ([8ccd8a1](https://github.com/rivet-gg/rivet/commit/8ccd8a1f357b5cc63aad94741ac3ab555ce9c9c6))
* remove outdated better stack integration ([#2680](https://github.com/rivet-gg/rivet/issues/2680)) ([63758a3](https://github.com/rivet-gg/rivet/commit/63758a3666fa31949390429606f677267fd83498))
* replace `hub-embed` with proxy to 127.0.0.1:5080 for /ui/ path in dev ([#2615](https://github.com/rivet-gg/rivet/issues/2615)) ([7e7c582](https://github.com/rivet-gg/rivet/commit/7e7c582fd9603ac39a8cfa17487dbd18b8820747))
* revert billing ([#2549](https://github.com/rivet-gg/rivet/issues/2549)) ([afbbfe3](https://github.com/rivet-gg/rivet/commit/afbbfe352de2c3076eff5cc67913ef859c6712df))
* rivetkit blog ([#2681](https://github.com/rivet-gg/rivet/issues/2681)) ([e526230](https://github.com/rivet-gg/rivet/commit/e526230a5fbcce9f9ed3d66051356b8f422b4b01))
* **site:** remove actor-core dep ([d4205a9](https://github.com/rivet-gg/rivet/commit/d4205a94774adb73208d40bf4e31fc30fe8a0329))
* **site:** remove deprecated errors ([#2606](https://github.com/rivet-gg/rivet/issues/2606)) ([fde245d](https://github.com/rivet-gg/rivet/commit/fde245d5b91a905ed65ccf5c7f991b3aea2d3c1e))
* support pegboard container runner inside docker compose ([#2611](https://github.com/rivet-gg/rivet/issues/2611)) ([0b74f0d](https://github.com/rivet-gg/rivet/commit/0b74f0d29a91a069baf9c59963c570c40510cfef))
* switch from snake_case to camelCase in config serialization ([#2657](https://github.com/rivet-gg/rivet/issues/2657)) ([6383f64](https://github.com/rivet-gg/rivet/commit/6383f640fa72ceaae4f1ba50aa066ca0cb96b58a))
* testing ci ([13a989c](https://github.com/rivet-gg/rivet/commit/13a989c811a7f14dbc4c8d1f4c1a421970186971))
* tweak wording for containers ([7ab4538](https://github.com/rivet-gg/rivet/commit/7ab45380f78d18e22d8aeae000b85da929b3fde9))
* update default port to 6420 ([#2660](https://github.com/rivet-gg/rivet/issues/2660)) ([12330f4](https://github.com/rivet-gg/rivet/commit/12330f49fb9c89812fffb314b20859892118f461))
* update deno to fix outgoing websockets ([#2446](https://github.com/rivet-gg/rivet/issues/2446)) ([a050f1e](https://github.com/rivet-gg/rivet/commit/a050f1ee4e1b02b9caf9605d3eb37ee9e4bf5150))
* update docs for new cli & rivetkit ([#2683](https://github.com/rivet-gg/rivet/issues/2683)) ([9ef0ac4](https://github.com/rivet-gg/rivet/commit/9ef0ac4ea908ddb0594c492a5cd40b59a567ee7d))
* update examples ([#2461](https://github.com/rivet-gg/rivet/issues/2461)) ([d3916f3](https://github.com/rivet-gg/rivet/commit/d3916f31050ab4842aa224be41bf13fc018079c6))
* update pricing buttons ([#2524](https://github.com/rivet-gg/rivet/issues/2524)) ([fe5fde3](https://github.com/rivet-gg/rivet/commit/fe5fde36e9c92c6282a2ebd5cb4fc4645e19354b))
* update styling on sales ([#2463](https://github.com/rivet-gg/rivet/issues/2463)) ([7855096](https://github.com/rivet-gg/rivet/commit/7855096e3d4cc6f31b8925794facc49e651211ce))
* update workspace ([#2505](https://github.com/rivet-gg/rivet/issues/2505)) ([a3cb3ec](https://github.com/rivet-gg/rivet/commit/a3cb3ecf120c2a08243813c0af17990fed3415c1))
* updated graphics for new landing page ([#2398](https://github.com/rivet-gg/rivet/issues/2398)) ([e657b90](https://github.com/rivet-gg/rivet/commit/e657b90e51e04a1630bb9d3ac9c2c64c905b7673))
* updated links ([#2577](https://github.com/rivet-gg/rivet/issues/2577)) ([33da3fa](https://github.com/rivet-gg/rivet/commit/33da3fa89a7445770ba022c1202e245b69e490f0))
* updated scenarios Pricing ([#2457](https://github.com/rivet-gg/rivet/issues/2457)) ([9fe7c47](https://github.com/rivet-gg/rivet/commit/9fe7c47701d5863819ee201b2953648e51854007))
* **vector:** add vector pipeline to ship clickhouse events from the edge ([#2526](https://github.com/rivet-gg/rivet/issues/2526)) ([ebbe9f7](https://github.com/rivet-gg/rivet/commit/ebbe9f7d08ad6fbc66657ee37a270bafe9674f90))
* writing rivet guard blog post ([#2509](https://github.com/rivet-gg/rivet/issues/2509)) ([1c35f83](https://github.com/rivet-gg/rivet/commit/1c35f8345d50c66c854c7509b17dd85b248ce09b))

## [25.4.1](https://github.com/rivet-gg/rivet/compare/v25.4.0...v25.4.1) (2025-04-26)


### Bug Fixes

* **cli:** fix cli compiled without transport feature ([#2403](https://github.com/rivet-gg/rivet/issues/2403)) ([a46b2f5](https://github.com/rivet-gg/rivet/commit/a46b2f586e1b7b55fa7da4d55fe3e2fb1a19388f))


### Chores

* **main:** release 25.4.0 ([#2402](https://github.com/rivet-gg/rivet/issues/2402)) ([844cd7d](https://github.com/rivet-gg/rivet/commit/844cd7d93a2fe9809a8d822717d21a6dda45fa1d))
* release 25.4.1 ([6289acb](https://github.com/rivet-gg/rivet/commit/6289acbea932704040a0e3433d936391e769ebbe))
* release 25.4.1 ([ff549fe](https://github.com/rivet-gg/rivet/commit/ff549fe7d58b95baed0e65e7da61b5bfa5e4c671))
* **release:** update version to 25.4.1 ([00f48ed](https://github.com/rivet-gg/rivet/commit/00f48edc8400d7c044dd126483f1526e3398ad50))
* **release:** update version to 25.4.1 ([2ca67f2](https://github.com/rivet-gg/rivet/commit/2ca67f2463b7fff393403d6a2187170b35c83c37))
* temporarily disable sentry ([#2406](https://github.com/rivet-gg/rivet/issues/2406)) ([0a94837](https://github.com/rivet-gg/rivet/commit/0a94837ffee0c901fd64cdafdebdacc039946b1d))
* update release please ([#2399](https://github.com/rivet-gg/rivet/issues/2399)) ([626d013](https://github.com/rivet-gg/rivet/commit/626d013b4754edf7cb80cb634b47bfce77d91e6b))

## [25.4.0](https://github.com/rivet-gg/rivet/compare/v25.3.1...v25.4.0) (2025-04-26)


### Features

* **hub/studio:** remove empty docs ([#2339](https://github.com/rivet-gg/rivet/issues/2339)) ([ed95552](https://github.com/rivet-gg/rivet/commit/ed95552b55dc7c8d49c93633a76e446e26a6effe))
* **hub:** add auth to inspector ([#2337](https://github.com/rivet-gg/rivet/issues/2337)) ([559218c](https://github.com/rivet-gg/rivet/commit/559218c2d9c6794015e379c8899a7ab3907d2a51))
* **hub:** logs ([#2364](https://github.com/rivet-gg/rivet/issues/2364)) ([cc22e91](https://github.com/rivet-gg/rivet/commit/cc22e919aec0cc859cdd32b34955bcccc5b7a31a))
* logs on clickhouse ([#2342](https://github.com/rivet-gg/rivet/issues/2342)) ([7cdd466](https://github.com/rivet-gg/rivet/commit/7cdd466db7d93b7b03ef5a6725f12d8c4dfa888d))
* routes ([#2365](https://github.com/rivet-gg/rivet/issues/2365)) ([ee96ff9](https://github.com/rivet-gg/rivet/commit/ee96ff952679a1ab9ee433b8b589ee1ad64b0c81))
* **site:** add giscus ([#2329](https://github.com/rivet-gg/rivet/issues/2329)) ([83f1f36](https://github.com/rivet-gg/rivet/commit/83f1f36fc375aff90adcf2bd782033a5114a8837))
* **site:** upgrade to App Router and Nextjs 15.2 ([#2328](https://github.com/rivet-gg/rivet/issues/2328)) ([1a66c42](https://github.com/rivet-gg/rivet/commit/1a66c429b9d188cb12b29a68761f9ba586f69a88))
* **studio:** add actor-core studio ([#2283](https://github.com/rivet-gg/rivet/issues/2283)) ([7fd26d9](https://github.com/rivet-gg/rivet/commit/7fd26d90cefd03f3e97ecaf2685bc6d60b90311b))
* **studio:** reconnect with inspect ([#2333](https://github.com/rivet-gg/rivet/issues/2333)) ([28fda52](https://github.com/rivet-gg/rivet/commit/28fda52d6904216bd91ccbf31230445f87cbca73))


### Bug Fixes

* add trace info to wfs and actor api ([#2341](https://github.com/rivet-gg/rivet/issues/2341)) ([7accb37](https://github.com/rivet-gg/rivet/commit/7accb37bc5e232e3bba0e8813f1e7e7f4f01d8e0))
* **cli:** remove openssl dependency again ([#2393](https://github.com/rivet-gg/rivet/issues/2393)) ([aed5eef](https://github.com/rivet-gg/rivet/commit/aed5eef6e9f193861974f5b32e39b95e58d0af11))
* **guard:** add dedicated websocket proxy handler ([#2321](https://github.com/rivet-gg/rivet/issues/2321)) ([bd718cf](https://github.com/rivet-gg/rivet/commit/bd718cf3a424227d5927bf4e3354cfb7633aee4f))
* **guard:** fix websocket proxying ([#2314](https://github.com/rivet-gg/rivet/issues/2314)) ([cbeeb86](https://github.com/rivet-gg/rivet/commit/cbeeb86a0c2f7868d0cfb95edfbf6017f8fc1dba))
* **guard:** purge cache on connect error ([#2344](https://github.com/rivet-gg/rivet/issues/2344)) ([9cbd422](https://github.com/rivet-gg/rivet/commit/9cbd42265695d2721fb496e8cf45dac8c340fda5))
* make copy buttons copy again ([#2379](https://github.com/rivet-gg/rivet/issues/2379)) ([d9e6701](https://github.com/rivet-gg/rivet/commit/d9e6701af5055f9a1888428e6ae405ed654826d9))
* **pegboard:** configure crypto for isolate-v8-runner ([#2310](https://github.com/rivet-gg/rivet/issues/2310)) ([c164e55](https://github.com/rivet-gg/rivet/commit/c164e558a44a3776123de69e73e32b2b882e1d90))
* **pegboard:** ignore all packets before init ([#2363](https://github.com/rivet-gg/rivet/issues/2363)) ([280bf12](https://github.com/rivet-gg/rivet/commit/280bf1280dce06516ec4a9f703b499329ea549a6))
* remove nonexistent actor sdk from release script ([598adc7](https://github.com/rivet-gg/rivet/commit/598adc76ba398cd9eddd7dfab9b8280834485d5b))
* return after actor create ([#2347](https://github.com/rivet-gg/rivet/issues/2347)) ([d6fbf36](https://github.com/rivet-gg/rivet/commit/d6fbf362709583b31fe2925b60c3bf7e311c34b6))
* **site:** deployment ([#2331](https://github.com/rivet-gg/rivet/issues/2331)) ([8e95ac6](https://github.com/rivet-gg/rivet/commit/8e95ac6f368aaa43e0beb01af37c5a3b4278dea4))
* **site:** display of code blocks ([#2323](https://github.com/rivet-gg/rivet/issues/2323)) ([0b9d562](https://github.com/rivet-gg/rivet/commit/0b9d56266040d654c3ff49775f68e2e7250a573f))
* **site:** invalid actors import leaking to site ([#2330](https://github.com/rivet-gg/rivet/issues/2330)) ([c57a557](https://github.com/rivet-gg/rivet/commit/c57a5579aeab7e2e51d11076e849abc6ecacb92c))
* sqlite driver tweaks ([#2370](https://github.com/rivet-gg/rivet/issues/2370)) ([701e77b](https://github.com/rivet-gg/rivet/commit/701e77bc9fbf626b6a7b4c0bf42b13298dee11e6))
* **studio:** change layout and make console autoscroll ([#2334](https://github.com/rivet-gg/rivet/issues/2334)) ([cb8bc18](https://github.com/rivet-gg/rivet/commit/cb8bc18778b76f66aa77d476a682f55d1c886f2e))
* **studio:** minor studio fixes ([#2306](https://github.com/rivet-gg/rivet/issues/2306)) ([a5f41c5](https://github.com/rivet-gg/rivet/commit/a5f41c543c66d1597e94eda75ec374155beae6dc))
* update runtime api to fern v0.49 ([#2391](https://github.com/rivet-gg/rivet/issues/2391)) ([26f9683](https://github.com/rivet-gg/rivet/commit/26f96831d0453a66d025fe59c4607dc4d763ac93))


### Documentation

* add links to actorcore on rivet ([#2375](https://github.com/rivet-gg/rivet/issues/2375)) ([d0b1aeb](https://github.com/rivet-gg/rivet/commit/d0b1aeb20ee6a67e5e893df734e2aa1944d0293d))
* add tip on ports & env vars for the dockerfile ([a067ca9](https://github.com/rivet-gg/rivet/commit/a067ca9337fdf09fefac4d77a4cd5bf93f5a4fdf))
* clean up separation between actors/containers/functions ([#2394](https://github.com/rivet-gg/rivet/issues/2394)) ([f35b5d5](https://github.com/rivet-gg/rivet/commit/f35b5d56582538e418ca323f7f2d8c1e72afb689))
* document ci/cd ([#2373](https://github.com/rivet-gg/rivet/issues/2373)) ([d68999b](https://github.com/rivet-gg/rivet/commit/d68999b5f5d82b8520a8adcf86e4b70de26d1b97))
* document game server use case ([#2298](https://github.com/rivet-gg/rivet/issues/2298)) ([11862b4](https://github.com/rivet-gg/rivet/commit/11862b4c3144b4c713549d51c623b35c3d1c681f))
* simplify sitemap ([#2374](https://github.com/rivet-gg/rivet/issues/2374)) ([6f03c44](https://github.com/rivet-gg/rivet/commit/6f03c445a2cb7620e9c633e8624d25f439be7e12))
* temporarily disable npm installation ([6ad6934](https://github.com/rivet-gg/rivet/commit/6ad6934e5c88fc232f0cb2944b389531d4e900f2))


### Continuous Integration

* configure git config globally ([#2390](https://github.com/rivet-gg/rivet/issues/2390)) ([433074c](https://github.com/rivet-gg/rivet/commit/433074c0392d51f2001fb32d19c9eca8c49f0f98))
* fix actor-core dependency in ci ([0c8a1ef](https://github.com/rivet-gg/rivet/commit/0c8a1ef5eafdedad7f3b68faca6ebb9fdb73e1f0))
* temporarily disable windows builds ([#2392](https://github.com/rivet-gg/rivet/issues/2392)) ([117b838](https://github.com/rivet-gg/rivet/commit/117b83825b05d73d36969fb69f17d2270add59a1))


### Chores

* add dev edge to justfile ([#2319](https://github.com/rivet-gg/rivet/issues/2319)) ([86bf47d](https://github.com/rivet-gg/rivet/commit/86bf47d3472b45c7ab34e50e45b35b3a3f349615))
* **api-actor:** handle no edge regions gracefully ([#2318](https://github.com/rivet-gg/rivet/issues/2318)) ([97a0110](https://github.com/rivet-gg/rivet/commit/97a01102e18ae5e32261066c0e87426d920523e7))
* **api-actor:** increase default rate limits ([#2311](https://github.com/rivet-gg/rivet/issues/2311)) ([8574269](https://github.com/rivet-gg/rivet/commit/857426926484f06ada210ff1a6c1e3c0ccf36ea6))
* disable building hub in docker container ([#2317](https://github.com/rivet-gg/rivet/issues/2317)) ([9a2ac65](https://github.com/rivet-gg/rivet/commit/9a2ac652c7e208672bae89e652938aafbfcffaeb))
* fix actor-core path ([198c963](https://github.com/rivet-gg/rivet/commit/198c963e770a25dd87656fea3ed78fdaeb98e966))
* fix broken links ([f7907d7](https://github.com/rivet-gg/rivet/commit/f7907d747a3236cb22bb9ee4f39f8d21334e2a68))
* fix broken links ([6f1ef12](https://github.com/rivet-gg/rivet/commit/6f1ef1228ea8caf96621b28e1c3ec9c4e84689c0))
* fix broken links ([8357a2c](https://github.com/rivet-gg/rivet/commit/8357a2c3633c5892d526025205d8ead32592e73f))
* fix broken links ([fe1cffd](https://github.com/rivet-gg/rivet/commit/fe1cffd97496b6e65a6d8d2eccb884df83c5adcd))
* fix building hub archive ([#2368](https://github.com/rivet-gg/rivet/issues/2368)) ([37f42ce](https://github.com/rivet-gg/rivet/commit/37f42ce7e32d50c0d62247325d72a9969ff6c551))
* fix docs links in actor list ([#2315](https://github.com/rivet-gg/rivet/issues/2315)) ([9a0eea2](https://github.com/rivet-gg/rivet/commit/9a0eea2b644d3030c06a984c3bef2bdc97d3499d))
* fix otel config for guard ([#2371](https://github.com/rivet-gg/rivet/issues/2371)) ([9f645c0](https://github.com/rivet-gg/rivet/commit/9f645c0c8ed133d616cfbf5b61c15f92abab0eb1))
* fix paths to sandboxed code execution examples ([#2381](https://github.com/rivet-gg/rivet/issues/2381)) ([564a402](https://github.com/rivet-gg/rivet/commit/564a40271d9ebc404b1157a291cb6ec3b1716f9e))
* fix rust example ([#2386](https://github.com/rivet-gg/rivet/issues/2386)) ([8d23b40](https://github.com/rivet-gg/rivet/commit/8d23b4048c2cd574ed17aaa79a54e358f4a0be1b))
* improve traces ([#2343](https://github.com/rivet-gg/rivet/issues/2343)) ([a7d1598](https://github.com/rivet-gg/rivet/commit/a7d15980a0337ddee98fb2bf67ffd1767ff71b10))
* **main:** release 25.4.0 ([#2302](https://github.com/rivet-gg/rivet/issues/2302)) ([320eb1b](https://github.com/rivet-gg/rivet/commit/320eb1bd4e159bd43662fa3f469af30dde657dd6))
* release 25.4.0 ([3d3c60e](https://github.com/rivet-gg/rivet/commit/3d3c60e5f178accf7995c2e79482d13c1e6dc084))
* release 25.4.0 ([1f1b9c5](https://github.com/rivet-gg/rivet/commit/1f1b9c52341f990a703b55ece9464b7ca92676ba))
* **release:** update version to 25.4.0 ([adb7f90](https://github.com/rivet-gg/rivet/commit/adb7f908f42d99ceb17ebb7441a21daf40e76422))
* **release:** update version to 25.4.0 ([90d409a](https://github.com/rivet-gg/rivet/commit/90d409ab25bd374bc01ef2c09de023175b194d65))
* switch logs query to use positionCaseInsensitive ([#2388](https://github.com/rivet-gg/rivet/issues/2388)) ([c30611d](https://github.com/rivet-gg/rivet/commit/c30611dbab009dd01602fdd4762df4470beba4c4))
* tweak clickhouse options for logs ([#2387](https://github.com/rivet-gg/rivet/issues/2387)) ([c057c6a](https://github.com/rivet-gg/rivet/commit/c057c6aead68c5fc4c8e0b77f925039a00d780bd))
* tweak wording for studio entrypoint ([#2308](https://github.com/rivet-gg/rivet/issues/2308)) ([2326b4c](https://github.com/rivet-gg/rivet/commit/2326b4ce863ce4eca99a5e292d557955859a1695))
* udpate guard websocket tests to include message sending ([#2320](https://github.com/rivet-gg/rivet/issues/2320)) ([e81d2ba](https://github.com/rivet-gg/rivet/commit/e81d2ba6e72ed51ca8f604045d5e5163f82ffe83))
* update lockfile ([d8a5d93](https://github.com/rivet-gg/rivet/commit/d8a5d9321eb1e19a845e43a3aae78a2273cd63ce))
* update release please ([#2399](https://github.com/rivet-gg/rivet/issues/2399)) ([626d013](https://github.com/rivet-gg/rivet/commit/626d013b4754edf7cb80cb634b47bfce77d91e6b))
* updated guide to print servers to json ([#2316](https://github.com/rivet-gg/rivet/issues/2316)) ([2e061d1](https://github.com/rivet-gg/rivet/commit/2e061d1601cf47bcd21c5635517856d7dfc51beb))

## [25.3.1](https://github.com/rivet-gg/rivet/compare/v25.3.0...v25.3.1) (2025-04-02)


### Features

* add configurable minimum age for lost servers ([#2297](https://github.com/rivet-gg/rivet/issues/2297)) ([ccc3e41](https://github.com/rivet-gg/rivet/commit/ccc3e417e961e3800404ba7f753fd841f8ba52f7))


### Bug Fixes

* **cluster:** fix nats provisioned in wrong subnet ([#2296](https://github.com/rivet-gg/rivet/issues/2296)) ([ec4edcd](https://github.com/rivet-gg/rivet/commit/ec4edcdf27688aeadaac5e39133d5c9056543c7c))
* **pegboard:** fix manager tests ([#2286](https://github.com/rivet-gg/rivet/issues/2286)) ([30e02e9](https://github.com/rivet-gg/rivet/commit/30e02e9528b0156fb65db66dd7075fa0c12b9494))


### Performance Improvements

* migrate to lz4 1.10.0 for parallelized decompression ([#2294](https://github.com/rivet-gg/rivet/issues/2294)) ([9eac08d](https://github.com/rivet-gg/rivet/commit/9eac08d1aec3111dc0c0139e21ceaa16b8ca716b))
* **pegboard:** download & extract with raw unix pipes ([#2295](https://github.com/rivet-gg/rivet/issues/2295)) ([2eac7fc](https://github.com/rivet-gg/rivet/commit/2eac7fcdcf2950d96497fe40cf53e6c2927ff817))
* **pegboard:** increase page size for passing data from downlaod to lz4 ([#2288](https://github.com/rivet-gg/rivet/issues/2288)) ([947b7f9](https://github.com/rivet-gg/rivet/commit/947b7f933b199e16be384928dc7882509677650a))
* **pegboard:** parallelize download image & setup cni network ([#2287](https://github.com/rivet-gg/rivet/issues/2287)) ([6f8e6a6](https://github.com/rivet-gg/rivet/commit/6f8e6a6beb58155aeaaac19a4e24d81d1c1c04a6))
* **pegboard:** parallelize writing configs under setup_oci_bundle ([#2289](https://github.com/rivet-gg/rivet/issues/2289)) ([c1011c5](https://github.com/rivet-gg/rivet/commit/c1011c5af6a8495fddcb635e4c93bb1a06e6773c))


### Chores

* add logging & duration metrics to pegboard setup ([#2291](https://github.com/rivet-gg/rivet/issues/2291)) ([3c04bec](https://github.com/rivet-gg/rivet/commit/3c04bec11670ab5d4b8d37123f1653f31af9310d))
* add script to distribute lz4 binary ([#2293](https://github.com/rivet-gg/rivet/issues/2293)) ([601075a](https://github.com/rivet-gg/rivet/commit/601075a421850176e49d72c81003ff19e3098b21))
* **api-actor:** don't attempt to contact edge dcs without worker nodes ([#2290](https://github.com/rivet-gg/rivet/issues/2290)) ([b124428](https://github.com/rivet-gg/rivet/commit/b124428ca787192518df42e1dc00fe0cce1fd7f0))
* build edge binaries in release mode ([#2292](https://github.com/rivet-gg/rivet/issues/2292)) ([65ee768](https://github.com/rivet-gg/rivet/commit/65ee7681c819a048c8263c3c785f267432ce3dc0))
* release 25.3.1 ([cdf53d3](https://github.com/rivet-gg/rivet/commit/cdf53d36f03d72c145373e1391645ca1ce98e8ab))

## [25.3.0](https://github.com/rivet-gg/rivet/compare/v25.2.2...v25.3.0) (2025-03-31)


### Features

* implement rivet guard ([#2276](https://github.com/rivet-gg/rivet/issues/2276)) ([9d61c63](https://github.com/rivet-gg/rivet/commit/9d61c63c42f861b15aaa2943b7c88c68abca3fc3))
* rg prod ([#2281](https://github.com/rivet-gg/rivet/issues/2281)) ([10aeaea](https://github.com/rivet-gg/rivet/commit/10aeaea348b34f0b2bf5815860da0decbd5f6e07))


### Chores

* add tls config to rivet guard ([#2279](https://github.com/rivet-gg/rivet/issues/2279)) ([bc0582f](https://github.com/rivet-gg/rivet/commit/bc0582f0da5241275bdfe5631c77cb0765977c5d))

## [25.2.2](https://github.com/rivet-gg/rivet/compare/v25.2.1...v25.2.2) (2025-03-28)


### Features

* add in memory cache driver ([#2215](https://github.com/rivet-gg/rivet/issues/2215)) ([e5bc6e6](https://github.com/rivet-gg/rivet/commit/e5bc6e686335821c19d28cde67e79ea1afc7092a))
* add network to actor metdata ([#2245](https://github.com/rivet-gg/rivet/issues/2245)) ([b23a0ae](https://github.com/rivet-gg/rivet/commit/b23a0ae60a32efbbf7bcb66eba0ce3b1a9ad00ec))
* **cli:** add `--extra-tags` and `--filter-tags` flags to `rivet deploy` ([#2224](https://github.com/rivet-gg/rivet/issues/2224)) ([12fbc1e](https://github.com/rivet-gg/rivet/commit/12fbc1e88959fffdc5227cb7b2ca9aec3caa78a6))
* compress sqlite db ([#2278](https://github.com/rivet-gg/rivet/issues/2278)) ([902bc29](https://github.com/rivet-gg/rivet/commit/902bc29417702397ffddd53eab49e578d0444b06))
* edge nats ([#2220](https://github.com/rivet-gg/rivet/issues/2220)) ([adf646c](https://github.com/rivet-gg/rivet/commit/adf646cf6879a26718bc0f233a0d0a0a7898a72c))
* graceful worker shutdown ([#2274](https://github.com/rivet-gg/rivet/issues/2274)) ([ea92b9d](https://github.com/rivet-gg/rivet/commit/ea92b9dafcb391c7186aafbf550d4f499b60e92e))
* **hub:** add a way to go direclty to an actor by id ([#2258](https://github.com/rivet-gg/rivet/issues/2258)) ([64e70ef](https://github.com/rivet-gg/rivet/commit/64e70ef5fefa375f79a2141c56523f135a5d9d99))
* **hub:** add confirmation screen to feedback modal ([#2259](https://github.com/rivet-gg/rivet/issues/2259)) ([2a90622](https://github.com/rivet-gg/rivet/commit/2a90622c1e965bef40dc125fa8466d845d5189ec))
* **hub:** bring hub to life ([#2234](https://github.com/rivet-gg/rivet/issues/2234)) ([90ff4a5](https://github.com/rivet-gg/rivet/commit/90ff4a54235e291c83cdfa296134c871fdfbaec2))
* **hub:** improve actor config readability ([#2261](https://github.com/rivet-gg/rivet/issues/2261)) ([582d933](https://github.com/rivet-gg/rivet/commit/582d9335dcab9b4d99dc94d31457018530002061))
* **hub:** link to project after linking project ([#2037](https://github.com/rivet-gg/rivet/issues/2037)) ([3f8de12](https://github.com/rivet-gg/rivet/commit/3f8de12e31370f5980573671b6f7783f529c12e6))
* **workflows:** sqlite WAL, deferrred flushing ([#2268](https://github.com/rivet-gg/rivet/issues/2268)) ([15cabdb](https://github.com/rivet-gg/rivet/commit/15cabdb3be35936c2cb4a4db42ae0164cfdcc39c))


### Bug Fixes

* add actor gen number, pb topo metric bug ([#2184](https://github.com/rivet-gg/rivet/issues/2184)) ([6410f61](https://github.com/rivet-gg/rivet/commit/6410f618258e394b46b882fd1baa536bf03fcf6b))
* add silencing to fdb driver ([#2177](https://github.com/rivet-gg/rivet/issues/2177)) ([5a21426](https://github.com/rivet-gg/rivet/commit/5a2142683939fd195a4c53420d904ee909cc76e7))
* **api-actor:** fix actor list showing wrong order ([#2228](https://github.com/rivet-gg/rivet/issues/2228)) ([3dec92c](https://github.com/rivet-gg/rivet/commit/3dec92c898932d5f65dd97d31a3e1f8f13720996))
* **hub:** cancelled error are now properly handled ([#2254](https://github.com/rivet-gg/rivet/issues/2254)) ([7aa18a2](https://github.com/rivet-gg/rivet/commit/7aa18a23567bd979067221da6b3ee7c89a9901d2))
* **hub:** display fold buttons when actor is not selected ([#2262](https://github.com/rivet-gg/rivet/issues/2262)) ([5de99e8](https://github.com/rivet-gg/rivet/commit/5de99e8410b38f78db7df6d65e9bd01f7cc939b4))
* **hub:** fix login loop ([#2239](https://github.com/rivet-gg/rivet/issues/2239)) ([fe6587f](https://github.com/rivet-gg/rivet/commit/fe6587ffb5e8a0c3f5bc005dd4c06ee2160ffbd5))
* **hub:** getting started introduction ([#2169](https://github.com/rivet-gg/rivet/issues/2169)) ([42a49c6](https://github.com/rivet-gg/rivet/commit/42a49c6560c3ccccdf3b34226c4a4150ee74b8cb))
* **hub:** group name validation ([#2256](https://github.com/rivet-gg/rivet/issues/2256)) ([69120dd](https://github.com/rivet-gg/rivet/commit/69120ddd9ffd970f858c7579709ef7d239b036ea))
* **hub:** login issues ([#2241](https://github.com/rivet-gg/rivet/issues/2241)) ([4dec3ef](https://github.com/rivet-gg/rivet/commit/4dec3efc3d1e535d0835dfb2a30bf6b4adbe3c6f))
* **hub:** re-render breadcrumbs on route change ([#2260](https://github.com/rivet-gg/rivet/issues/2260)) ([be207d7](https://github.com/rivet-gg/rivet/commit/be207d7242355e43ed1362d0ca731a63de341fab))
* **hub:** remove trash icon from actor logs ([#2257](https://github.com/rivet-gg/rivet/issues/2257)) ([30e2a48](https://github.com/rivet-gg/rivet/commit/30e2a48397ed379dc242dc1015e85ab5712538c2))
* reset actor gc before reschedule ([#2175](https://github.com/rivet-gg/rivet/issues/2175)) ([83d4598](https://github.com/rivet-gg/rivet/commit/83d4598a217871b59078f2c140b0843708b26803))
* **sdks/api:** fix ignores ignoring types dir ([84ae1c1](https://github.com/rivet-gg/rivet/commit/84ae1c1c4aef0f1df57ba0c29d63530523e300d8))


### Code Refactoring

* **hub:** hide actor-core actors inspector when displaying other actors ([#2236](https://github.com/rivet-gg/rivet/issues/2236)) ([6010c82](https://github.com/rivet-gg/rivet/commit/6010c826c1094802acef6265f60cdcaf8ea36cc0))
* **hub:** improve actors state sharing ([#2265](https://github.com/rivet-gg/rivet/issues/2265)) ([8b9c440](https://github.com/rivet-gg/rivet/commit/8b9c440e55ab2109dad571fb54ebba9165c5da07))
* **hub:** move off Tanstack Router Context in favour of React Context ([#2255](https://github.com/rivet-gg/rivet/issues/2255)) ([3aeb800](https://github.com/rivet-gg/rivet/commit/3aeb80004b75adb31392181e92f9cacf92761993))
* **hub:** Rivet.actor -&gt; Rivet.actors ([#2233](https://github.com/rivet-gg/rivet/issues/2233)) ([5badc3f](https://github.com/rivet-gg/rivet/commit/5badc3f4873318bc1fc88da73b6e1f6c96ff4f40))
* **hub:** update billing plans description ([#2263](https://github.com/rivet-gg/rivet/issues/2263)) ([2d3ec69](https://github.com/rivet-gg/rivet/commit/2d3ec6950b19f8dfe853c8fb613b989f8e031e3d))
* **hub:** use build tags to determine whenever an actor can have inspector ([#2264](https://github.com/rivet-gg/rivet/issues/2264)) ([28975f8](https://github.com/rivet-gg/rivet/commit/28975f8fbdcaa2705ce7dae6f1345fc5c10519b1))
* optimize dx experience for Max ([#2243](https://github.com/rivet-gg/rivet/issues/2243)) ([f0c46c7](https://github.com/rivet-gg/rivet/commit/f0c46c7841d2435143af5043bbf539b8725c4f4f))
* remove @rivet-gg/actors ([#2232](https://github.com/rivet-gg/rivet/issues/2232)) ([f9f47a6](https://github.com/rivet-gg/rivet/commit/f9f47a6ddf431c65e1e7ef61f3ae693c8c65a24e))
* **site:** improve readability ([#2271](https://github.com/rivet-gg/rivet/issues/2271)) ([1318c02](https://github.com/rivet-gg/rivet/commit/1318c0283067d0e9c0abbf2f28c7afc834f5c36c))


### Chores

* add fdb metrics ([#2176](https://github.com/rivet-gg/rivet/issues/2176)) ([5abeedb](https://github.com/rivet-gg/rivet/commit/5abeedb3620affe4414f7739ce1fb7d692026b32))
* add w3c spec blog ([#2251](https://github.com/rivet-gg/rivet/issues/2251)) ([652973b](https://github.com/rivet-gg/rivet/commit/652973bbaad2f337aa363ce71c792a8af9eef5d1))
* disable otel ([#2244](https://github.com/rivet-gg/rivet/issues/2244)) ([79cd271](https://github.com/rivet-gg/rivet/commit/79cd2718e7b34615c5abe61c4827a4427a4351b7))
* increase default rate limit for Rivet Guard ([#2226](https://github.com/rivet-gg/rivet/issues/2226)) ([4d711ca](https://github.com/rivet-gg/rivet/commit/4d711ca6011f15591e19832399a3d83312574c1b))
* og image ([#2222](https://github.com/rivet-gg/rivet/issues/2222)) ([0df741c](https://github.com/rivet-gg/rivet/commit/0df741c48dd6466124c19e0a1ec875296f5d07f9))
* release 25.2.2 ([d17ecf4](https://github.com/rivet-gg/rivet/commit/d17ecf488527e4a31c5446f11fcf00b2516dcb15))
* release 25.2.2 ([cf81b35](https://github.com/rivet-gg/rivet/commit/cf81b354c94120e3219b250a00f629e49bea7578))
* **release:** update version to 25.2.2 ([bd72622](https://github.com/rivet-gg/rivet/commit/bd726221bcef21b4d73b9e02571157bd75ceddd0))
* **release:** update version to 25.2.2 ([9faa4cc](https://github.com/rivet-gg/rivet/commit/9faa4cccc9a5833e2eedb14420f467d78e4672cc))
* remove `rivet init1 from readme ([93ab2f8](https://github.com/rivet-gg/rivet/commit/93ab2f8950aa0659f8089581ec7114f7e45c965f))
* update CTA ([#2247](https://github.com/rivet-gg/rivet/issues/2247)) ([460969f](https://github.com/rivet-gg/rivet/commit/460969fbc97771fade3b4abddccf3aa4fc6c333e))
* update discussion links ([ebf8d6d](https://github.com/rivet-gg/rivet/commit/ebf8d6d8dccbe21d521a66a0eae9923f6e6dda50))
* update reqwest client ([#2249](https://github.com/rivet-gg/rivet/issues/2249)) ([b85c995](https://github.com/rivet-gg/rivet/commit/b85c9954364965776a81c0472fbd31e1d287f19d))
* update w3c standard with recommendations ([#2269](https://github.com/rivet-gg/rivet/issues/2269)) ([9f92cb7](https://github.com/rivet-gg/rivet/commit/9f92cb7dc5de59a0b4c25e898f5bc06be56bff8d))

## [25.2.1](https://github.com/rivet-gg/rivet/compare/v25.2.0...v25.2.1) (2025-03-15)


### Features

* **cli:** add --version flag to `deploy`, `build publish`, and `actor create` ([#2202](https://github.com/rivet-gg/rivet/issues/2202)) ([a0f0b66](https://github.com/rivet-gg/rivet/commit/a0f0b66b03d57e2ce4dc6bcf69b2b4298e73d8f0))
* **cli:** add `RIVET_ENDPOINT` and `RIVET_CLOUD_TOKEN` env vars for auth ([#2203](https://github.com/rivet-gg/rivet/issues/2203)) ([6ae4dc1](https://github.com/rivet-gg/rivet/commit/6ae4dc1a8fa7fe2551f6504971b5b54cddaf3dc3))
* **cli:** publish to npm ([#2039](https://github.com/rivet-gg/rivet/issues/2039)) ([9ccee01](https://github.com/rivet-gg/rivet/commit/9ccee01607f2e46a453076c77df3e0e712638dab))
* **hub:** add actor-core inspector ([#2171](https://github.com/rivet-gg/rivet/issues/2171)) ([e6177fc](https://github.com/rivet-gg/rivet/commit/e6177fc1291cc28a9dc1b609c4b7551bc69e5b88))
* **hub:** add name column to actor builds ([#2172](https://github.com/rivet-gg/rivet/issues/2172)) ([e83ba5b](https://github.com/rivet-gg/rivet/commit/e83ba5bad78a022c750a9053064ce3fb47090df7))


### Bug Fixes

* add prepare script to @rivet-gg/cli so it builds before postinstall ([1020e3a](https://github.com/rivet-gg/rivet/commit/1020e3aff9cb6b752b7854289d263e6bb2e5ba8c))
* **cli:** adjust @rivet-gg/cli for npm ([#2174](https://github.com/rivet-gg/rivet/issues/2174)) ([3d49081](https://github.com/rivet-gg/rivet/commit/3d49081974004f9744df8f947ea62741a34436fd))
* container runner logs not working ([#2152](https://github.com/rivet-gg/rivet/issues/2152)) ([bf69be0](https://github.com/rivet-gg/rivet/commit/bf69be0a9b975ddbccff2a0bab398a65bc8d634b))
* core features alignment ([#2186](https://github.com/rivet-gg/rivet/issues/2186)) ([34b52da](https://github.com/rivet-gg/rivet/commit/34b52da94a85274a150b53a05128497e0d700f18))
* delete history for select wf, add actor start metric ([#2155](https://github.com/rivet-gg/rivet/issues/2155)) ([560bd20](https://github.com/rivet-gg/rivet/commit/560bd20a65ff64b84d2bebed1b03f542a45d7b22))
* fallback if @rivet-gg/cli doesn't work ([0897635](https://github.com/rivet-gg/rivet/commit/0897635965e5672c2084d94cebef0da720ec9873))
* fix building api packages ([7735744](https://github.com/rivet-gg/rivet/commit/77357446e715dea13e707f273b5972f65e9988f1))
* fix release update versions ([f10bfca](https://github.com/rivet-gg/rivet/commit/f10bfcaece4e6163581d28c8a5bf5437ca64a450))
* **hub:** add trailing slashes to urls when missing ([#2179](https://github.com/rivet-gg/rivet/issues/2179)) ([3cbbbfe](https://github.com/rivet-gg/rivet/commit/3cbbbfef02433b87dde0ef61ab04876d7d16cfbb))
* **hub:** display error message only if there's nothing to show ([#2166](https://github.com/rivet-gg/rivet/issues/2166)) ([745fb32](https://github.com/rivet-gg/rivet/commit/745fb32010c3320ce696df0e43b26a45616e00cc))
* **hub:** force context creation on lobbies page ([#2161](https://github.com/rivet-gg/rivet/issues/2161)) ([391708d](https://github.com/rivet-gg/rivet/commit/391708da0ffd16349f0ac3f3d1414757677e4d40))
* remove landing animation ([#2189](https://github.com/rivet-gg/rivet/issues/2189)) ([51946bb](https://github.com/rivet-gg/rivet/commit/51946bbcc419e7eda53f2465a4f14e4e2c1fb508))
* remove node_modules hack in favor of yarn pnp ([816046e](https://github.com/rivet-gg/rivet/commit/816046ebb1388cf86f3b7ea2ecb725bb32d7ac42))
* update cli image and wording ([#2193](https://github.com/rivet-gg/rivet/issues/2193)) ([a3ca882](https://github.com/rivet-gg/rivet/commit/a3ca8826dfcafdacf187fb0b0aeb0f39edbb3946))
* **workflows:** fix dupe wf run bug, signal idx bug, add wake condition property to get wf ([#2163](https://github.com/rivet-gg/rivet/issues/2163)) ([c42cc71](https://github.com/rivet-gg/rivet/commit/c42cc718750d3f97b1beadcd7b4b1b31870efdd2))


### Documentation

* **cli:** add docs for all remaining commands ([#2207](https://github.com/rivet-gg/rivet/issues/2207)) ([76b2fb5](https://github.com/rivet-gg/rivet/commit/76b2fb5ebe61ac92c9db1cf92fc592de92febbf6))
* document runtime ([#2211](https://github.com/rivet-gg/rivet/issues/2211)) ([6ae3443](https://github.com/rivet-gg/rivet/commit/6ae344361498392c2d416109199f422d8542e97d))
* revamp docs ([#2164](https://github.com/rivet-gg/rivet/issues/2164)) ([6cf3bcf](https://github.com/rivet-gg/rivet/commit/6cf3bcf0343dffa48d44249339b66188e54efb38))


### Chores

* add config validation command ([#2195](https://github.com/rivet-gg/rivet/issues/2195)) ([6bd1a60](https://github.com/rivet-gg/rivet/commit/6bd1a60cedbd629bd8c985be1a3417e51394c6c8))
* add pnp for esbuild ([00f87b9](https://github.com/rivet-gg/rivet/commit/00f87b985d8a6de0701bad42dac8c7ee9f9d208c))
* add sandboxed-code-execution demo for js & container ([#2165](https://github.com/rivet-gg/rivet/issues/2165)) ([9d90db3](https://github.com/rivet-gg/rivet/commit/9d90db3a7d71e96c5bcc2a7beac77a8f976c59c5))
* **api-status:** add container status check ([#2159](https://github.com/rivet-gg/rivet/issues/2159)) ([d1f57f4](https://github.com/rivet-gg/rivet/commit/d1f57f4a43f34e72065713c41f70a5f8b6aa8838))
* changelog cli ([#2191](https://github.com/rivet-gg/rivet/issues/2191)) ([ef51dc2](https://github.com/rivet-gg/rivet/commit/ef51dc218fda20f6c9ad8ff5b190dd607d7fb40e))
* **cli:** remove `BuildAccess` config ([#2208](https://github.com/rivet-gg/rivet/issues/2208)) ([81c62b5](https://github.com/rivet-gg/rivet/commit/81c62b5824929faee6f97513f1f3020aa3559cb3))
* disable durable lifecycle in system test ([#2157](https://github.com/rivet-gg/rivet/issues/2157)) ([aa1ecc9](https://github.com/rivet-gg/rivet/commit/aa1ecc9496b255a79a204354b890e5f806a76ee5))
* disable publishing cli since it's broken ([46bfeff](https://github.com/rivet-gg/rivet/commit/46bfeffbdec6a9aaa8dd797ad5abd9fb520fa702))
* **examples:** remove old configs ([#2206](https://github.com/rivet-gg/rivet/issues/2206)) ([f2a0bb4](https://github.com/rivet-gg/rivet/commit/f2a0bb48b237522e4284191ff025f9b5c85d4a4d))
* fix api docs ([#2209](https://github.com/rivet-gg/rivet/issues/2209)) ([7363327](https://github.com/rivet-gg/rivet/commit/73633276170a43ea5f67c21907ba416dff1feeac))
* flatten examples ([#2181](https://github.com/rivet-gg/rivet/issues/2181)) ([76a3602](https://github.com/rivet-gg/rivet/commit/76a3602fc118627c7717468da62ce8c6caff953a))
* install before publishing ([6762d8c](https://github.com/rivet-gg/rivet/commit/6762d8c315aee896bf44322e67c9ca6baf51c7c0))
* manifesto and changelog update ([#2200](https://github.com/rivet-gg/rivet/issues/2200)) ([688eed1](https://github.com/rivet-gg/rivet/commit/688eed1a21e073784452b44ea9a7f6ad758b1dcf))
* move apis to workspace ([3b304c3](https://github.com/rivet-gg/rivet/commit/3b304c34a1a8e36e1d21f0dea630d57f7499943d))
* release 25.2.1 ([7a41802](https://github.com/rivet-gg/rivet/commit/7a4180205598624c66b48089530763e12abeadf4))
* release 25.2.1 ([344ed3c](https://github.com/rivet-gg/rivet/commit/344ed3c20242e708700d7f097111fd665b31e26d))
* release 25.2.1 ([306f48d](https://github.com/rivet-gg/rivet/commit/306f48dc0abe2d4171e97b520ebfff91dc9aa7f5))
* release 25.2.1 ([3b3a112](https://github.com/rivet-gg/rivet/commit/3b3a112d235468136ed42243e29a6d04219b5a42))
* release 25.2.1 ([d233338](https://github.com/rivet-gg/rivet/commit/d233338596455542bc736a697de95953afe73739))
* release 25.2.1 ([d1e4eb4](https://github.com/rivet-gg/rivet/commit/d1e4eb4c87375878f1c568a42ad09bf05cb847f4))
* release 25.2.1 ([3768a7d](https://github.com/rivet-gg/rivet/commit/3768a7dad048ab9e0a0a8beb63efd192361478a5))
* release 25.2.1 ([ab5dc69](https://github.com/rivet-gg/rivet/commit/ab5dc69058601bdc6eb64c578f3be61dec328675))
* release 25.2.1 ([5efe76b](https://github.com/rivet-gg/rivet/commit/5efe76b8e67694fdaed97741812c7f37d886d90e))
* **release:** update version to 25.2.1 ([0989257](https://github.com/rivet-gg/rivet/commit/09892576c497fe265eec897790b79c465b258b9c))
* **release:** update version to 25.2.1 ([875c7ad](https://github.com/rivet-gg/rivet/commit/875c7ad800aeac352d3c90da9daf34aad1deda27))
* **release:** update version to 25.2.1 ([e598b6d](https://github.com/rivet-gg/rivet/commit/e598b6ddbf60c912ea1d94a04c2950e09dc46cbd))
* **release:** update version to 25.2.1 ([b901253](https://github.com/rivet-gg/rivet/commit/b901253b0f99f75545a2960f962bd4186f98f9d8))
* **release:** update version to 25.2.1 ([62afbfe](https://github.com/rivet-gg/rivet/commit/62afbfe0acd1c07001d749c0b4fb841dfbbe7e41))
* **release:** update version to 25.2.1 ([5dc75ce](https://github.com/rivet-gg/rivet/commit/5dc75ce6753cd21a0efcd01df1ebc176eeff9a74))
* **release:** update version to 25.2.1 ([4eb83de](https://github.com/rivet-gg/rivet/commit/4eb83de01ab13e9409975a4dddb45a14ea3c4803))
* **release:** update version to 25.2.1 ([24cdfc5](https://github.com/rivet-gg/rivet/commit/24cdfc5339dee2e4a81f407193c62d8876c998c9))
* **release:** update version to 25.2.1 ([916df87](https://github.com/rivet-gg/rivet/commit/916df8758958f179e4ff787549ee47eac4361330))
* remove @rivet-gg/cli from release temporarily ([08f5934](https://github.com/rivet-gg/rivet/commit/08f5934028e6c54c04b52b34258c8a3d5aa4016f))
* remove actors sdk in favor of actorcore ([#2199](https://github.com/rivet-gg/rivet/issues/2199)) ([ee7693b](https://github.com/rivet-gg/rivet/commit/ee7693ba7fb93b6a648f7e9e71586ad83eb6f786))
* remove init command ([#2198](https://github.com/rivet-gg/rivet/issues/2198)) ([b215811](https://github.com/rivet-gg/rivet/commit/b21581181f35f86f1ef8a1a0cd14c13f89038da6))
* rename actor -&gt; actors in sdk for consistency ([#2197](https://github.com/rivet-gg/rivet/issues/2197)) ([479ac9a](https://github.com/rivet-gg/rivet/commit/479ac9a5be2b4fb77ac7a3a889fd97185e4a1cb1))
* rename actor.regions.resolve to actor.regions.recommend ([#2196](https://github.com/rivet-gg/rivet/issues/2196)) ([b28003d](https://github.com/rivet-gg/rivet/commit/b28003dca4f7a7701414ee7920e07fa133512fd1))
* update package name to @rivet-gg/cli ([999431d](https://github.com/rivet-gg/rivet/commit/999431d3634cc1163674e17ca6f08c7d7527e071))
* updated oss friends ([#2212](https://github.com/rivet-gg/rivet/issues/2212)) ([c04db5c](https://github.com/rivet-gg/rivet/commit/c04db5c02be6d19787ee29fc57ea3068f6c779e1))
* updated site font ([#2204](https://github.com/rivet-gg/rivet/issues/2204)) ([37cce55](https://github.com/rivet-gg/rivet/commit/37cce55f9605638ca51e5060b2aa171b971db0b3))
* website redesign ([#2170](https://github.com/rivet-gg/rivet/issues/2170)) ([fd7fdf2](https://github.com/rivet-gg/rivet/commit/fd7fdf2d8946999c6b953820c868c348ddf1f9ee))
* workspace typo ([15db225](https://github.com/rivet-gg/rivet/commit/15db225f4f6a613626feb9c11132bef9a38dbea2))

## [25.2.0](https://github.com/rivet-gg/rivet/compare/v5.1.2...v25.2.0) (2025-03-07)


### Features

* add api-endpoint and access-token metadata commands ([#2090](https://github.com/rivet-gg/rivet/issues/2090)) ([af7cc8c](https://github.com/rivet-gg/rivet/commit/af7cc8cb0b5cd14da7699eefd6c7970595188da4))
* add auth status metadata command ([#2091](https://github.com/rivet-gg/rivet/issues/2091)) ([569adf9](https://github.com/rivet-gg/rivet/commit/569adf9e601aff6963045d6c919450b8f4166cb7))
* add env ls cmd ([#2109](https://github.com/rivet-gg/rivet/issues/2109)) ([c2ae14b](https://github.com/rivet-gg/rivet/commit/c2ae14b0236256aa345ec446a28ff1b57eab1cd3))
* add fdb cli ([#2051](https://github.com/rivet-gg/rivet/issues/2051)) ([5675b72](https://github.com/rivet-gg/rivet/commit/5675b722f5ad14dd9181b3bb8129bf81df44453b))
* add grafana to docker compose ([#2083](https://github.com/rivet-gg/rivet/issues/2083)) ([efb191d](https://github.com/rivet-gg/rivet/commit/efb191da1de1a5856f198e86e2efb8755967a301))
* add support for JSON5/JSONC config files ([#2059](https://github.com/rivet-gg/rivet/issues/2059)) ([850d25e](https://github.com/rivet-gg/rivet/commit/850d25e40eed032c462837bbccf8bdee6ec5262d))
* add X-API-Version header to CORS config ([#2143](https://github.com/rivet-gg/rivet/issues/2143)) ([84934c6](https://github.com/rivet-gg/rivet/commit/84934c6965d66ac29fa7b03fbc9560f9621bdeed))
* **cli:** add experimental `rivet shell` command ([#2113](https://github.com/rivet-gg/rivet/issues/2113)) ([d5fe5d8](https://github.com/rivet-gg/rivet/commit/d5fe5d8764acc0f577aa388e35c92a7578ed1773))
* **cli:** add project name id to metadata ([#2114](https://github.com/rivet-gg/rivet/issues/2114)) ([58e00e3](https://github.com/rivet-gg/rivet/commit/58e00e3e78b329077403fabf31898beb553fcb52))
* **clusters:** add worker pool type ([#2008](https://github.com/rivet-gg/rivet/issues/2008)) ([4b62182](https://github.com/rivet-gg/rivet/commit/4b621827f3b98185cf5e19aa6a694bd798bc64ce))
* fdb sqlite workflows driver ([#1850](https://github.com/rivet-gg/rivet/issues/1850)) ([fa40493](https://github.com/rivet-gg/rivet/commit/fa40493880c3a96547be00ce268a13bd9352fe2f))
* **pb, ds:** move to edge ([#1942](https://github.com/rivet-gg/rivet/issues/1942)) ([45d0d4c](https://github.com/rivet-gg/rivet/commit/45d0d4cb39707f850b28b2b3c2fc628e5a747ce4))
* **site:** add social links ([#2102](https://github.com/rivet-gg/rivet/issues/2102)) ([ba4fc46](https://github.com/rivet-gg/rivet/commit/ba4fc466d850aa68e24be728e0a30162836beeaa))
* **workflows:** abstract debug trait ([#2033](https://github.com/rivet-gg/rivet/issues/2033)) ([a8e9ea7](https://github.com/rivet-gg/rivet/commit/a8e9ea709a032e79f187a8805f878275cd698e19))
* **workflows:** impl db debug for fdb driver ([#2040](https://github.com/rivet-gg/rivet/issues/2040)) ([8366e9b](https://github.com/rivet-gg/rivet/commit/8366e9bcd7d8060a935b10a49590a8ab48007e90))
* **workflows:** implement metrics for fdb driver ([#2057](https://github.com/rivet-gg/rivet/issues/2057)) ([18448bc](https://github.com/rivet-gg/rivet/commit/18448bc4757bd8586b0fa8660f45b8a6fd4be244))


### Bug Fixes

* add exit to system test ([#2142](https://github.com/rivet-gg/rivet/issues/2142)) ([9d175a4](https://github.com/rivet-gg/rivet/commit/9d175a48863bb7b266f362ed5ca5d343b6d554b7))
* add metrics for wf engine ([#2076](https://github.com/rivet-gg/rivet/issues/2076)) ([1ea2b9c](https://github.com/rivet-gg/rivet/commit/1ea2b9c4a1e166847e20c6a4f86ccfa11e871ac1))
* add otel to install scripts ([#2092](https://github.com/rivet-gg/rivet/issues/2092)) ([2254745](https://github.com/rivet-gg/rivet/commit/2254745082046e9109d1174f9a18d7a0b898e7d5))
* **api-actor:** fix sorting of actors at the core ([#2139](https://github.com/rivet-gg/rivet/issues/2139)) ([834da81](https://github.com/rivet-gg/rivet/commit/834da819bb0f0430bee3875fd306f1ea66fdfa27))
* **api-actor:** make actor list cursor work correctly using new pagination api ([#2140](https://github.com/rivet-gg/rivet/issues/2140)) ([77bccb4](https://github.com/rivet-gg/rivet/commit/77bccb4499277ea629c7a2c3dbd3c2591fb925ea))
* **cluster:** write install script for worker pool ([#2009](https://github.com/rivet-gg/rivet/issues/2009)) ([cecd3fc](https://github.com/rivet-gg/rivet/commit/cecd3fcfef8d44c2cb0536d1551a29a409d85c67))
* **docker-compose:** remove invalid health check from otel-collector ([#2127](https://github.com/rivet-gg/rivet/issues/2127)) ([b645483](https://github.com/rivet-gg/rivet/commit/b6454830626da897db051368cb1190cacf878d0d))
* **docker:** auto-create otel database ([#2088](https://github.com/rivet-gg/rivet/issues/2088)) ([545fd10](https://github.com/rivet-gg/rivet/commit/545fd103a3cb510236eb89a438a83b8a46d789b9))
* **docker:** fix corepack not enabled error ([#2058](https://github.com/rivet-gg/rivet/issues/2058)) ([3dce130](https://github.com/rivet-gg/rivet/commit/3dce130b2ab5449bddd44323c5d48eb05146cf4a))
* **ds:** ds list ([#2066](https://github.com/rivet-gg/rivet/issues/2066)) ([3636d2e](https://github.com/rivet-gg/rivet/commit/3636d2e58dcf5526d64d68de7e0e930edb47661f))
* fdb tuples, actor lost kill ([#2118](https://github.com/rivet-gg/rivet/issues/2118)) ([af18f50](https://github.com/rivet-gg/rivet/commit/af18f50cc9718219c08e179125e7882d58ea1298))
* **fdb:** conflict ranges ([#2063](https://github.com/rivet-gg/rivet/issues/2063)) ([9ef4a32](https://github.com/rivet-gg/rivet/commit/9ef4a32b2f03310816a554d2b2e36788446f5328))
* fix connecting to clickhouse from edge servers ([#2128](https://github.com/rivet-gg/rivet/issues/2128)) ([955e41b](https://github.com/rivet-gg/rivet/commit/955e41bda71a062ef91bb4499a7a79d62c926dd6))
* fix pb draining ([#2136](https://github.com/rivet-gg/rivet/issues/2136)) ([209e3e9](https://github.com/rivet-gg/rivet/commit/209e3e9fe75b6c6e9be473f241b8cfd1eff5661e))
* fix system test ([#2121](https://github.com/rivet-gg/rivet/issues/2121)) ([6a5f7a4](https://github.com/rivet-gg/rivet/commit/6a5f7a4b93e0d41016541e2dd9848f6eae96625a))
* fix topology units, core actor endpoints ([#2135](https://github.com/rivet-gg/rivet/issues/2135)) ([81bdce8](https://github.com/rivet-gg/rivet/commit/81bdce80cf73ad0d5927e181f70f94cb1ba60152))
* fix wf change bug with client wf ([#2134](https://github.com/rivet-gg/rivet/issues/2134)) ([68388ad](https://github.com/rivet-gg/rivet/commit/68388ad02219fceabc20024d0393aec0cc240363))
* force amd64 platform for FDB tests ([#2049](https://github.com/rivet-gg/rivet/issues/2049)) ([bad88ad](https://github.com/rivet-gg/rivet/commit/bad88ad466ca766dbfb0e332c1ba85bc386136bd))
* get actors running e2e on edge ([#2027](https://github.com/rivet-gg/rivet/issues/2027)) ([bc2650d](https://github.com/rivet-gg/rivet/commit/bc2650d3a3c40bab03c7bad4e589a8e41310e01e))
* get edge api access over gg ([#2032](https://github.com/rivet-gg/rivet/issues/2032)) ([898d493](https://github.com/rivet-gg/rivet/commit/898d493d1b851db9f48b854ed07c8b282fe2a6d3))
* get sqlite working on edge ([#2097](https://github.com/rivet-gg/rivet/issues/2097)) ([26fdb96](https://github.com/rivet-gg/rivet/commit/26fdb962819b3e66cd9b9d90d635d06dc4b2018b))
* get sqlite working on edge ([#2099](https://github.com/rivet-gg/rivet/issues/2099)) ([0305b02](https://github.com/rivet-gg/rivet/commit/0305b02108da0837bc8ecedbe2d0f5a06e79459b))
* get stuff building ([#2020](https://github.com/rivet-gg/rivet/issues/2020)) ([6a4c3aa](https://github.com/rivet-gg/rivet/commit/6a4c3aa1b88ff47b629360ac5e3c4415593b21c4))
* get tunnels working, fix tls, wf bug fix ([#2025](https://github.com/rivet-gg/rivet/issues/2025)) ([a54e50b](https://github.com/rivet-gg/rivet/commit/a54e50b8f7254a809daffac4c80c658380479ca2))
* invalid component ([#2104](https://github.com/rivet-gg/rivet/issues/2104)) ([53cdd83](https://github.com/rivet-gg/rivet/commit/53cdd8324387114df4e2cbba871398fc23fea49a))
* **job-runner:** fix build pack in job-runner Dockerfile ([#2064](https://github.com/rivet-gg/rivet/issues/2064)) ([c427c40](https://github.com/rivet-gg/rivet/commit/c427c4088253daec3a1990ad1567d7f8e3fb2eeb))
* move actor state to fdb ([#2056](https://github.com/rivet-gg/rivet/issues/2056)) ([6900cf1](https://github.com/rivet-gg/rivet/commit/6900cf1f8385f42289c0df7ac011cc5d473efe59))
* navigator on ssr pages ([#2106](https://github.com/rivet-gg/rivet/issues/2106)) ([91d9405](https://github.com/rivet-gg/rivet/commit/91d9405fb6e9c131b2e2a373ca816578008602bd))
* **pb:** fix actor wf ([#2119](https://github.com/rivet-gg/rivet/issues/2119)) ([64085e7](https://github.com/rivet-gg/rivet/commit/64085e70faf2c2730eaefa5889f260f2502f0f66))
* **pb:** rescheduling ([#2146](https://github.com/rivet-gg/rivet/issues/2146)) ([001653d](https://github.com/rivet-gg/rivet/commit/001653da11a0b5566bb5c901eac292c80081e029))
* **runtime:** remove forcing tokio core count to 2 ([#2087](https://github.com/rivet-gg/rivet/issues/2087)) ([fd38a16](https://github.com/rivet-gg/rivet/commit/fd38a16d4c4f97ed32e7af17ca862efbf1fa83b3))
* ship actor logs to correct clickhouse database & table ([#2126](https://github.com/rivet-gg/rivet/issues/2126)) ([2f3b4be](https://github.com/rivet-gg/rivet/commit/2f3b4beb797acd4e0dac40ee9c7dba645b10451c))
* **system-test:** fix exiting after timeout in isolates ([#2138](https://github.com/rivet-gg/rivet/issues/2138)) ([b7ce0dd](https://github.com/rivet-gg/rivet/commit/b7ce0dd1fdb79a2de06aba46c5d8f2a57a474164))
* **system-test:** fix isolates use of hono ([#2124](https://github.com/rivet-gg/rivet/issues/2124)) ([0033f49](https://github.com/rivet-gg/rivet/commit/0033f49f143acd57562efe0ed3dfa636884b0072))
* update actor kv keys ([#2120](https://github.com/rivet-gg/rivet/issues/2120)) ([3824f89](https://github.com/rivet-gg/rivet/commit/3824f895d54f67b8b8c932d38a354e62561bfc7d))
* use direct clickhouse urls instead of tunneled urls ([#2150](https://github.com/rivet-gg/rivet/issues/2150)) ([02a3ef3](https://github.com/rivet-gg/rivet/commit/02a3ef303f0c82f0926e089b809b8129fd2999fe))
* various bug fixes ([#2069](https://github.com/rivet-gg/rivet/issues/2069)) ([c90e883](https://github.com/rivet-gg/rivet/commit/c90e883f4504cbe2b15e85642c59f091ae8faaac))
* **workflows:** allow op ctx to do all the things ([#1938](https://github.com/rivet-gg/rivet/issues/1938)) ([39365d4](https://github.com/rivet-gg/rivet/commit/39365d4a08c6c6e3dfa824f1862a508cbe3e714e))
* **workflows:** fix activity error backoff, in memory polling ([#2062](https://github.com/rivet-gg/rivet/issues/2062)) ([d29c11b](https://github.com/rivet-gg/rivet/commit/d29c11b199405d45a35d9e76cde7834f1bbfb181))
* **workflows:** remove tagged signals from fdb driver ([#2137](https://github.com/rivet-gg/rivet/issues/2137)) ([2119e4a](https://github.com/rivet-gg/rivet/commit/2119e4aa87d5b811ab0d1fe74e1f54d6a5f9a8c4))


### Documentation

* add helm rollback troubleshooting guide ([#2123](https://github.com/rivet-gg/rivet/issues/2123)) ([ca7898f](https://github.com/rivet-gg/rivet/commit/ca7898f37ced2aa90bc3d53c47ddc14fc1003532))


### Chores

* add basic load test ([#2060](https://github.com/rivet-gg/rivet/issues/2060)) ([85d01fa](https://github.com/rivet-gg/rivet/commit/85d01fa96aed74508c4481b5458b90e2ce25fc7f))
* add cockroachdb cluster identifier ([#2132](https://github.com/rivet-gg/rivet/issues/2132)) ([79ba98a](https://github.com/rivet-gg/rivet/commit/79ba98a58911feb2027718f8fc9ec2aacb690a21))
* add fdb-backed sqlite driver ([#2046](https://github.com/rivet-gg/rivet/issues/2046)) ([6d3a2ee](https://github.com/rivet-gg/rivet/commit/6d3a2ee409eba723b1486bcd02637975582e38e6))
* add load test for actor lifecycle containers ([#2110](https://github.com/rivet-gg/rivet/issues/2110)) ([e8ee5bd](https://github.com/rivet-gg/rivet/commit/e8ee5bde2bbeb1ddee5c1b67a67bb9b2cc6757bb))
* add native fdb library to path ([#2045](https://github.com/rivet-gg/rivet/issues/2045)) ([faa7e3d](https://github.com/rivet-gg/rivet/commit/faa7e3d3eaca89e18f3200b28de4147ea92552d4))
* add some logs ([#2133](https://github.com/rivet-gg/rivet/issues/2133)) ([c65c7ae](https://github.com/rivet-gg/rivet/commit/c65c7aeaa5503ab17dcf21600522ba6f0b2a8a69))
* add timeout flag to docker-compose down ([#2048](https://github.com/rivet-gg/rivet/issues/2048)) ([f9efe0e](https://github.com/rivet-gg/rivet/commit/f9efe0ebdc2024c72cb9d9fbe0e0b10a48f41121))
* add websocket example ([a5d021e](https://github.com/rivet-gg/rivet/commit/a5d021e9bc383c428f32b80864937253451a41c8))
* ai docs blog ([#2093](https://github.com/rivet-gg/rivet/issues/2093)) ([bf17bcd](https://github.com/rivet-gg/rivet/commit/bf17bcd449e41418bc1a93a1622522518c175c5b))
* auto-generate vendored hub when generating fern ([#2144](https://github.com/rivet-gg/rivet/issues/2144)) ([f5d99c2](https://github.com/rivet-gg/rivet/commit/f5d99c2dc0b53e4e136f7b26cdfb07a367b64ec8))
* better stack changelog ([#2080](https://github.com/rivet-gg/rivet/issues/2080)) ([b6bfa06](https://github.com/rivet-gg/rivet/commit/b6bfa06faa15c71280fec89ca27755d2b5406d6a))
* bump version from 5.1.2 to 25.1.3 ([#2122](https://github.com/rivet-gg/rivet/issues/2122)) ([e54c7ec](https://github.com/rivet-gg/rivet/commit/e54c7eca778f83bfb151be8246621504a5725f54))
* change grafana port from 3000 to 3100 ([#2131](https://github.com/rivet-gg/rivet/issues/2131)) ([6e337d9](https://github.com/rivet-gg/rivet/commit/6e337d94a575cefcefa525eed6fbb4d6f706b2a6))
* **cli:** disable shell cmd for adapter in clean envs ([#2112](https://github.com/rivet-gg/rivet/issues/2112)) ([781301d](https://github.com/rivet-gg/rivet/commit/781301d6af1295c8bd34b2fe3e794bb94d5c2323))
* combine ds into pb ([#2089](https://github.com/rivet-gg/rivet/issues/2089)) ([cceb5f9](https://github.com/rivet-gg/rivet/commit/cceb5f997015ded02e17ab32d15f3c12863cd392))
* edit ai docs blog ([#2095](https://github.com/rivet-gg/rivet/issues/2095)) ([5bfbe61](https://github.com/rivet-gg/rivet/commit/5bfbe61532b47b2aa8df7eec60b7492b4baebaf8))
* **examples:** add Better Stack monitoring example ([#2077](https://github.com/rivet-gg/rivet/issues/2077)) ([2d495e4](https://github.com/rivet-gg/rivet/commit/2d495e465970af1dad31329c305011a2a24268b4))
* **examples:** websocket example ([#2116](https://github.com/rivet-gg/rivet/issues/2116)) ([a06370e](https://github.com/rivet-gg/rivet/commit/a06370e3a13c54264ee8bea0b78da75f34655f22))
* **hub-embed:** disable all node-related code if hub building is disabled ([#2084](https://github.com/rivet-gg/rivet/issues/2084)) ([1230f3b](https://github.com/rivet-gg/rivet/commit/1230f3bcb4a15941ff0c3d8142f76e0e4d5f5d76))
* optimizations and such ([#2086](https://github.com/rivet-gg/rivet/issues/2086)) ([e085624](https://github.com/rivet-gg/rivet/commit/e0856241a69b744fa22e290e8220fa1d500a1cfc))
* **pools:** remove cert hack for clickhouse ([#2148](https://github.com/rivet-gg/rivet/issues/2148)) ([a2cbf1b](https://github.com/rivet-gg/rivet/commit/a2cbf1bf4f92686b3bd2a19628ee2a997e6b680d))
* **pools:** rewrite sqlite manager to run without locks using scc ([#2082](https://github.com/rivet-gg/rivet/issues/2082)) ([176ad21](https://github.com/rivet-gg/rivet/commit/176ad21cf22275e8322e6f1903d757a06f45f846))
* release 25.1.3 ([ca021eb](https://github.com/rivet-gg/rivet/commit/ca021eb2ad3d367d4f96860390d46d521465ee5a))
* release 25.2.0 ([4c5857a](https://github.com/rivet-gg/rivet/commit/4c5857a257872093b3ee7bfd6fdd10af1bee18e3))
* release 5.1.3 ([a37b6a4](https://github.com/rivet-gg/rivet/commit/a37b6a41f73dd2d12f5c3b51ea92e4eb24c6975d))
* **release:** update version to 25.1.3 ([7b49d63](https://github.com/rivet-gg/rivet/commit/7b49d63174623cde273266857764fae1912f1add))
* **release:** update version to 25.2.0 ([77e28ed](https://github.com/rivet-gg/rivet/commit/77e28ed198dd1ad2b72a1c504d27d5e65338a3de))
* **release:** update version to 5.1.3 ([6171e7f](https://github.com/rivet-gg/rivet/commit/6171e7f96d04cefaa5588af07c7b384013186c33))
* restructure server binaries ([#1941](https://github.com/rivet-gg/rivet/issues/1941)) ([7942152](https://github.com/rivet-gg/rivet/commit/79421526f4cb66eafeb63b1cd0dd6e5753a487c8))
* **runtime:** add otel support to runtime ([#2085](https://github.com/rivet-gg/rivet/issues/2085)) ([56a2a5e](https://github.com/rivet-gg/rivet/commit/56a2a5e7de7d3273cae3e7016c679ba22c73ac52))
* **site:** improve orama searchbox ([#2034](https://github.com/rivet-gg/rivet/issues/2034)) ([#2100](https://github.com/rivet-gg/rivet/issues/2100)) ([d99af82](https://github.com/rivet-gg/rivet/commit/d99af825eea57403e93b82cdfca4f7bffd73db75))
* **system-test:** add tick log every second ([#2129](https://github.com/rivet-gg/rivet/issues/2129)) ([3e07944](https://github.com/rivet-gg/rivet/commit/3e079444a43eb2d0d7b24d18c0132b5750828814))
* update traefik port to 9000 ([#2149](https://github.com/rivet-gg/rivet/issues/2149)) ([4c41e02](https://github.com/rivet-gg/rivet/commit/4c41e02c4c2a694ccbc4bee5c886a5028c136f5e))
* **workflows:** add gc and metrics indexes ([#1998](https://github.com/rivet-gg/rivet/issues/1998)) ([1dfdc88](https://github.com/rivet-gg/rivet/commit/1dfdc882b12b54ba33fe538b0e54ce6ed0acb6e5))
* **workflows:** move wf gc and metrics publish into worker ([#1943](https://github.com/rivet-gg/rivet/issues/1943)) ([fe6659b](https://github.com/rivet-gg/rivet/commit/fe6659b8066e739830395a3e735ee722b3121225))

## [5.1.2](https://github.com/rivet-gg/rivet/compare/v25.1.1...v5.1.2) (2025-02-24)


### Features

* **cli:** add `rivet push` command ([#2042](https://github.com/rivet-gg/rivet/issues/2042)) ([e9b6b3d](https://github.com/rivet-gg/rivet/commit/e9b6b3d2375cff723a402da770ee7e247c48b804))
* **hub:** add automatic region to actor creation form ([#2018](https://github.com/rivet-gg/rivet/issues/2018)) ([18c3eee](https://github.com/rivet-gg/rivet/commit/18c3eee1508d436917d78734d81eb0c4a9c72776))
* **hub:** add timestamp to builds list ([#2013](https://github.com/rivet-gg/rivet/issues/2013)) ([c0451c7](https://github.com/rivet-gg/rivet/commit/c0451c73d020f2a9935ad8e14687a85a750f432a))
* **hub:** allow filtering by own tags ([#2016](https://github.com/rivet-gg/rivet/issues/2016)) ([4be65c2](https://github.com/rivet-gg/rivet/commit/4be65c24e0016c3c719385da6e3ad9357809381b))
* **hub:** use same setup guide as main site ([#2029](https://github.com/rivet-gg/rivet/issues/2029)) ([4608305](https://github.com/rivet-gg/rivet/commit/4608305335502a701c7aa13260d59618df6551f6))
* **hub:** when actor is starting and no logs are found display message ([#2017](https://github.com/rivet-gg/rivet/issues/2017)) ([6d96d60](https://github.com/rivet-gg/rivet/commit/6d96d609e3fbc2d7f0b935b595f320ec9f3bd313))


### Bug Fixes

* **hub:** invalid token when parsing recent team id ([#2011](https://github.com/rivet-gg/rivet/issues/2011)) ([9120786](https://github.com/rivet-gg/rivet/commit/91207861693134dd8cde00d4a79c3af147390b24))
* **hub:** logs are not streamed when opening newly created actor ([#2015](https://github.com/rivet-gg/rivet/issues/2015)) ([2352677](https://github.com/rivet-gg/rivet/commit/23526779129f4dee92052ff116ac1aac5037c4e8))
* **hub:** prevent long lines to break the layout ([#2014](https://github.com/rivet-gg/rivet/issues/2014)) ([a16e496](https://github.com/rivet-gg/rivet/commit/a16e4968ad3abb02439866c0ecc1af4a939baf5d))
* **hub:** remove outline on avatars ([#2010](https://github.com/rivet-gg/rivet/issues/2010)) ([64e322a](https://github.com/rivet-gg/rivet/commit/64e322a0cce7998a64699cda7d5210cec2de2396))
* **js-utils:** correctly handle esbuild with import-statement not matching node stdlib ([#2067](https://github.com/rivet-gg/rivet/issues/2067)) ([9b39c35](https://github.com/rivet-gg/rivet/commit/9b39c35a8a9830bf1a6655c967d3b6c352d548c0))


### Code Refactoring

* **hub:** improve bundle size ([#2030](https://github.com/rivet-gg/rivet/issues/2030)) ([398d9f8](https://github.com/rivet-gg/rivet/commit/398d9f858b59012d527a67dad656ff1a98597c52))
* **hub:** remove "actions" label in context menus ([#2019](https://github.com/rivet-gg/rivet/issues/2019)) ([2ed27d9](https://github.com/rivet-gg/rivet/commit/2ed27d931f5fa7eefafc4b57f4b9c915109df677))
* **hub:** remove public & dev tokens unless legacy mm ([#2012](https://github.com/rivet-gg/rivet/issues/2012)) ([e0f6ac5](https://github.com/rivet-gg/rivet/commit/e0f6ac5f95fbb421825c82f83c8595fb8e0d2a06))


### Chores

* add inspector and gif for changelog ([#2024](https://github.com/rivet-gg/rivet/issues/2024)) ([4e38407](https://github.com/rivet-gg/rivet/commit/4e38407e45c44cee66ba8d683ca5714c971587cc))
* add s3 example ([#2068](https://github.com/rivet-gg/rivet/issues/2068)) ([fb604f6](https://github.com/rivet-gg/rivet/commit/fb604f628ac510df77f811c69d5e5a750c2eed1f))
* added getting started prereq ([#2021](https://github.com/rivet-gg/rivet/issues/2021)) ([d13839e](https://github.com/rivet-gg/rivet/commit/d13839e90c55a74cb7d2d4f02803d8408bc02369))
* **blog:** post sqlite article ([ab75fb4](https://github.com/rivet-gg/rivet/commit/ab75fb456461563acd4f199442f755ec98c9a55a))
* Credits update ([#2053](https://github.com/rivet-gg/rivet/issues/2053)) ([6ccf25c](https://github.com/rivet-gg/rivet/commit/6ccf25c13013c8f70fdbf1bdf2751066bc5858ce))
* **js-utils:** switch from unenv to node-stdlib-browser ([#2071](https://github.com/rivet-gg/rivet/issues/2071)) ([479bc55](https://github.com/rivet-gg/rivet/commit/479bc557294f1b6bc446a7dceab94ce0692e43ec))
* **js-utils:** upgrade to unenv 2 ([#2070](https://github.com/rivet-gg/rivet/issues/2070)) ([83a6ca5](https://github.com/rivet-gg/rivet/commit/83a6ca5c91659518bc4b4c82d4c508cadb59ad10))
* release 5.1.2 ([5237e0e](https://github.com/rivet-gg/rivet/commit/5237e0efcd0349fc625a1f860470427754bcc282))
* release 5.1.2 ([c54ca06](https://github.com/rivet-gg/rivet/commit/c54ca065985a67ebcd23e0d138c7862457e7fb02))
* **release:** generate yarn.lock on release ([c432d47](https://github.com/rivet-gg/rivet/commit/c432d4711944d570ff4baf0b26e14cf458e87f2f))
* **release:** update version to 5.1.2 ([9d0bbe2](https://github.com/rivet-gg/rivet/commit/9d0bbe2e6c6c6171d3f0ed1452dd401fc4976aee))
* **release:** update version to 5.1.2 ([7f92886](https://github.com/rivet-gg/rivet/commit/7f92886ce6256500a0f2dc16bc00a9e9c111952b))
* remove lefthook git hooks ([#2073](https://github.com/rivet-gg/rivet/issues/2073)) ([86010d0](https://github.com/rivet-gg/rivet/commit/86010d09781f4283e5123e9d89db9ded6904ea66))
* update discussions url ([b39c154](https://github.com/rivet-gg/rivet/commit/b39c154f3164ec5b00cc1d18bd0d3c4ccbc443b5))
* update pricing blog ([#2050](https://github.com/rivet-gg/rivet/issues/2050)) ([295ba50](https://github.com/rivet-gg/rivet/commit/295ba504bfa645fec897b7906996adfc22a95cc0))
* Updated Pricing ([#2044](https://github.com/rivet-gg/rivet/issues/2044)) ([50226fd](https://github.com/rivet-gg/rivet/commit/50226fd787b7676887aabe683ff9b20ba7f125fc))

## [25.1.1](https://github.com/rivet-gg/rivet/compare/v25.1.0...v25.1.1) (2025-02-08)


### Bug Fixes

* **cli:** auto-generate .yarnrc.yml to disable pnp for tsx support ([#2003](https://github.com/rivet-gg/rivet/issues/2003)) ([f7d5c49](https://github.com/rivet-gg/rivet/commit/f7d5c49fc39ad643d026f83bd7804f181a4992ff))
* **cli:** set abs path for esbuild to support yarn pnp ([#2001](https://github.com/rivet-gg/rivet/issues/2001)) ([995d761](https://github.com/rivet-gg/rivet/commit/995d76113dc37024e9bd16f32a11d3c118ba850d))


### Documentation

* simplify setup guide and modernize SDK instructions ([#2004](https://github.com/rivet-gg/rivet/issues/2004)) ([a65ccef](https://github.com/rivet-gg/rivet/commit/a65ccef3f489035c6c1337ae9c4109299a691859))


### Chores

* migrate from jsr.io to npm ([#2005](https://github.com/rivet-gg/rivet/issues/2005)) ([0a6d677](https://github.com/rivet-gg/rivet/commit/0a6d67741e073bf64930b48de4381099a4c48abc))
* release 25.1.1 ([8a9c8e3](https://github.com/rivet-gg/rivet/commit/8a9c8e331953b0371a52773a2e8497ad5cb370f5))
* **release:** update version to 25.1.1 ([6561ef5](https://github.com/rivet-gg/rivet/commit/6561ef53729e551d383cc66c7c5748cab964c0b0))

## [25.1.0](https://github.com/rivet-gg/rivet/compare/v24.6.2...v25.1.0) (2025-02-04)


### Features

* add ats prewarm to pegboard ([#1816](https://github.com/rivet-gg/rivet/issues/1816)) ([691421e](https://github.com/rivet-gg/rivet/commit/691421e59cc9522b06b9b52407691acd8f679146))
* add build script for direct esbuild testing ([#1880](https://github.com/rivet-gg/rivet/issues/1880)) ([a8d1c76](https://github.com/rivet-gg/rivet/commit/a8d1c762d145395054f6fab39a711a69dd4601bc))
* add error screen for network issues ([#1871](https://github.com/rivet-gg/rivet/issues/1871)) ([3c331a7](https://github.com/rivet-gg/rivet/commit/3c331a7e35505757396f87da2863707b13e0cb9c))
* add project and environment view commands ([#1888](https://github.com/rivet-gg/rivet/issues/1888)) ([df70e3f](https://github.com/rivet-gg/rivet/commit/df70e3f1e9ae3155f1c82b0b22d06fc3a23a7def))
* add tunnel cert rotation ([#1804](https://github.com/rivet-gg/rivet/issues/1804)) ([66f5c63](https://github.com/rivet-gg/rivet/commit/66f5c632be1f30dd9362267196c1cb9395367eb1))
* **cli:** add `rivet view` command as alias of `rivet environment view` ([#1911](https://github.com/rivet-gg/rivet/issues/1911)) ([d4c4824](https://github.com/rivet-gg/rivet/commit/d4c4824138fe2c918dc500daae5c17447f8c0aea))
* **cli:** automatically prompt to login for commands that require auth ([#1913](https://github.com/rivet-gg/rivet/issues/1913)) ([9c7c793](https://github.com/rivet-gg/rivet/commit/9c7c793d02656a6387479482b392cbfc21aa5873))
* github star button ([#1851](https://github.com/rivet-gg/rivet/issues/1851)) ([65a9b2e](https://github.com/rivet-gg/rivet/commit/65a9b2e3e5775a1e85f15c4dd91a6345d95555cd))
* **hub:** actor repl ([#1841](https://github.com/rivet-gg/rivet/issues/1841)) ([80b7b39](https://github.com/rivet-gg/rivet/commit/80b7b39b765cfd6d196762ddc12e5469d93ea62d))
* **hub:** actors and builds filters ([#1884](https://github.com/rivet-gg/rivet/issues/1884)) ([bad6906](https://github.com/rivet-gg/rivet/commit/bad6906b9bda56eef2509229f18af61a56e5dcbc))
* **hub:** add dedicated name column for actors & builds ([#1995](https://github.com/rivet-gg/rivet/issues/1995)) ([089abbf](https://github.com/rivet-gg/rivet/commit/089abbf22239286476866603834320f0a0477a4f))
* **hub:** add more options to create actor form ([#1944](https://github.com/rivet-gg/rivet/issues/1944)) ([6b6af49](https://github.com/rivet-gg/rivet/commit/6b6af49c7ba273538b441bbadbde812b30b472f6))
* **hub:** add separate protocol for actor inspect ([#1946](https://github.com/rivet-gg/rivet/issues/1946)) ([603c305](https://github.com/rivet-gg/rivet/commit/603c3056f987aa12749c3d94c831643de3d14701))
* **hub:** display onboarding only when user has no builds ([#1947](https://github.com/rivet-gg/rivet/issues/1947)) ([e32080d](https://github.com/rivet-gg/rivet/commit/e32080d2abb538659a1f791983b5d96fd63fff90))
* **sdks/actor/runtime:** include url for debugging actor in internal error metadata ([#1952](https://github.com/rivet-gg/rivet/issues/1952)) ([0962fb4](https://github.com/rivet-gg/rivet/commit/0962fb4fb6cc5cd7e4f3b4b955b659eaeb1a1334))
* **sdks/actor:** add http api for calling rpcs ([#1950](https://github.com/rivet-gg/rivet/issues/1950)) ([d1c8e27](https://github.com/rivet-gg/rivet/commit/d1c8e27f67358702b814d6234afba3fbe1038961))
* **site:** add "edit this page" button ([#1885](https://github.com/rivet-gg/rivet/issues/1885)) ([ae145ce](https://github.com/rivet-gg/rivet/commit/ae145ce8eb12104742517098512a5f0044811587))
* **site:** add orama search ([#1948](https://github.com/rivet-gg/rivet/issues/1948)) ([ac5de3e](https://github.com/rivet-gg/rivet/commit/ac5de3ec07faf771bb2d2ac0babcaf3c0c755e97))
* use new onboarding flow when linking a device ([#1869](https://github.com/rivet-gg/rivet/issues/1869)) ([5e63965](https://github.com/rivet-gg/rivet/commit/5e63965e82e86790372923f8db3a95b726680ee6))
* **workflows:** add loop state ([#1939](https://github.com/rivet-gg/rivet/issues/1939)) ([ce8db74](https://github.com/rivet-gg/rivet/commit/ce8db746138c9da99989b8879899b6d4751e522d))


### Bug Fixes

* **actors-sdk:** use proper binary fromat when receiving cbor data ([#1743](https://github.com/rivet-gg/rivet/issues/1743)) ([acd927e](https://github.com/rivet-gg/rivet/commit/acd927e4c913d2a8290eea54dcd6b1d2d2dc8095))
* add missing space in README tagline ([#1968](https://github.com/rivet-gg/rivet/issues/1968)) ([3a9c065](https://github.com/rivet-gg/rivet/commit/3a9c065faf8a36914d66dfecc70668431d570848))
* add website image to header ([#1900](https://github.com/rivet-gg/rivet/issues/1900)) ([cb2ce1e](https://github.com/rivet-gg/rivet/commit/cb2ce1e541736e50b7fc5122ccfe4388c24cca21))
* adjust release scripts to new workflow ([#1863](https://github.com/rivet-gg/rivet/issues/1863)) ([cf7f7b6](https://github.com/rivet-gg/rivet/commit/cf7f7b6f3160e4983d6565b8c3d3db260f06c65c))
* **build:** fix fetching build by id with null tags ([#1991](https://github.com/rivet-gg/rivet/issues/1991)) ([4e2619c](https://github.com/rivet-gg/rivet/commit/4e2619c76ea9c3ea3b429f551bcee5d11d6d6218))
* **ci:** fix building toolchain ([#1996](https://github.com/rivet-gg/rivet/issues/1996)) ([94a3bf2](https://github.com/rivet-gg/rivet/commit/94a3bf2f237b2ab51227e046a7f29837ca72be95))
* **cli:** make esbuild portable ([#1917](https://github.com/rivet-gg/rivet/issues/1917)) ([e674c16](https://github.com/rivet-gg/rivet/commit/e674c162577550a295e02920d4fce3bc83abda75))
* **cluster:** create new cluster server wf with stateful loop ([#1940](https://github.com/rivet-gg/rivet/issues/1940)) ([82c0290](https://github.com/rivet-gg/rivet/commit/82c0290639b6f30cf27f56fe902c9cc752ea0ee8))
* correct package name in icons rebuild command ([#1963](https://github.com/rivet-gg/rivet/issues/1963)) ([f8336c0](https://github.com/rivet-gg/rivet/commit/f8336c0b62981acc986bc5bbe2d21f7994638d27))
* **docker:** auto-build sdk & hub in build.rs ([#1972](https://github.com/rivet-gg/rivet/issues/1972)) ([36f37cc](https://github.com/rivet-gg/rivet/commit/36f37ccbc47bda916ccd47a8150d3690f892ff3b))
* **docker:** update base-runner deiban to 12.1 to fix ca-certificates install error ([#1903](https://github.com/rivet-gg/rivet/issues/1903)) ([c5adb1e](https://github.com/rivet-gg/rivet/commit/c5adb1ef0a9b83dddf5e60ef952df6e328d86f35))
* docs button ([#1858](https://github.com/rivet-gg/rivet/issues/1858)) ([2b20094](https://github.com/rivet-gg/rivet/commit/2b2009404b1acc9b1cdb2e8562e7fa3340d48169))
* **frontend/packages/icons:** fix failling to install @rivet-gg/icons when fontawesome token is not provided ([#1982](https://github.com/rivet-gg/rivet/issues/1982)) ([d264645](https://github.com/rivet-gg/rivet/commit/d26464508b3f3e2d79d72ab18fc0fac2603af1f5))
* **hub:** allow clicking continue button with only one project in local dev ([#1973](https://github.com/rivet-gg/rivet/issues/1973)) ([7f3697a](https://github.com/rivet-gg/rivet/commit/7f3697ae71d7a8b24a0c78050e44f1cbfd40b911))
* **hub:** build with rivet gives 404s ([#1855](https://github.com/rivet-gg/rivet/issues/1855)) ([62fde8e](https://github.com/rivet-gg/rivet/commit/62fde8e6cc54393a81dde8186272e9be84d49892))
* **hub:** fix actors overflowing with long state ([#1979](https://github.com/rivet-gg/rivet/issues/1979)) ([5667155](https://github.com/rivet-gg/rivet/commit/5667155de8555779bd52425c370421200d45526e))
* **hub:** use https to create manager url ([#1907](https://github.com/rivet-gg/rivet/issues/1907)) ([5ea3608](https://github.com/rivet-gg/rivet/commit/5ea360868d4d8680b157ce105542fb6cc4e6185f))
* improve default export error message ([#1895](https://github.com/rivet-gg/rivet/issues/1895)) ([8dc418b](https://github.com/rivet-gg/rivet/commit/8dc418b38b5722a0e140e4ccdd4da3cd2b4642b5))
* links for pricing and sales ([#1845](https://github.com/rivet-gg/rivet/issues/1845)) ([010d367](https://github.com/rivet-gg/rivet/commit/010d367a9b3871c5dedd558012fb8f0e2739c835))
* **pegboard:** fix query in get_client_from_dc ([#1897](https://github.com/rivet-gg/rivet/issues/1897)) ([ba82622](https://github.com/rivet-gg/rivet/commit/ba8262219d4d7b0096c08308d0fbee5b36390052))
* periodically pull ats addr ([#1814](https://github.com/rivet-gg/rivet/issues/1814)) ([dd65e86](https://github.com/rivet-gg/rivet/commit/dd65e86b0e6dab62e49fb838cfddc9387ab460cc))
* promo image ([#1847](https://github.com/rivet-gg/rivet/issues/1847)) ([55e7b20](https://github.com/rivet-gg/rivet/commit/55e7b20d3b2efdac03578d345f852a514b428dc6))
* release scripts ([#1932](https://github.com/rivet-gg/rivet/issues/1932)) ([057e98b](https://github.com/rivet-gg/rivet/commit/057e98b64864b9f9ceb9a77111552865b247fbb8))
* release scripts ([#1934](https://github.com/rivet-gg/rivet/issues/1934)) ([4733bf0](https://github.com/rivet-gg/rivet/commit/4733bf09769a3387c16f9b37300c4802ba7d1d13))
* remove react scan ([#1862](https://github.com/rivet-gg/rivet/issues/1862)) ([2871f2b](https://github.com/rivet-gg/rivet/commit/2871f2b244cb0582c4836172fdfc0d27c8ad1376))
* remove source hash from cache ([#1843](https://github.com/rivet-gg/rivet/issues/1843)) ([86e5238](https://github.com/rivet-gg/rivet/commit/86e5238f1366ca14dddfbbe7682767a3d4d803d9))
* rename resolvePromise to promise in ActorHandleRaw ([#1957](https://github.com/rivet-gg/rivet/issues/1957)) ([a54ddc9](https://github.com/rivet-gg/rivet/commit/a54ddc94fde63203a6e7054683423353a884ece9))
* reset recent team redirection when leaving a group ([#1873](https://github.com/rivet-gg/rivet/issues/1873)) ([1a80f92](https://github.com/rivet-gg/rivet/commit/1a80f92cfa7d9be2f07d226dbaa2bcebda63cd72))
* revert upsert query in pegboard ws ([#1831](https://github.com/rivet-gg/rivet/issues/1831)) ([e4256d9](https://github.com/rivet-gg/rivet/commit/e4256d91fab8fbfbb5f62a4087854575b00f7e52))
* **sdks/actor/client:** enable @rivet-gg/actor-client/test to run in node env ([#1912](https://github.com/rivet-gg/rivet/issues/1912)) ([d1176c9](https://github.com/rivet-gg/rivet/commit/d1176c941e5e366a8ef24cb13b12090aa4adb4c6))
* **sdks/actor/manager:** fix cors to allow more origins ([#1910](https://github.com/rivet-gg/rivet/issues/1910)) ([9a7d71b](https://github.com/rivet-gg/rivet/commit/9a7d71b623967626b6299c6078df251a55978530))
* **sdks/actor/manager:** fix manager builds with new workspaces ([#1890](https://github.com/rivet-gg/rivet/issues/1890)) ([0d41cc8](https://github.com/rivet-gg/rivet/commit/0d41cc8a57a3d1e4df857dbf4dfa30dbce46716d))
* **sdks/actor/runtime:** fix internal errors not getting logged & make UserError public ([#1958](https://github.com/rivet-gg/rivet/issues/1958)) ([c02af68](https://github.com/rivet-gg/rivet/commit/c02af68d4545171362ee8a0d21768d06256cdc33))
* **sdks/actor/runtime:** work around typescript bug inferring ConnParams with ExtractActorConnParams ([#1951](https://github.com/rivet-gg/rivet/issues/1951)) ([070af03](https://github.com/rivet-gg/rivet/commit/070af03c5e2a3579d4ae3f29e52a0ff29f7cff47))
* select styles and build base url ([#1867](https://github.com/rivet-gg/rivet/issues/1867)) ([2dbead7](https://github.com/rivet-gg/rivet/commit/2dbead7603bf5c17e4dcd693565021b09089e50a))
* **site:** auto-generate unframer components on dev ([#1970](https://github.com/rivet-gg/rivet/issues/1970)) ([572d1ae](https://github.com/rivet-gg/rivet/commit/572d1aec981cf115634b14585e25226f8abe69ef))
* **site:** remove use of `assert { type: ... }` for better nodejs compat ([#1962](https://github.com/rivet-gg/rivet/issues/1962)) ([b44278c](https://github.com/rivet-gg/rivet/commit/b44278c3445ef4abcfa38254a034e44f5889de24))
* **toolchain:** correctly handle current tag with multiple builds with the same name ([#1992](https://github.com/rivet-gg/rivet/issues/1992)) ([b1cfd2d](https://github.com/rivet-gg/rivet/commit/b1cfd2d1a83f16166e743375a447335b49078fcf))
* **toolchain:** fix vergen_git2 -&gt; vergen dependency ([#1983](https://github.com/rivet-gg/rivet/issues/1983)) ([ec17c12](https://github.com/rivet-gg/rivet/commit/ec17c1216e78b33dd9118bef6e76081e21118d7b))


### Documentation

* add cd site to dev instructions ([#1971](https://github.com/rivet-gg/rivet/issues/1971)) ([b464d59](https://github.com/rivet-gg/rivet/commit/b464d59845b3586995d82e5af4223867b1d029f8))
* add Deno and Node.js compatibility tips ([#1878](https://github.com/rivet-gg/rivet/issues/1878)) ([868a233](https://github.com/rivet-gg/rivet/commit/868a23353161e77f8e66f8e40862512ac8b3c704))
* add FoundationDB macOS troubleshooting guide ([#1975](https://github.com/rivet-gg/rivet/issues/1975)) ([e2f422f](https://github.com/rivet-gg/rivet/commit/e2f422f8a66cb31ae8ca4ee06bc8fed6aaf4da76))
* add lifecycle hooks implementation note ([#1875](https://github.com/rivet-gg/rivet/issues/1875)) ([edcf200](https://github.com/rivet-gg/rivet/commit/edcf2008e9683ff1823d57ff1586cc08b616af1e))
* clarify actor lifecycle method behaviors ([#1877](https://github.com/rivet-gg/rivet/issues/1877)) ([bc746c8](https://github.com/rivet-gg/rivet/commit/bc746c8ed30e2c9134872f074cea6fe881a28691))
* improve monorepo development documentation ([#1889](https://github.com/rivet-gg/rivet/issues/1889)) ([3f83b73](https://github.com/rivet-gg/rivet/commit/3f83b738fb158746ccc692567552ab9decbc7b19))
* reorganize self-hosting documentation ([#1876](https://github.com/rivet-gg/rivet/issues/1876)) ([0d2b412](https://github.com/rivet-gg/rivet/commit/0d2b41206378c5ae2ed928c202b58e40e6c07569))


### Chores

* **actors-sdk-embed:** auto-build & install manager dependencies ([#1964](https://github.com/rivet-gg/rivet/issues/1964)) ([2783a6f](https://github.com/rivet-gg/rivet/commit/2783a6fb962e42088411650ca35a2e09b1e22c15))
* **actors:** disable waiting for upgrade complete signal in api ([#1898](https://github.com/rivet-gg/rivet/issues/1898)) ([960390e](https://github.com/rivet-gg/rivet/commit/960390eda2ef007e37e1fde8008723ba03d77bf6))
* add deno.json warning in deploy task ([#1919](https://github.com/rivet-gg/rivet/issues/1919)) ([b533f6b](https://github.com/rivet-gg/rivet/commit/b533f6bcf09b4cce1cfe7031b722b3ce5da0cdbb))
* add mcp.run demo ([#1937](https://github.com/rivet-gg/rivet/issues/1937)) ([c7e224f](https://github.com/rivet-gg/rivet/commit/c7e224f3d6fbe55ff66cd39c9a5074bae021b2ff))
* add openhands support ([#1887](https://github.com/rivet-gg/rivet/issues/1887)) ([a829b16](https://github.com/rivet-gg/rivet/commit/a829b168748e41cb082bdf0a44c94c9a8282f542))
* add pkg.pr.new ([#1870](https://github.com/rivet-gg/rivet/issues/1870)) ([52b8aad](https://github.com/rivet-gg/rivet/commit/52b8aad971bd5dfe5398f6010a9f57deeee42e4c))
* clean up readme ([#1969](https://github.com/rivet-gg/rivet/issues/1969)) ([37214d7](https://github.com/rivet-gg/rivet/commit/37214d70eec580c80005066bc6835e318acdae02))
* **docker/monolith:** remove unneeded apt-transport-https dep ([#1985](https://github.com/rivet-gg/rivet/issues/1985)) ([bcebac3](https://github.com/rivet-gg/rivet/commit/bcebac3b831d536a7efae7093a0dcf0194222589))
* **docker:** set platform and target for dev server ([#1977](https://github.com/rivet-gg/rivet/issues/1977)) ([4a689ed](https://github.com/rivet-gg/rivet/commit/4a689edc7e3f72db9a9da4a9a52e0f087aaf631d))
* enable corepack in release workflow ([#1924](https://github.com/rivet-gg/rivet/issues/1924)) ([6373c30](https://github.com/rivet-gg/rivet/commit/6373c306a611c0c01f2d1f98696085ec40d9b9ea))
* enable git-lfs in release workflow ([#1987](https://github.com/rivet-gg/rivet/issues/1987)) ([e9f7686](https://github.com/rivet-gg/rivet/commit/e9f76865d02ceeb78150d835d85b7170d555d32a))
* **examples:** add  registry ([#1734](https://github.com/rivet-gg/rivet/issues/1734)) ([bf01b9a](https://github.com/rivet-gg/rivet/commit/bf01b9a24ad9e28471e894ceb3df5c3006d0e658))
* flatten all .gitignores in to root & merge in to .dockerignore ([#1981](https://github.com/rivet-gg/rivet/issues/1981)) ([35aac2b](https://github.com/rivet-gg/rivet/commit/35aac2b3662671ed2ae9f4e9482ea49310d17ab7))
* format package.json files arrays ([#1923](https://github.com/rivet-gg/rivet/issues/1923)) ([b46efac](https://github.com/rivet-gg/rivet/commit/b46efacf966d355ab37e9eebdbc0175aac1a17f5))
* **hub-embed:** auto-build & embed hub instead of pulling from releases ([#1966](https://github.com/rivet-gg/rivet/issues/1966)) ([093c92d](https://github.com/rivet-gg/rivet/commit/093c92d09a02d2e953db12168b89ac82b3aef7e3))
* **hub:** add sane defaults for .env ([#1965](https://github.com/rivet-gg/rivet/issues/1965)) ([0bf75dd](https://github.com/rivet-gg/rivet/commit/0bf75dd386dec87746057f05e5391e7c4ecd9539))
* **hub:** update dev command to use turbo ([#1984](https://github.com/rivet-gg/rivet/issues/1984)) ([8914ef7](https://github.com/rivet-gg/rivet/commit/8914ef7acdf8240611ec5cb21a46f1f4e8eff2cf))
* improve frontend apps workflow ([#1756](https://github.com/rivet-gg/rivet/issues/1756)) ([b66b5f0](https://github.com/rivet-gg/rivet/commit/b66b5f03568e2809db44f89bd68cd0c7d01fe425))
* limit log output to 100 lines ([#1994](https://github.com/rivet-gg/rivet/issues/1994)) ([de4433b](https://github.com/rivet-gg/rivet/commit/de4433b5ff21687d4eca5b883777fa5bfcf3e386))
* migrate to @luca/esbuild-deno-loader ([#1881](https://github.com/rivet-gg/rivet/issues/1881)) ([58521da](https://github.com/rivet-gg/rivet/commit/58521da754d54638a5f44556f087348a4e1ba611))
* release 25.1.0 ([601e44e](https://github.com/rivet-gg/rivet/commit/601e44ef4bc1c1f45d6b7b42b92d6ce3e4d1351d))
* remove jsr packages from js-utils-embed ([#1916](https://github.com/rivet-gg/rivet/issues/1916)) ([ebef9f2](https://github.com/rivet-gg/rivet/commit/ebef9f2598c928bed63ae700ddd61ad2ffa6871c))
* **sdks/actor/client:** log rpc name on error ([#1956](https://github.com/rivet-gg/rivet/issues/1956)) ([40214b4](https://github.com/rivet-gg/rivet/commit/40214b47711679dc9d6853eae415cbddd401feb9))
* **sdks/actor/runtime:** make all config parameters optional recursively ([#1954](https://github.com/rivet-gg/rivet/issues/1954)) ([879a0df](https://github.com/rivet-gg/rivet/commit/879a0df6bc3832469c8fefb9c7c3fdcedd26ce98))
* **sdks/actor/runtime:** remove legacy version check ([#1960](https://github.com/rivet-gg/rivet/issues/1960)) ([d8c9a9a](https://github.com/rivet-gg/rivet/commit/d8c9a9ab88c74676f88bf634807080b0690b5cf7))
* **sdks/actor:** remove all jsr libraries ([#1915](https://github.com/rivet-gg/rivet/issues/1915)) ([fcceb8d](https://github.com/rivet-gg/rivet/commit/fcceb8d6daa84a51c80fa8bad907424e7d0545ba))
* **toolchain:** include node polyfill for unsupported standard libraries ([#1894](https://github.com/rivet-gg/rivet/issues/1894)) ([9912720](https://github.com/rivet-gg/rivet/commit/99127204f23bb777a89a81676e8f2ac8aa18c2c1))
* **toolchain:** switch from es2020 to esnext target ([#1955](https://github.com/rivet-gg/rivet/issues/1955)) ([cfe97df](https://github.com/rivet-gg/rivet/commit/cfe97df428e37cddd4c2b4f3862ebdd541c9c35a))
* **toolchain:** update default templates for npm compat ([#1914](https://github.com/rivet-gg/rivet/issues/1914)) ([4266f46](https://github.com/rivet-gg/rivet/commit/4266f4658465eba24cbae9e99bcd916401394efc))
* update apis ([#1922](https://github.com/rivet-gg/rivet/issues/1922)) ([d83ab10](https://github.com/rivet-gg/rivet/commit/d83ab10dc3daf138733b5dd5cf29adfe842724c6))
* update dev command from start to dev ([#1967](https://github.com/rivet-gg/rivet/issues/1967)) ([b88a5da](https://github.com/rivet-gg/rivet/commit/b88a5da17ce607057649e1bf57c0d4e6f1b19bb1))
* update examples ([#1936](https://github.com/rivet-gg/rivet/issues/1936)) ([c6ca79e](https://github.com/rivet-gg/rivet/commit/c6ca79ef05332026adde1a770111b941b7bcee64))
* update generateArticle script extension to .js ([#1993](https://github.com/rivet-gg/rivet/issues/1993)) ([e65b5cb](https://github.com/rivet-gg/rivet/commit/e65b5cbd1b05c9b36635e66ca8b3d4f35abda4a1))
* update quickstart ([#1905](https://github.com/rivet-gg/rivet/issues/1905)) ([6856eda](https://github.com/rivet-gg/rivet/commit/6856edadaf1ac1d5419ba88a00b0de33e16b775a))
* update readme ([#1886](https://github.com/rivet-gg/rivet/issues/1886)) ([9351758](https://github.com/rivet-gg/rivet/commit/9351758a5401e092e530717f472f3a4673a9bab5))
* update site ([#1840](https://github.com/rivet-gg/rivet/issues/1840)) ([dd1781b](https://github.com/rivet-gg/rivet/commit/dd1781b77f9db418a27c6f361a3a11be3513569d))
* update site ([#1893](https://github.com/rivet-gg/rivet/issues/1893)) ([fda6ecb](https://github.com/rivet-gg/rivet/commit/fda6ecbbce08233a1bb4e268c5c4b4dbc569abce))
* update website ([#1856](https://github.com/rivet-gg/rivet/issues/1856)) ([2a21415](https://github.com/rivet-gg/rivet/commit/2a2141572225097a12f78e78d0b7caf3d88202c1))
* upgrade system-test to npm ([#1918](https://github.com/rivet-gg/rivet/issues/1918)) ([4e9cde8](https://github.com/rivet-gg/rivet/commit/4e9cde839f1e6fc0f715ad2bc65a0059f5f56b9f))
* **workflow:** increase poll intervals ([#1990](https://github.com/rivet-gg/rivet/issues/1990)) ([ea5a3ba](https://github.com/rivet-gg/rivet/commit/ea5a3ba21ef98946db53b7c1277034aa6e2f0517))

## [24.6.2](https://github.com/rivet-gg/rivet/compare/v24.6.2-rc.1...v24.6.2) (2025-01-13)


### Features

* add url to actor ports ([#1811](https://github.com/rivet-gg/rivet/issues/1811)) ([8a80712](https://github.com/rivet-gg/rivet/commit/8a807123849bc8638613ca68a5aa957214b2c3ea))
* **site:** add support for intersection schemas ([#1725](https://github.com/rivet-gg/rivet/issues/1725)) ([9b9e51c](https://github.com/rivet-gg/rivet/commit/9b9e51c074aa8e4b04d4db11b094e36b691fb65e))


### Bug Fixes

* add validation around internal_port ([#1809](https://github.com/rivet-gg/rivet/issues/1809)) ([37ae0f5](https://github.com/rivet-gg/rivet/commit/37ae0f5d49ab65cab98e84f930b8dc35bbbc3c69))
* **docker/dev-full:** force linux/amd64 for server container ([#1818](https://github.com/rivet-gg/rivet/issues/1818)) ([948a1c0](https://github.com/rivet-gg/rivet/commit/948a1c0f79a5f40b2ab4296c93a0179ebd722c0d))
* fix manager log rotation ([#1806](https://github.com/rivet-gg/rivet/issues/1806)) ([4e96fd7](https://github.com/rivet-gg/rivet/commit/4e96fd73bc3e2e92d16ca449c285cc03fff0f06f))
* fix various wf bugs, ds undrain, nonreporting metric ([#1791](https://github.com/rivet-gg/rivet/issues/1791)) ([4b86429](https://github.com/rivet-gg/rivet/commit/4b86429f99b8d0e5f38fa70a3a3749ea9f12ac95))
* fix workflow listen_with_timeout history bug ([#1822](https://github.com/rivet-gg/rivet/issues/1822)) ([c315617](https://github.com/rivet-gg/rivet/commit/c31561791fe3167f3edb090a54bf1fd243b880c0))
* **hub:** blank screen when `CLAIMS_ENTITLEMENT_EXPIRED` ([#1730](https://github.com/rivet-gg/rivet/issues/1730)) ([2a2bba4](https://github.com/rivet-gg/rivet/commit/2a2bba483a6c89c82cb31d48deae78651240bd12))
* **hub:** failed request to rivet.gg/changelog.json causes error that blocks usage of hub ([#1731](https://github.com/rivet-gg/rivet/issues/1731)) ([86924c9](https://github.com/rivet-gg/rivet/commit/86924c9335705460a6450f6b7086ec2f15b8f318))
* **pegboard:** build with bullseye for correct glibc version ([#1823](https://github.com/rivet-gg/rivet/issues/1823)) ([936bbdb](https://github.com/rivet-gg/rivet/commit/936bbdb6510d2c1a2b060fbdcb015eba7dc4c5e6))
* remove unused workflow idxs ([#1810](https://github.com/rivet-gg/rivet/issues/1810)) ([8df19e8](https://github.com/rivet-gg/rivet/commit/8df19e89764c6197d09de6005aea361d544e51dc))
* **sdks/actor/runtime:** bad import path for 40_rivet_kv.d.ts ([#1819](https://github.com/rivet-gg/rivet/issues/1819)) ([5c599c6](https://github.com/rivet-gg/rivet/commit/5c599c645080dd1258ea4a0292f8dcf052373265))
* **site:** add missing redirects to opengamebackend.org ([#1726](https://github.com/rivet-gg/rivet/issues/1726)) ([d84fa85](https://github.com/rivet-gg/rivet/commit/d84fa85f6550f0f1087a83f8dfe54e059b3daec4))
* **site:** hide dropdown when hovering over tab without dropdown ([#1727](https://github.com/rivet-gg/rivet/issues/1727)) ([d9d16f7](https://github.com/rivet-gg/rivet/commit/d9d16f757bbc7663180d07c2aed2c5cfa8d65873))
* **site:** pricing page anchors to bottom of page ([#1728](https://github.com/rivet-gg/rivet/issues/1728)) ([6903892](https://github.com/rivet-gg/rivet/commit/69038922002d397f3fe2e4492ad0e9e2ef9292e0))


### Code Refactoring

* **hub:** change plan names in billing badge ([#1732](https://github.com/rivet-gg/rivet/issues/1732)) ([9727f21](https://github.com/rivet-gg/rivet/commit/9727f216d3884f94ee438f6419caafdcb5bc7e0b))


### Chores

* **fe/deps:** update yarn.lock ([#1827](https://github.com/rivet-gg/rivet/issues/1827)) ([c95e36b](https://github.com/rivet-gg/rivet/commit/c95e36bab57d1617bbfef592dd65cb4db20af838))
* **hub:** move hub source code to rivet ([#1729](https://github.com/rivet-gg/rivet/issues/1729)) ([ac89fe1](https://github.com/rivet-gg/rivet/commit/ac89fe1142a303529d6ae17a5661a847aeb4bde5))
* **hub:** update hub ([#1835](https://github.com/rivet-gg/rivet/issues/1835)) ([0db5070](https://github.com/rivet-gg/rivet/commit/0db50707a9e41f27d62d05a77489d23b233e4aa6))
* improve actors allocated metric ([#1812](https://github.com/rivet-gg/rivet/issues/1812)) ([46ea373](https://github.com/rivet-gg/rivet/commit/46ea3738a1156b57e1030df679d3ed36b235ca35))
* **justfile:** add system test shortcut command ([#1825](https://github.com/rivet-gg/rivet/issues/1825)) ([1ce6f44](https://github.com/rivet-gg/rivet/commit/1ce6f440a68330ddde26aa150f203a138546feed))
* **mm:** handle orphaned runs without matching lobbies ([#1826](https://github.com/rivet-gg/rivet/issues/1826)) ([34e333c](https://github.com/rivet-gg/rivet/commit/34e333c819c3d6d3c77aabc19e514141398bf962))
* release 24.6.2 ([cc607c3](https://github.com/rivet-gg/rivet/commit/cc607c321636eb4072c4707bb6862d3e7afedcfb))
* release 24.6.2 ([221567e](https://github.com/rivet-gg/rivet/commit/221567edf18bee18caeefccb58164ef47ef5173c))
* **release:** update version to 24.6.2 ([a18a756](https://github.com/rivet-gg/rivet/commit/a18a75650ca20c15cf6f478a712c66b6ef4045ce))
* sync fe repo ([#1829](https://github.com/rivet-gg/rivet/issues/1829)) ([df42570](https://github.com/rivet-gg/rivet/commit/df4257038a5b2cc511f0c7d3b05cfca020bc01f1))

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
