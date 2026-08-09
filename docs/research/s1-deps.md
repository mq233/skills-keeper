# S1 Rust 依赖选型核查（frontmatter 解析 / 文件 hash / SQLite）

> 对应 issue：wayfinder #18「S1 Rust 依赖选型核查（frontmatter 解析 / 文件 hash / SQLite）」（S1 库矩阵只读实施规划 #17 的子任务）
> 调研日期：2026-08-09。所有结论均来自一手来源（crates.io API、docs.rs、GitHub 仓库 README/releases、RustSec advisory-db、本地工具链与依赖树），每条结论标注来源编号；无法证实的内容明确标注「未证实」。

---

## 来源链接

| 编号 | 来源 | URL |
| --- | --- | --- |
| S1 | crates.io API：serde_yaml | https://crates.io/api/v1/crates/serde_yaml |
| S2 | dtolnay/serde-yaml GitHub 仓库（README 归档声明） | https://github.com/dtolnay/serde-yaml |
| S3 | crates.io API：serde_yml | https://crates.io/api/v1/crates/serde_yml |
| S4 | crates.io API：noyalib | https://crates.io/api/v1/crates/noyalib |
| S5 | sebastienrousseau/noyalib GitHub README | https://github.com/sebastienrousseau/noyalib |
| S6 | RustSec advisory：RUSTSEC-2025-0068（serde_yml） | https://rustsec.org/advisories/RUSTSEC-2025-0068 |
| S7 | RustSec advisory：RUSTSEC-2025-0067（libyml） | https://rustsec.org/advisories/RUSTSEC-2025-0067 |
| S8 | crates.io API：serde_yaml_ng | https://crates.io/api/v1/crates/serde_yaml_ng |
| S9 | acatton/serde-yaml-ng GitHub README | https://github.com/acatton/serde-yaml-ng |
| S10 | crates.io API：serde_norway | https://crates.io/api/v1/crates/serde_norway |
| S11 | crates.io API：serde_yaml2 | https://crates.io/api/v1/crates/serde_yaml2 |
| S12 | crates.io API：yaml-peg | https://crates.io/api/v1/crates/yaml-peg |
| S13 | crates.io API：yaml-rust2 | https://crates.io/api/v1/crates/yaml-rust2 |
| S14 | Ethiraric/yaml-rust2 GitHub README | https://github.com/Ethiraric/yaml-rust2 |
| S15 | docs.rs：yaml-rust2 0.11.0（features/MSRV） | https://docs.rs/yaml-rust2/latest/yaml_rust2/ |
| S16 | crates.io API：sha2 | https://crates.io/api/v1/crates/sha2 |
| S17 | crates.io API：blake3 | https://crates.io/api/v1/crates/blake3 |
| S18 | BLAKE3-team/BLAKE3 GitHub README | https://github.com/BLAKE3-team/BLAKE3 |
| S19 | BLAKE3-team/BLAKE3 GitHub releases | https://github.com/BLAKE3-team/BLAKE3/releases |
| S20 | crates.io API：xxhash-rust | https://crates.io/api/v1/crates/xxhash-rust |
| S21 | DoumanAsh/xxhash-rust GitHub README | https://github.com/DoumanAsh/xxhash-rust |
| S22 | Cyan4973/xxHash GitHub README（官方） | https://github.com/Cyan4973/xxHash |
| S23 | crates.io API：rusqlite | https://crates.io/api/v1/crates/rusqlite |
| S24 | rusqlite/rusqlite GitHub README | https://github.com/rusqlite/rusqlite |
| S25 | rusqlite GitHub Cargo.toml（master） | https://raw.githubusercontent.com/rusqlite/rusqlite/master/Cargo.toml |
| S26 | crates.io API：refinery | https://crates.io/api/v1/crates/refinery |
| S27 | crates.io API：tauri | https://crates.io/api/v1/crates/tauri |
| S28 | 本地 `cargo tree`（E:\workspace\skills-keeper\src-tauri，2026-08-09） | —（本机执行） |
| S29 | 本地 `rustc --version` / `cargo --version` | —（本机执行，rustc 1.97.1，2026-07-14） |
| S30 | RustSec advisory-db GitHub API（crates/serde-yaml、crates/serde-yaml-ng 目录均 404） | https://api.github.com/repos/rustsec/advisory-db/contents/crates/serde-yaml |
| S31 | tauri-cli Cargo.toml（dev 分支） | https://raw.githubusercontent.com/tauri-apps/tauri/dev/crates/tauri-cli/Cargo.toml |

---

