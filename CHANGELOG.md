# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Calendar Versioning](https://calver.org/).

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
