# [1.0.0-beta.10](https://github.com/iotaledger/inx-chronicle/compare/v1.0.0-beta.9...v1.0.0-beta.10) (2022-08-17)


### Features

* **analytics:** add `richest-addresses` and `token-distribution` endpoints ([#523](https://github.com/iotaledger/inx-chronicle/issues/523)) ([99049b6](https://github.com/iotaledger/inx-chronicle/commit/99049b6dbe36943418d5cfc2ae676d6520840927))
* **docker:** `production` builds and support `hornet-nest` ([#557](https://github.com/iotaledger/inx-chronicle/issues/557)) ([70fe622](https://github.com/iotaledger/inx-chronicle/commit/70fe622607f2024ee0eec67c35994cd5f1083090))
* **metrics:** use `tracing` instead of `log` ([#554](https://github.com/iotaledger/inx-chronicle/issues/554)) ([3a585ad](https://github.com/iotaledger/inx-chronicle/commit/3a585ad2f83905d49e8714cba77091ca1010b17f))



# [1.0.0-beta.9](https://github.com/iotaledger/inx-chronicle/compare/v1.0.0-beta.8...v1.0.0-beta.9) (2022-08-16)


### Bug Fixes

* **api:** update Indexer API query params ([#548](https://github.com/iotaledger/inx-chronicle/issues/548)) ([9451e88](https://github.com/iotaledger/inx-chronicle/commit/9451e8813c97d3f77090d9f80c9f0fda311f2fdf))
* **inx:** stream mapper ([#532](https://github.com/iotaledger/inx-chronicle/issues/532)) ([4d6a13a](https://github.com/iotaledger/inx-chronicle/commit/4d6a13a5176ba9aa76520e6f4f97137a84f30292))



# [1.0.0-beta.8](https://github.com/iotaledger/inx-chronicle/compare/v1.0.0-beta.7...v1.0.0-beta.8) (2022-08-05)


### Bug Fixes

* **api:** activity analytics ([#529](https://github.com/iotaledger/inx-chronicle/issues/529)) ([a9b294a](https://github.com/iotaledger/inx-chronicle/commit/a9b294a47f0f633d027e31b127f9fded7d06dc4a))
* **inx:** stream-based mapper ([#528](https://github.com/iotaledger/inx-chronicle/issues/528)) ([0d29b37](https://github.com/iotaledger/inx-chronicle/commit/0d29b379d37a9b5f29bb58fa351c7cc25b40b8fb))



# [1.0.0-beta.7](https://github.com/iotaledger/inx-chronicle/compare/v1.0.0-beta.6...v1.0.0-beta.7) (2022-08-04)


### Bug Fixes

* **api:** remove `gaps` endpoint ([#511](https://github.com/iotaledger/inx-chronicle/issues/511)) ([2befce8](https://github.com/iotaledger/inx-chronicle/commit/2befce8639653b402227ebd1b7214cac7cfc9954))


### Features

* **analytics:** implement ledger and most activity-based analytics ([#482](https://github.com/iotaledger/inx-chronicle/issues/482)) ([755f9d2](https://github.com/iotaledger/inx-chronicle/commit/755f9d2efe0006da5f0bd0f7a72bd6d8f07360be))
* **inx:** switch to stream-based updates ([#524](https://github.com/iotaledger/inx-chronicle/issues/524)) ([8ded3c0](https://github.com/iotaledger/inx-chronicle/commit/8ded3c0b3400e25e46443ac7b1aa7ea77e0b5da3))



# [1.0.0-beta.6](https://github.com/iotaledger/inx-chronicle/compare/v1.0.0-beta.5...v1.0.0-beta.6) (2022-08-02)


### Bug Fixes

* **db:** 500 on hitting the `balance/` endpoint ([#491](https://github.com/iotaledger/inx-chronicle/issues/491)) ([fe4a71c](https://github.com/iotaledger/inx-chronicle/commit/fe4a71c59eadf2c8281474ee94b5f3a437882159))


### Features

* **docker:** add `depends_on` for `inx-chronicle` ([#512](https://github.com/iotaledger/inx-chronicle/issues/512)) ([6674cb4](https://github.com/iotaledger/inx-chronicle/commit/6674cb41bd427629a6f5fba82f34a1b02c4d0c2f))



# [1.0.0-beta.5](https://github.com/iotaledger/inx-chronicle/compare/v1.0.0-beta.4...v1.0.0-beta.5) (2022-08-01)


### Bug Fixes

* **api:** re-enable utxo-changes route ([#490](https://github.com/iotaledger/inx-chronicle/issues/490)) ([3697f27](https://github.com/iotaledger/inx-chronicle/commit/3697f27f761a2547fbcf0ea528c9ed01d2407ac6))
* **db:** better indexation for `insert_ledger_updates` ([#507](https://github.com/iotaledger/inx-chronicle/issues/507)) ([dd4d796](https://github.com/iotaledger/inx-chronicle/commit/dd4d79626bf246a9d2c8c351a70b29be39a3e8bd))
* **inx:** remove `ConeStream` and `Syncer` ([#500](https://github.com/iotaledger/inx-chronicle/issues/500)) ([4dc2aa1](https://github.com/iotaledger/inx-chronicle/commit/4dc2aa15433b8a118b336c10e72d2f06e6d989dc))


### Features

* **api:** deny unknown query fields ([#492](https://github.com/iotaledger/inx-chronicle/issues/492)) ([7258d58](https://github.com/iotaledger/inx-chronicle/commit/7258d58b4fcdc6c59ed9cce0d8213c2ff8ced9e9))
* **db:** better reporting and logging ([#493](https://github.com/iotaledger/inx-chronicle/issues/493)) ([8eaddc6](https://github.com/iotaledger/inx-chronicle/commit/8eaddc6e8eb7cca46eb9ff348a63b9b40a85b2fd))
* **docker:** use `replSet` in `docker-compose` ([#506](https://github.com/iotaledger/inx-chronicle/issues/506)) ([13ed2c5](https://github.com/iotaledger/inx-chronicle/commit/13ed2c5a22ab51e6c8d3b1ff24a620f521a7ecc5))
* **inx:** add time logging ([#508](https://github.com/iotaledger/inx-chronicle/issues/508)) ([df329a3](https://github.com/iotaledger/inx-chronicle/commit/df329a3b12ea0e285fbcb6f2e8d5d251bec57d53))



# [1.0.0-beta.4](https://github.com/iotaledger/inx-chronicle/compare/v1.0.0-beta.3...v1.0.0-beta.4) (2022-07-28)


### Bug Fixes

* **inx:** sync gaps with single milestone ([#487](https://github.com/iotaledger/inx-chronicle/issues/487)) ([d689c8c](https://github.com/iotaledger/inx-chronicle/commit/d689c8c33e190304f6e070e7ae5d1632507b824a))



# [1.0.0-beta.3](https://github.com/iotaledger/inx-chronicle/compare/v1.0.0-beta.2...v1.0.0-beta.3) (2022-07-28)


### Bug Fixes

* **db:** projection in `get_gaps` ([#485](https://github.com/iotaledger/inx-chronicle/issues/485)) ([9170c11](https://github.com/iotaledger/inx-chronicle/commit/9170c11ef76ea579b146104bd6d63ed7f531a86c))
* **indexer:** correct parsing error in indexer output by id ([#481](https://github.com/iotaledger/inx-chronicle/issues/481)) ([eb212ec](https://github.com/iotaledger/inx-chronicle/commit/eb212ecbb9a632aeabe4af927893535e3ff3e184))



# [1.0.0-beta.2](https://github.com/iotaledger/inx-chronicle/compare/v1.0.0-beta.1...v1.0.0-beta.2) (2022-07-27)


### Bug Fixes

* **inx:** better error reporting ([#479](https://github.com/iotaledger/inx-chronicle/issues/479)) ([14329b6](https://github.com/iotaledger/inx-chronicle/commit/14329b62f331e1c7474a653bffbf35f52f0e6f27))



# [1.0.0-beta.1](https://github.com/iotaledger/inx-chronicle/compare/v0.1.0-alpha.15...v1.0.0-beta.1) (2022-07-27)


### Bug Fixes

* **api:** add max page size and tests ([#468](https://github.com/iotaledger/inx-chronicle/issues/468)) ([ed797eb](https://github.com/iotaledger/inx-chronicle/commit/ed797eb70494324ba198a648eb0acb689b409d86))
* **api:** fix missing camel case renaming ([#457](https://github.com/iotaledger/inx-chronicle/issues/457)) ([d0446d2](https://github.com/iotaledger/inx-chronicle/commit/d0446d2a8f5fcd9e59d5642585cb8d3a1e9d3e92))
* **db:** fix block children endpoint ([#475](https://github.com/iotaledger/inx-chronicle/issues/475)) ([0ad9ba0](https://github.com/iotaledger/inx-chronicle/commit/0ad9ba098d8467865fefed2675874f73289da136))
* **types:** inputs commitment conversion ([#459](https://github.com/iotaledger/inx-chronicle/issues/459)) ([ceb736b](https://github.com/iotaledger/inx-chronicle/commit/ceb736b33b442b44d1a50a8f642bfad45296e5b0))


### Features

* **api:** implement `balance/` endpoint ([#388](https://github.com/iotaledger/inx-chronicle/issues/388)) ([57ec3aa](https://github.com/iotaledger/inx-chronicle/commit/57ec3aade1d74c0a365ed538da933e4ca936e286))
* **indexer:** add Indexer API ([#429](https://github.com/iotaledger/inx-chronicle/issues/429)) ([822b0a5](https://github.com/iotaledger/inx-chronicle/commit/822b0a592bb114a7318bac0874ec13e9c3d9cee5))
* **inx:** use `bee-inx` ([#470](https://github.com/iotaledger/inx-chronicle/issues/470)) ([1426dc8](https://github.com/iotaledger/inx-chronicle/commit/1426dc878d764fd3c81195c52a9e205028a9f710))



# [0.1.0-alpha.15](https://github.com/iotaledger/inx-chronicle/compare/v0.1.0-alpha.14...v0.1.0-alpha.15) (2022-07-19)


### Bug Fixes

* **ci:** qualify `Report` to avoid build errors ([#454](https://github.com/iotaledger/inx-chronicle/issues/454)) ([160b6af](https://github.com/iotaledger/inx-chronicle/commit/160b6aff63fc42460d08c41170c2adb19964a1f4))



# [0.1.0-alpha.14](https://github.com/iotaledger/inx-chronicle/compare/v0.1.0-alpha.13...v0.1.0-alpha.14) (2022-07-15)


### Bug Fixes

* **ci:** improve feature handling and CI ([#428](https://github.com/iotaledger/inx-chronicle/issues/428)) ([633767d](https://github.com/iotaledger/inx-chronicle/commit/633767d9cf45840ff29f66e6c3f25cbab7b770b2))
* **db:** ledger updates sort order ([#441](https://github.com/iotaledger/inx-chronicle/issues/441)) ([df0786d](https://github.com/iotaledger/inx-chronicle/commit/df0786da13bfaca016c6da741925c5fc33ff553b))



# [0.1.0-alpha.13](https://github.com/iotaledger/inx-chronicle/compare/v0.1.0-alpha.12...v0.1.0-alpha.13) (2022-07-14)


### Bug Fixes

* **api:** improve `is_healthy` checking ([#436](https://github.com/iotaledger/inx-chronicle/issues/436)) ([683efa4](https://github.com/iotaledger/inx-chronicle/commit/683efa48396445e72b9274532de3e908dd8dfc25))



# [0.1.0-alpha.12](https://github.com/iotaledger/inx-chronicle/compare/v0.1.0-alpha.11...v0.1.0-alpha.12) (2022-07-12)


### Bug Fixes

* **api:** remove `inx` from `is_healthy` check ([#415](https://github.com/iotaledger/inx-chronicle/issues/415)) ([6a7bdce](https://github.com/iotaledger/inx-chronicle/commit/6a7bdce3cb22d682a2d4537842a9e47d09136280))
* properly merge `ENV` and `config.template.toml` ([#418](https://github.com/iotaledger/inx-chronicle/issues/418)) ([3167d8d](https://github.com/iotaledger/inx-chronicle/commit/3167d8de47a7dd70f9052a302e8a3fb6aad59f54))


### Features

* **analytics:** enable `/addresses` endpoint ([#420](https://github.com/iotaledger/inx-chronicle/issues/420)) ([fc082cd](https://github.com/iotaledger/inx-chronicle/commit/fc082cdd9c5e3e186c46df6cf13bc45bb71e8678))



# [0.1.0-alpha.11](https://github.com/iotaledger/inx-chronicle/compare/v0.1.0-alpha.10...v0.1.0-alpha.11) (2022-07-11)


### Bug Fixes

* add `ErrorLevel` trait to specify error log levels ([#405](https://github.com/iotaledger/inx-chronicle/issues/405)) ([3cc1cac](https://github.com/iotaledger/inx-chronicle/commit/3cc1cace9edcc1e5edae16185ce4abb4cc7a1b99))
* **api:** add ledger index to output queries ([#336](https://github.com/iotaledger/inx-chronicle/issues/336)) ([f35d103](https://github.com/iotaledger/inx-chronicle/commit/f35d1036870b957f0695277a92c93fb87eea71a0))
* **db:** add `unlock_condition` to `id_index` ([#402](https://github.com/iotaledger/inx-chronicle/issues/402)) ([e0145b3](https://github.com/iotaledger/inx-chronicle/commit/e0145b376ee12cdae792af62283e9c2e669804d7))
* **metrics:** correctly set Prometheus targets ([#404](https://github.com/iotaledger/inx-chronicle/issues/404)) ([250ccbf](https://github.com/iotaledger/inx-chronicle/commit/250ccbfcbcb2b9e8dc9ecffb37bff1e6df3ff23f))


### Features

* **config:** set `api`, `inx`, `metrics` features dynamically ([#397](https://github.com/iotaledger/inx-chronicle/issues/397)) ([3140767](https://github.com/iotaledger/inx-chronicle/commit/31407675d1890e1edbfd94ed770a58dcb9366e45))
* **metrics:** differentiate b/n `metrics` and `metrics-debug` ([#403](https://github.com/iotaledger/inx-chronicle/issues/403)) ([6839203](https://github.com/iotaledger/inx-chronicle/commit/68392034f6b62559d6992866a2a90c9b3728ece9))



# [0.1.0-alpha.10](https://github.com/iotaledger/inx-chronicle/compare/v0.1.0-alpha.9...v0.1.0-alpha.10) (2022-07-06)


### Bug Fixes

* **db:** fix sorted paginated ledger update queries ([#371](https://github.com/iotaledger/inx-chronicle/issues/371)) ([7595aea](https://github.com/iotaledger/inx-chronicle/commit/7595aea36289d048be485d86838a816828e5c89d))
* **db:** prevent duplicate inserts of `LedgerUpdateDocument`s ([#373](https://github.com/iotaledger/inx-chronicle/issues/373)) ([d961653](https://github.com/iotaledger/inx-chronicle/commit/d961653b5e484ec25f07d2568ee0ce981c34ca96))
* **platform:** support shutdown in Docker environment ([#366](https://github.com/iotaledger/inx-chronicle/issues/366)) ([8cead0e](https://github.com/iotaledger/inx-chronicle/commit/8cead0e89cb9678d75114780cba70c03dfa9cbd2))


### Features

* **api:** implement `is_healthy` check for `health/` API endpoint ([#339](https://github.com/iotaledger/inx-chronicle/issues/339)) ([7c95e56](https://github.com/iotaledger/inx-chronicle/commit/7c95e564121008904765641a3bce8047e07d1a33))



# [0.1.0-alpha.9](https://github.com/iotaledger/inx-chronicle/compare/v0.1.0-alpha.8...v0.1.0-alpha.9) (2022-06-30)


### Bug Fixes

* **api:** add serde rename on fields ([#362](https://github.com/iotaledger/inx-chronicle/issues/362)) ([5a8bab7](https://github.com/iotaledger/inx-chronicle/commit/5a8bab7ff11e3f6d6195f44c9cc3bec87479ef93))
* **config:** print file path on file read error ([#354](https://github.com/iotaledger/inx-chronicle/issues/354)) ([09849bc](https://github.com/iotaledger/inx-chronicle/commit/09849bc5d7d9a906f542386c5544e2374a1cf590))


### Features

* **api:** add `ledger/updates/by-milestone` endpoint ([#326](https://github.com/iotaledger/inx-chronicle/issues/326)) ([dbef5f1](https://github.com/iotaledger/inx-chronicle/commit/dbef5f13573a6021d20e8ff38022a13d47073e95))
* **api:** support sort option in queries ([#363](https://github.com/iotaledger/inx-chronicle/issues/363)) ([db116f3](https://github.com/iotaledger/inx-chronicle/commit/db116f3aca5fb43a466ea574637f49c3f2d130fb))



# [0.1.0-alpha.8](https://github.com/iotaledger/inx-chronicle/compare/v0.1.0-alpha.7...v0.1.0-alpha.8) (2022-06-27)


### Bug Fixes

* **api:** clean up receipt route handlers and db queries ([#344](https://github.com/iotaledger/inx-chronicle/issues/344)) ([aa09e5c](https://github.com/iotaledger/inx-chronicle/commit/aa09e5c0baab48d83351755224584fe317d55733))
* **doc:** fully document `config.template.toml` ([#345](https://github.com/iotaledger/inx-chronicle/issues/345)) ([ebd200c](https://github.com/iotaledger/inx-chronicle/commit/ebd200cb4b7e8db425148b91c9fe832d9c54522a))


### Features

* **api:** add JWT authentication ([#281](https://github.com/iotaledger/inx-chronicle/issues/281)) ([6510cb1](https://github.com/iotaledger/inx-chronicle/commit/6510cb1747a4cc1de3420b53e0df216740452a1f)), closes [#205](https://github.com/iotaledger/inx-chronicle/issues/205)
* **api:** implement the raw bytes endpoint for milestones ([#340](https://github.com/iotaledger/inx-chronicle/issues/340)) ([0134fc4](https://github.com/iotaledger/inx-chronicle/commit/0134fc471381d32cb6ea74b4904dd5e327884e04))
* **inx:** more detailed logging of INX events ([#349](https://github.com/iotaledger/inx-chronicle/issues/349)) ([986cdbf](https://github.com/iotaledger/inx-chronicle/commit/986cdbf6d8524caf9d47f141562fe59436f3f932))



# [0.1.0-alpha.7](https://github.com/iotaledger/inx-chronicle/compare/v0.1.0-alpha.6...v0.1.0-alpha.7) (2022-06-22)


### Bug Fixes

* **api:** rename `explorer` to `history` ([#313](https://github.com/iotaledger/inx-chronicle/issues/313)) ([517e53e](https://github.com/iotaledger/inx-chronicle/commit/517e53edbfcffa0da5d6cca1220a16b2f220bf53))



# [0.1.0-alpha.6](https://github.com/iotaledger/inx-chronicle/compare/v0.1.0-alpha.5...v0.1.0-alpha.6) (2022-06-21)


### Features

* **analytics:** add transaction analytics ([#292](https://github.com/iotaledger/inx-chronicle/issues/292)) ([8af160f](https://github.com/iotaledger/inx-chronicle/commit/8af160f32659f3fe15c65a98dc96e921ef51b75f))


### Performance Improvements

* **inx:** remove clones in ledger update stream ([#298](https://github.com/iotaledger/inx-chronicle/issues/298)) ([f5606cb](https://github.com/iotaledger/inx-chronicle/commit/f5606cbdcc94ae05ed9c660d5d40aced766939a8))



# [0.1.0-alpha.5](https://github.com/iotaledger/inx-chronicle/compare/v0.1.0-alpha.4...v0.1.0-alpha.5) (2022-06-15)


### Bug Fixes

* **db:** fix compound `transaction_id_index` ([#290](https://github.com/iotaledger/inx-chronicle/issues/290)) ([afc9dbb](https://github.com/iotaledger/inx-chronicle/commit/afc9dbb56051f2d1ae1227a484efa7045b807714))


### Features

* add partial index for transaction id ([#293](https://github.com/iotaledger/inx-chronicle/issues/293)) ([dca0e88](https://github.com/iotaledger/inx-chronicle/commit/dca0e881e1cdf6390bce987b321416d010246932))



# [0.1.0-alpha.4](https://github.com/iotaledger/inx-chronicle/compare/v0.1.0-alpha.3...v0.1.0-alpha.4) (2022-06-15)


### Bug Fixes

* **db:** make `transaction_id_index` unique ([#287](https://github.com/iotaledger/inx-chronicle/issues/287)) ([622eba3](https://github.com/iotaledger/inx-chronicle/commit/622eba320d991dcbff0f49390c8b2acc3e50d250))
* **metrics:** use `with_graceful_shutdown` for metrics server ([#285](https://github.com/iotaledger/inx-chronicle/issues/285)) ([b91c1af](https://github.com/iotaledger/inx-chronicle/commit/b91c1af989369385c46bc3541ddf079d8294379a))



# [0.1.0-alpha.3](https://github.com/iotaledger/inx-chronicle/compare/v0.1.0-alpha.2...v0.1.0-alpha.3) (2022-06-14)



# [0.1.0-alpha.2](https://github.com/iotaledger/inx-chronicle/compare/3880235ca0fc51d19884ad4bd32ceaea958b4b7d...v0.1.0-alpha.2) (2022-06-14)


### Bug Fixes

* `unreachable_pub` instances and add compiler warning ([#143](https://github.com/iotaledger/inx-chronicle/issues/143)) ([ea77593](https://github.com/iotaledger/inx-chronicle/commit/ea77593b1cfc82d55b46ebaf98b6eeabe830de02))
* **api:** clean up `impl_success_response` ([#130](https://github.com/iotaledger/inx-chronicle/issues/130)) ([e5097d7](https://github.com/iotaledger/inx-chronicle/commit/e5097d719584c837fb8b958d29b0a8ce8018f7a8))
* **ci:** create images on `release` instead of `tags` ([#272](https://github.com/iotaledger/inx-chronicle/issues/272)) ([62f9f6c](https://github.com/iotaledger/inx-chronicle/commit/62f9f6cbdad3a0cb0847e19ab918fdcb08ea608c))
* **collector:** merge the collector and inx ([#141](https://github.com/iotaledger/inx-chronicle/issues/141)) ([1406a9f](https://github.com/iotaledger/inx-chronicle/commit/1406a9f6e87ec64c638d3ace15567ed45924b7a4))
* **collector:** re-add list of `visited` messages ([#131](https://github.com/iotaledger/inx-chronicle/issues/131)) ([02bcdbb](https://github.com/iotaledger/inx-chronicle/commit/02bcdbb541999ebdb261b2ee9f5484f2f32c5ef0))
* **db:** Rename `message_id` to `_id`  ([#172](https://github.com/iotaledger/inx-chronicle/issues/172)) ([d5da16a](https://github.com/iotaledger/inx-chronicle/commit/d5da16a3780c7298e1fe62d36c5707321b7d5bc0))
* **deps:** update Hornet to `v2.0.0-alpha14` ([#189](https://github.com/iotaledger/inx-chronicle/issues/189)) ([7f21210](https://github.com/iotaledger/inx-chronicle/commit/7f2121071730e4cc75fcb79b5fe43c7c890758e9))
* **docker:** fix `Dockerfile` ([#194](https://github.com/iotaledger/inx-chronicle/issues/194)) ([d0be40e](https://github.com/iotaledger/inx-chronicle/commit/d0be40e8e53484433fb74e85a2f357a2628b38ef))
* **docker:** revert to `--release` profile due to `cargo-chef` ([#220](https://github.com/iotaledger/inx-chronicle/issues/220)) ([82be5ec](https://github.com/iotaledger/inx-chronicle/commit/82be5ec027e9ec8d75d4f15397784f25edb4f414))
* Make `solidifiers` immutable ([#159](https://github.com/iotaledger/inx-chronicle/issues/159)) ([8c55537](https://github.com/iotaledger/inx-chronicle/commit/8c5553720c2d8d5d09f90d519643bbe9ad989684))
* **runtime:** use `warn!` instead of `error!` ([#271](https://github.com/iotaledger/inx-chronicle/issues/271)) ([6389916](https://github.com/iotaledger/inx-chronicle/commit/638991612392d9eb16b4920cc7ba42fcc3f1082c))
* **syncer:** clamp the syncer milestones properly ([#203](https://github.com/iotaledger/inx-chronicle/issues/203)) ([8cf40c5](https://github.com/iotaledger/inx-chronicle/commit/8cf40c5817cfbdd67f61dfe269500b281df33014))
* update `bee-metrics` and log first error for process metrics ([#176](https://github.com/iotaledger/inx-chronicle/issues/176)) ([09d1cd1](https://github.com/iotaledger/inx-chronicle/commit/09d1cd108000cfe81217d5708c6604ed530a3658))


### Features

* add `incoming_requests` API metric ([#162](https://github.com/iotaledger/inx-chronicle/issues/162)) ([1f9de59](https://github.com/iotaledger/inx-chronicle/commit/1f9de59fc6e28a18141fd3a022bdc393a9228ba6))
* add `tokio-console` tracing ([#115](https://github.com/iotaledger/inx-chronicle/issues/115)) ([dc4ae5c](https://github.com/iotaledger/inx-chronicle/commit/dc4ae5cf1fdd32f7174bf461218f55f342524bc7))
* add manual actor name impls ([#204](https://github.com/iotaledger/inx-chronicle/issues/204)) ([24ab7a2](https://github.com/iotaledger/inx-chronicle/commit/24ab7a237657f59eab14d6454f30fd9ab462722e))
* **build:** optimize production builds ([#173](https://github.com/iotaledger/inx-chronicle/issues/173)) ([67a07e9](https://github.com/iotaledger/inx-chronicle/commit/67a07e91919f5cc67b3a6657ba7998ad261cca3b))
* **inx:** retry on INX connection errors ([#243](https://github.com/iotaledger/inx-chronicle/issues/243)) ([7173fd3](https://github.com/iotaledger/inx-chronicle/commit/7173fd33ba3cb3b8578400378edd570e04003437))
* **metrics:** add channel metrics to runtime ([#169](https://github.com/iotaledger/inx-chronicle/issues/169)) ([afbf3a4](https://github.com/iotaledger/inx-chronicle/commit/afbf3a4410254f4c306abed8fd43b050c430c990))
* **metrics:** add initial support for metrics ([#123](https://github.com/iotaledger/inx-chronicle/issues/123)) ([c6ed8a6](https://github.com/iotaledger/inx-chronicle/commit/c6ed8a68b09a745a127f57ee57cef6313eda4059))
* **metrics:** add size metric to MongoDB ([#183](https://github.com/iotaledger/inx-chronicle/issues/183)) ([ef8b125](https://github.com/iotaledger/inx-chronicle/commit/ef8b1251be7c1b0844328bbaca876d2f4b5ac1d8))
* **metrics:** add solidification counter metric ([#170](https://github.com/iotaledger/inx-chronicle/issues/170)) ([46f5bcb](https://github.com/iotaledger/inx-chronicle/commit/46f5bcb83afccb1b01cabadb16f150fab59a9b7a))
* **model:** use arrays to store bytes when possible ([#206](https://github.com/iotaledger/inx-chronicle/issues/206)) ([a304a94](https://github.com/iotaledger/inx-chronicle/commit/a304a94125282df0ca38921e9b25531f7b2fd248))
* **types:** add Copy and Into<Bson> impls ([#230](https://github.com/iotaledger/inx-chronicle/issues/230)) ([165303c](https://github.com/iotaledger/inx-chronicle/commit/165303c064034a8a20ffd09df8c6217bd60ffaa0))


### Reverts

* Revert "Remove cross-plattform Docker images (#60)" (#62) ([3880235](https://github.com/iotaledger/inx-chronicle/commit/3880235ca0fc51d19884ad4bd32ceaea958b4b7d)), closes [#60](https://github.com/iotaledger/inx-chronicle/issues/60) [#62](https://github.com/iotaledger/inx-chronicle/issues/62)