## 一、YAML frontmatter 解析选型

### 1.1 候选全景（crates.io API 数据，2026-08-09 抓取）

| crate | 最新版 | 发布时间 | 总下载 | 90 天下载 | serde derive | 维护状态 |
| --- | --- | --- | --- | --- | --- | --- |
| serde_yaml | 0.9.34+deprecated | 2024-03-25 | 3.63 亿 | 8680 万 | ✓ | 已弃用（S1、S2） |
| serde_yml | 0.0.13 | 2026-05-27 | 2100 万 | 655 万 | ✓ | 已弃用 + RUSTSEC unsound（S3、S6） |
| serde_yaml_ng | 0.10.0 | 2024-05-26 | 806 万 | 444 万 | ✓ | 维护慢但存活（S8、S9） |
| serde_norway | 0.9.42 | 2024-12-21 | 885 万 | 219 万 | ✓ | crates.io 停更约 1.5 年（S10） |
| serde_yaml2 | 0.1.3 | 2025-05-12 | 175 万 | 52 万 | ✓ | 0.1.x，低活跃（S11） |
| yaml-peg | 1.0.9 | 2025-08-28 | 7.8 万 | 1.7 万 | ✓ | 小众（S12） |
| yaml-rust2 | 0.11.0 | 2025-12-16 | 5009 万 | 1341 万 | ✗（无 serde feature） | 活跃，基础维护模式（S13、S14、S15） |
| noyalib | 0.0.18 | 2026-07-31 | 18.6 万 | 18.6 万 | ✓ | 活跃但 0.0.x（S4、S5） |

### 1.2 serde_yaml（原版）：弃用铁证，新代码不应引入

- crates.io 最新版本号即 `0.9.34+deprecated`，发布于 2024-03-25（S1）。
- GitHub 仓库于 **2024-03-25 归档**，README 标注「This repository was archived by the owner on Mar 25, 2024. It is now read-only」与「_(This project is no longer maintained.)_」（S2）。
- 存量巨大（3.63 亿总下载、90 天 8680 万，S1）——但那是历史存量；作为「活依赖」它已停止修复与新特性，本项目新引入没有理由选它。

> 注：任务背景中「serde_yaml 于 2024 年 12 月被弃用」的说法未在一手来源中证实——crates.io 与 GitHub 的一手信息均为 **2024-03-25** 归档/标记弃用（S1、S2）。

### 1.3 serde_yml：已弃用且被 RUSTSEC 标记 unsound，必须排除

- crates.io 页面 description 现以「DEPRECATED — `serde_yml` is unmaintained.」开头；0.0.13（2026-05-27）是转发到纯 Rust 库 `noyalib` 的兼容薄层，官方指引迁移到 noyalib（S3、S5）。
- **RUSTSEC-2025-0068**（2025-09-12 发布，无补丁版本）：`serde_yml::ser::Serializer.emitter` 可触发段错误（unsound），crate 同时标记 unmaintained；其底层 `libyml` 同批被 **RUSTSEC-2025-0067** 标记 unsound + unmaintained（S6、S7）。
- RustSec 官方指引：迁移到 **serde_norway 或 serde_yaml_ng**（maintained forks），或纯 Rust 替代 serde_yaml2 / yaml-peg（S6）。

### 1.4 serde_yaml_ng：推荐主选

- dtolnay 原版 serde-yaml 的**独立延续 fork**（直接 fork 自原版提交），README 明确目标「compatible as much as possible with David Tolnay's original library」，API 与 serde_yaml 0.9 一致（`from_str`/`to_string`），支持 serde derive（S9）。
- crates.io 最新 0.10.0（2024-05-26），总下载 806 万、90 天 444 万——四个活跃候选（serde_yaml_ng / serde_norway / serde_yaml2 / noyalib）中下载量最大（S8）。
- 是 RustSec-2025-0068 官方指引点名的「maintained fork」之一（S6）。
- **已知风险（如实记录）**：
  - 只支持 YAML 1.1（S9）——对本项目三字段标量 frontmatter 足够；
  - crates.io 发版已停滞 2 年（2024-05-26 后无新版本，S8）；GitHub 侧 2026 年仍在处理 issue/PR、计划把底层 unsafe-libyaml 迁移到 libyaml-safer（S9）；
  - 作者自述「a library for myself」，不承诺专业级支持（S9）；
  - RustSec advisory-db 中无该 crate 的 advisory 目录（GitHub API 404，S30）——即目前无已收录安全公告（不排除未来收录）。

### 1.5 其他候选简评

