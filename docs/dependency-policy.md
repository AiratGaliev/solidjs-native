# Dependency Policy and Version Baseline

- Статус: обязательная инженерная политика.
- Дата baseline: 2026-11-10.
- Назначение: выбирать версии SolidJS Native самостоятельно, не наследуя dependency versions Native Shell или другого референсного проекта.

## 1. Главный принцип

SolidJS Native использует последние стабильные версии зависимостей на момент их принятия или планового обновления, при условии подтверждённой совместимости с архитектурой, платформами и остальным dependency graph.

**Версии из NSH не являются ограничением, источником истины или рекомендуемым baseline для SolidJS Native.**

Из NSH мы заимствуем архитектурные идеи и проверенные инженерные решения. Мы независимо выбираем:
- версии Rust crates;
- версии embedded JavaScript engine и Rust bindings;
- версии npm packages;
- toolchain и build tools;
- platform SDKs;
- graphics, layout, text и accessibility dependencies.

Нельзя понижать версию только потому, что её использует NSH. Нельзя повышать версию только ради большей цифры, если это нарушает compatibility, security или целевую поддержку платформ.

## 2. Что означает latest stable

Для выбора зависимости:

1. Проверить официальный registry/release source на дату изменения.
2. Выбрать последний non-prerelease stable release.
3. Проверить поддерживаемые Rust/Node/platform toolchains.
4. Проверить peer dependencies, feature flags, platform constraints и license.
5. Проверить security advisories и known regressions.
6. Проверить влияние на runtime/ABI/protocol/compiler output.
7. Запустить чистую сборку и subsystem tests на поддерживаемых target environments.
8. Зафиксировать выбранную версию и причину в lockfile и, при необходимости, dependency decision record.

Stable означает release channel, который maintainer объявил стабильным. Alpha, beta, rc, next, canary, nightly, experimental и Git main/master не являются default choices.

Исключение допускается только при документированном техническом блокере или необходимой функции. Такое исключение должно содержать:
- причину отказа от последней стабильной версии;
- известное ограничение/регрессию;
- upstream issue или evidence;
- тест/прототип, подтверждающий выбор;
- условие и критерий пересмотра.

## 3. Compatibility over version number

Не нужно обновлять связанные пакеты отдельно, если существует совместимая группа:

- SolidJS version + Universal renderer contract + Solid compiler plugin.
- Vite version + vite-plugin-solid + Vite config/build mode.
- QuickJS-NG source version + rquickjs/rquickjs-sys binding layer + runtime behavior.
- Rust toolchain + Rust crates + target-specific native dependencies.
- Platform SDK + platform crate + native ABI.

Major versions нельзя автоматически считать совместимыми. Каждая major upgrade проходит smoke, typecheck, unit, integration и target-specific checks.

Для compiler/runtime pair дополнительно сохранять fixture bundle, собранный установленной версией compiler, и запускать его в embedded runtime. Обновление компилятора без такой проверки не допускается.

## 4. Reproducible builds

Последние стабильные версии выбираются при обновлении, но ежедневная сборка должна быть воспроизводимой.

Правила:

- Прямые зависимости фиксируются в manifest и lockfile.
- Cargo.lock обязателен для приложения, CLI, примеров и workspace build.
- pnpm-lock.yaml обязателен и проверяется через pnpm install --frozen-lockfile.
- Toolchain versions задаются проектными config files.
- Development toolchain и MSRV — разные контракты: `rust-toolchain.toml` pins Rust 1.99.1, while Cargo `rust-version` states the minimum supported compiler.
- MSRV должен быть не ниже самого высокого MSRV среди обязательных dependencies; текущий минимум проекта поднят до 1.87 из-за rquickjs.
- CI использует pinned development toolchain и отдельную MSRV job, когда она настроена.
- Не использовать плавающие Git branches в production dependencies.
- Проверять checksum/integrity, registry provenance и license metadata.
- Все native SDK prerequisites фиксируются в platform setup docs.
- Lockfile не редактируется вручную.

Для библиотечных crates publishable API может иметь semver range, но workspace CI и release artifacts всё равно должны проверяться с зафиксированным dependency graph.

## 5. Update cadence

Обновления проверяются:
- перед началом каждого большого milestone;
- перед добавлением нового subsystem dependency;
- при security advisory;
- при исправлении нужного upstream bug;
- регулярно, не реже одного планового dependency review за месяц в период активной разработки.

Patch/minor updates обычно объединяются в один dependency maintenance change. Major upgrades выполняются отдельным изменением с compatibility review.

Не обновлять весь граф во время unrelated feature implementation. Если upgrade меняет compiler output, runtime ABI, C/C++ build, native SDK or platform behavior, он должен иметь отдельные проверки.

## 6. Dependency acceptance checklist

Для каждой новой зависимости:

- [ ] Действительно нужна; функциональность не следует реализовать меньшим количеством кода без потери качества.
- [ ] Последняя стабильная версия проверена в официальном registry/release.
- [ ] Нет более подходящей maintained альтернативы.
- [ ] License совместима с проектом и будущим распространением.
- [ ] Security advisories reviewed.
- [ ] MSRV/Node/OS/CPU requirements подходят целевым платформам.
- [ ] Dependency tree, binary size and compile cost оценены.
- [ ] Feature flags включены минимально необходимым набором.
- [ ] Не добавляет лишнюю runtime/async/threading модель.
- [ ] Документированы ограничения и reason for adoption.
- [ ] Есть тест, который доказывает правильность интеграции.