- **serde_norway**（cafkafk 维护，RustSec 点名的另一 fork）：0.9.42（2024-12-21）后 crates.io 无更新，约 1.5 年停更（S10）——与 serde_yaml_ng 相比无优势，且 RustSec 点名顺序中 serde_yaml_ng 同样在列（S6）。
- **serde_yaml2**（yaml-rust2 的 serde 层）：0.1.3（2025-05-12），版本号 0.1.x 说明 API 未稳定；但其底层 yaml-rust2 是活跃的（2025-12-16 发版，S13）——可作为 serde_yaml_ng 万一停摆时的备选线（S11、S13）。
- **yaml-peg**：下载量 7.8 万、单人小众项目（S12），无理由选择。
- **yaml-rust2**：YAML 1.2 全兼容、通过官方测试套件（S14），MSRV 1.65（S15），下载量大且活跃；但**无 serde 集成**（features 只有 encoding/debug_prints，S15），需要手写 Value→结构体转换；且维护模式为「仅基础维护、新功能转移给 saphyr」（S14）。对本项目可用但不如 serde_yaml_ng 省事。
- **noyalib**（serde_yml 作者的新库）：纯 Rust、零 unsafe、完整 serde 集成、YAML 1.2（官方测试套件 406/406）、流式优先、MSRV 1.86，0.0.18（2026-07-31）活跃发布中，serde_yml 的 0.0.13 已把用户引向它（S3、S4、S5）。**但版本 0.0.x（API 未稳定）、全部历史只有 88 个提交、作者即被 RUSTSEC 标记 unsound 的 serde_yml 的同一维护者**（S5、S6）——2026-08 时点不建议作为生产依赖引入，值得跟踪观察（详见第六节）。
- **手写解析**（针对三字段最小集）：不推荐手写 YAML 解析器。真实 SKILL.md 的 frontmatter 可能出现引号、块标量（`description: |`）、注释、未知字段（导入时工具特有字段并存，见技术规划 §3.2），YAML 语法面广——noyalib 过官方测试套件需 406 个用例（S5），手写解析的正确性风险与测试成本都不划算。「frontmatter 块提取（首尾 `---` 分隔行扫描）」可手写，但块内 YAML 必须交给库解析。

### 1.6 结论：采用 serde_yaml_ng，并采用「Value + 手写三字段提取」容错写法

**推荐：`serde_yaml_ng = "0.10"`。** 理由：

1. API 与 serde_yaml 0.9 完全一致（S9），迁移/参考成本为零，社区既有示例直接可用；
2. RustSec-2025-0068 官方指引点名的 maintained fork（S6）；
3. 候选集中下载量与生态使用量最大（S8），且无已收录安全公告（S30）；
4. 本项目场景是「用户本地自己的文件 + 三字段最小集」，YAML 1.1 覆盖足够、宽松解析行为（未知字段忽略）正合容错需求（S9）。

**容错设计建议（工程建议，非一手来源结论）**：不要直接 `from_str::<SkillFrontmatter>` 反序列化到 derive 结构体，而是先解析为 `serde_yaml_ng::Value`，再手动提取三个字段、标量统一转字符串。原因：YAML 1.1 的标量类型推断会把 `version: 1.0` 解析为 float、`no`/`yes` 解析为 bool，直接反序列化为 `String` 会报类型错误；`Value` 路径天然忽略未知字段（导入场景工具特有字段并存，技术规划 §3.2），三字段提取约 20-30 行即可，行为完全可控。

---

## 二、文件与目录 hash 选型

### 2.1 候选对比（crates.io API 数据，2026-08-09 抓取）

| crate | 最新版 | 发布时间 | 总下载 | 90 天下载 | 密码学安全 | 维护方 |
| --- | --- | --- | --- | --- | --- | --- |
| sha2 | 0.11.0 | 2026-03-25 | 8.21 亿 | 2.18 亿 | ✓ | RustCrypto 团队（S16） |
| blake3 | 1.8.6 | 2026-08-05 | 1.61 亿 | 3730 万 | ✓ | BLAKE3-team（S17） |
| xxhash-rust | 0.8.18 | 2026-07-21 | 8888 万 | 2484 万 | ✗ | DoumanAsh 单人（S20） |

### 2.2 各自要点

- **sha2**：RustCrypto 官方、纯 Rust、生态最广（8.21 亿下载，S16）。软件实现速度低于 blake3（BLAKE3 官方声称远快于 SHA-2，S18）。注意：**Tauri 依赖树已自带 sha2 0.10.9**（由 tauri-codegen 2.6.3 引入，S28）——若本项目直接依赖 sha2 0.11 会与树内 0.10 双版本共存；想复用现有版本需指定 `"0.10"`。
- **blake3**：密码学安全哈希（Merkle 树结构，可多线程/多核并行，x86 自动 SIMD：SSE2/SSE4.1/AVX2/AVX-512，另有 NEON/WASM），BLAKE3 官方自称「Much faster than MD5, SHA-1, SHA-2, SHA-3, and BLAKE2」（S18）。1.8.6 于 2026-08-05 发布（一周前）；MSRV 1.85（2024 edition，S19）；依赖 digest 0.11（1.8.4 起，S19）。许可证 CC0-1.0 OR Apache-2.0（S17）。`Hasher::update` 支持流式/增量喂数据（S18），适合目录级 hash 聚合。
- **xxhash-rust**：DoumanAsh 单人维护，提供 xxh32/xxh64/xxh3（128-bit 变体 XXH128）（S21）。xxHash 官方项目定位为「Extremely fast **non-cryptographic** hash algorithm」（S22）——非密码学哈希。速度极快，但不提供抗碰撞/抗攻击保证。

### 2.3 结论：采用 blake3

**推荐：`blake3 = "1"`。** 理由：

1. **场景匹配**：S1 扫描器的 v/t/r 三方 hash 是内容一致性比对（技术规划 §4.3），非密码学对抗场景，但要「低误判」——blake3 是密码学安全哈希，碰撞误判概率趋零（S18），同时速度远超 SHA-2（S18）；
2. **性能**：目录扫描是高频操作（每次扫描全量 hash），blake3 的 SIMD 自动检测 + 多线程并行在桌面机上是吞吐量最优选（S18）；
3. **目录 hash 聚合天然适配**：`Hasher::update` 流式依次喂入每个文件的 rel_path + 内容即可得 Skill 目录整体 hash（S18 增量特性），无需自建组合算法（工程建议）；
4. **维护健康**：1.8.6 一周前刚发布、官方团队维护（S17、S19）；
5. **兼容性**：MSRV 1.85 < 本项目 Rust 1.97.1（S19、S29）；Tauri 依赖树中无 blake3，无版本冲突（S28）。

**sha2 为保守备选**：若希望新增依赖面最小，可指定 `sha2 = "0.10"` 与 Tauri 自带版本共用（零新增编译量，S28），但换取的是明显更慢的软件哈希。**xxhash-rust 不推荐**：非密码学（S22）+ 单人维护（S20），在本项目「低误判优先」的取向下没有理由为它放弃 blake3 的兼得优势。

---

## 三、rusqlite 与 schema 迁移

### 3.1 版本与兼容性

- crates.io 当前最新 **0.40.2**（2026-08-08 发布，S23）；GitHub master 同步到 0.40.1 + libsqlite3-sys 0.38（README/Cargo.toml 滞后于 crates.io 一个小补丁，正常节奏，S24、S25）。
- **MSRV 政策**：README 明示「Latest stable Rust version at the time of release」（S24）——即跟随 Rust stable。edition 2024（S25）要求 Rust ≥ 1.85；本项目本地工具链 1.97.1（2026-07-14，S29），完全兼容。
- **bundled 特性**：编译内嵌 SQLite 源码（当前内置 **SQLite 3.53.2**），不依赖系统 SQLite；bundled 同时隐含启用 modern_sqlite；base 包支持 SQLite ≥ 3.34.1（S24）。桌面应用用 bundled 是官方推荐做法（S24），与决议 #8「rusqlite（bundled）」一致。
- **与 Tauri 无冲突**：Tauri 2.11.5（2026-07-01，S27）依赖树中不含 rusqlite（S28）。

### 3.2 迁移方案：手写版本表（维持规划 §4.1 设计），不引入 refinery

- 技术规划 §4.1 的 `db/migrations.rs` 本就是「版本表 + 递增迁移」的手写设计；§3.5 的 schema 只有三张表、S1-S6 期间基本定型。
- 候选 crate **refinery**：0.9.2（2026-06-10，活跃），总下载 928 万（S26）——功能完整（embed_migrations! 编译期嵌入 SQL、版本追踪、rusqlite 支持），但对三表 schema 属于杀鸡用牛刀：引入宏嵌入、rusqlite feature 版本耦合、迁移目录管理等复杂度。
- **结论：维持手写**——`PRAGMA user_version`（或独立 schema_version 表）+ 递增迁移函数数组，约 50 行代码、零额外依赖、便于 cargo test 单测先行（技术规划 §7 测试策略）。若未来迁移数量显著增长（如 S9 后续能力带来新表），再评估 refinery 0.9.x（S26）。