Особое внимание для embedded/runtime dependencies:
- ownership и GC callbacks;
- thread safety / Send / Sync assumptions;
- scheduler and event-loop semantics;
- interrupt/cancellation support;
- cross-compilation;
- static/dynamic linking;
- mobile targets;
- security boundary and memory limits.

## 7. Baseline as of 2026-11-10

Эта таблица — проверенный baseline для текущего проекта и ближайших работ. Она не заменяет проверку совместимости при фактическом внедрении каждого будущего subsystem.

| Component | Stable baseline checked | Current project state | Decision |
|---|---:|---|---|
| Rust development toolchain | 1.99.1 | `rust-toolchain.toml` pins 1.99.1; installed locally | Adopted; workspace tests, Clippy and rustfmt pass |
| Rust MSRV | 1.87 | `Cargo.toml` declares 1.87; rquickjs 0.12 raised its MSRV to 1.87 | Declared to match the required runtime binding; add a dedicated 1.87 CI job before claiming MSRV verification |
| Node.js | 26.13.0 Active LTS | Local machine 24.22.1; Vite 8 supports Node `^20.19.0` or `>=22.12.0` | Node 24.22.1 remains valid for current development; add Node 26.13.0 to CI before making it the sole required version |
| pnpm | 12.18.0 | Project pins 12.18.0; lockfile metadata updated | Adopted and verified with frozen-lockfile installation |
| solid-js | 1.9.18 | Root dependency pinned to 1.9.18; solid-native peer range starts at 1.9.18 | Adopted; keep Solid 2 prereleases out of stable builds |
| TypeScript | 7.0.2 | Project pins 7.0.2; TypeScript 7 config migration removed `baseUrl` and made paths relative | Adopted; `pnpm typecheck` passes |
| Vite | 8.3.7 | Project pins 8.3.7; lockfile resolves 8.3.7 | Adopted; build and embedded smoke pass |
| vite-plugin-solid | 2.11.15 | Project pins 2.11.15; peer range accepts Vite 8 and Solid 1.9 | Adopted; Vite 8 build and QuickJS smoke pass |
| @types/node | 26.6.6 | Project pins 26.6.6 | Adopted; current TypeScript check passes on local Node 24.22.1 |
| rquickjs | 0.14.0 | Cargo workspace pins exactly `=0.14.0`; lockfile resolves 0.14.0 | Latest stable binding checked; exact pin adopted |
| thiserror | 2.0.21 | Cargo workspace pins exactly `=2.0.21`; lockfile resolves 2.0.21 | Latest stable version checked; exact pin adopted |
| serde | 1.0.229 | Transitive serde_core at 1.0.229; not a direct dependency yet | Select current stable when M1 protocol implementation adds it |
| Taffy | 0.15.1 reported by current docs.rs index | Not a dependency yet | Recheck official crates.io and test its low-level API when M3 starts |
| QuickJS-NG source | upstream 0.18.0 | rquickjs 0.14.0 changelog identifies bundled QuickJS-NG 0.16.2 | Explicit compatibility investigation required; see section 8 |

Versions reflect registry/release information checked on 2026-11-10. They are not claims that all listed versions have already passed this project's build. A future dependency is not adopted until its milestone's integration prototype is green.

## 8. QuickJS-NG and rquickjs version split

Do not treat rquickjs version and upstream QuickJS-NG version as the same version.

As of this baseline:
- rquickjs 0.14.0 is the latest stable Rust binding found.
- Its changelog says its 0.13.0 release updated bundled sys/quickjs to QuickJS-NG commit 2c620e4, identified there as QuickJS-NG 0.16.2.
- Upstream QuickJS-NG has since released 0.18.0.

Therefore, upgrading rquickjs to its latest crate release does not automatically provide the latest upstream QuickJS-NG source.

Before M2 runtime hardening:
1. Audit rquickjs-sys bundled source and exact upstream commit.
2. Compare QuickJS-NG 0.16.2 and 0.18.0 for fixes that affect our runtime.
3. Inspect open rquickjs upstream work/issues for 0.18 integration.
4. Build a small integration branch with the latest compatible binding/source combination.
5. Run rquickjs tests, SolidJS runtime compatibility tests, Windows MSVC build and cross-target builds.
6. Prefer upstream rquickjs integration if it becomes available.
7. If no suitable stable binding exists and the newer engine is required, evaluate a minimal maintained fork/patch of rquickjs-sys; do not hand-edit vendored engine sources in the application crate.

The selection must be based on compatibility and security evidence, not solely on matching upstream version numbers. Bytecode artifacts must never be assumed compatible across engine versions.

## 9. Stable versus experimental technology

Project default:
- stable Rust toolchain;
- stable npm releases;
- stable crates.io releases;
- stable platform SDK/API versions.

Experimental releases may be tested in isolated branches or benchmarks. They must not become default production dependencies without an explicit ADR and a clear migration/rollback plan.

For a young dependency ecosystem, newest stable may still be immature. Adoption should consider maintenance, issue response, release cadence, ecosystem usage and platform coverage — not just publication date.

## 10. Automation

Add or maintain the following checks as the repository grows:

- dependency update PRs grouped by ecosystem;
- CI install from frozen lockfiles;
- license and provenance inventory;
- security advisory scanning;
- MSRV/toolchain check;
- platform target matrix;
- dependency graph and binary-size report for native releases;
- compiler/runtime compatibility fixtures.

Automated update bots may propose version bumps but cannot merge major updates without required tests and review.