---

## 四、Tauri 2 依赖树交叉检查

- 本地 `cargo tree`（tauri 2.11.5 + tauri-plugin-opener 2.5.4，S28）：依赖树中与本课题相关的 crate **只有 `sha2 v0.10.9`**（由 tauri-codegen 2.6.3 → tauri-macros 2.6.3 引入）。**无** rusqlite、blake3、xxhash-rust、任何 YAML crate、refinery。
- 结论：S1 新增依赖（rusqlite / serde_yaml_ng / blake3）均不会与 Tauri 产生版本冲突；唯一交叉点是 sha2——若引入 sha2 建议用 `"0.10"` 与树内版本复用，避免 0.10/0.11 双版本（S28）。
- 生态旁证：Tauri 官方 CLI（tauri-cli）自身也不依赖任何 YAML crate（配置走 TOML/JSON，S31）——YAML 在 Tauri 生态内没有既定先例可循，本选型独立成立。

---

## 五、S1 推荐依赖清单（最终）

| crate | 版本策略 | 用途 | 理由 |
| --- | --- | --- | --- |
| `rusqlite` | `"0.40"`，features = `["bundled"]` | SQLite 三表 + 手写迁移 | 决议 #8 已定；0.40.2（2026-08-08）；bundled 内置 SQLite 3.53.2；MSRV 政策=发布时 stable，1.97.1 兼容（S23、S24、S25） |
| `serde_yaml_ng` | `"0.10"` | SKILL.md frontmatter 解析（三字段最小集 + 容错） | API 与 serde_yaml 0.9 一致；RustSec-2025-0068 官方指引点名；宽松解析容错；无已收录安全公告（S6、S8、S9、S30） |
| `blake3` | `"1"` | 文件内容 hash + Skill 目录整体 hash（v/t/r 三方比对） | 密码学安全 + 最快（SIMD/多线程）；流式 update 天然聚合目录 hash；官方维护（1.8.6 2026-08-05）；MSRV 1.85 < 1.97.1；与 Tauri 树无冲突（S17、S18、S19、S28、S29） |
| （不新增）手写迁移 | `PRAGMA user_version` + 递增迁移数组 | db/migrations.rs 版本表 | 三表小 schema；零额外依赖；规划 §4.1 既有设计；refinery 复杂度用不上（S26） |

**备选（不默认引入）**：

- `sha2 = "0.10"`：若想最小化新增依赖面，与 Tauri 自带版本共用（S28）；代价是显著更慢的软件哈希（S18）。
- `serde_yaml2 = "0.1"`：serde_yaml_ng 万一停摆时的迁移备选线（底层 yaml-rust2 活跃，2025-12-16 发版，S11、S13）。
- `noyalib`：暂不引入（0.0.x 未稳定 + 作者历史口碑风险，S4、S5、S6）；建议跟踪，其活跃度与质量值得观望。

---

## 六、未证实与风险标注

- **「serde_yaml 于 2024 年 12 月弃用」未证实**：任务背景如此描述，但 crates.io（0.9.34+deprecated，2024-03-25）与 GitHub（2024-03-25 归档）的一手信息均为 2024 年 3 月（S1、S2）。以一手来源为准。
- **serde_yaml_ng 的 GitHub 活跃度**：页面显示 2026 年仍在处理 issue/PR、计划迁移 libyaml-safer，但抓取内容无具体最近提交日期，属推断；其 crates.io 发版停留 2024-05-26 为硬事实（S8、S9）。若 S1 开发期间发现需要其修复的解析 bug，可能无人及时发版——本项目解析面极小，可接受（未证实此项风险的具体影响）。
- **yaml-rust2 的 GitHub 最近提交日期未证实**（S14 页面未显示具体日期；crates.io 0.11.0 发布于 2025-12-16 为硬事实，S13）。
- **RustSec advisory-db 无 serde_yaml / serde_yaml_ng 条目**（S30 两者 GitHub API 均 404）——仅说明目前无已收录公告，不排除未来收录。
- **noyalib 作者历史**：作者 sebastienrousseau 即被 RUSTSEC-2025-0068 标记 unsound 的 serde_yml 的维护者（S4、S6）；noyalib 自身是否经受住审计（cargo vet/Miri 为其自述流程）尚未经社区时间检验（S5）。
- **下载量为 crates.io 统计口径**（总下载含历史存量），90 天下载量更能反映当前使用趋势（各来源编号见对比表）。
