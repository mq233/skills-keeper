# CI 三平台构建依赖调研（Tauri 2 × GitHub Actions）

> 对应 issue：wayfinder #13「阶段 0 工程骨架」
> 调研日期：2026-08-09。所有结论均来自官方文档与官方仓库 README 等一手来源，每条结论标注来源；无法证实的内容明确标注「未证实」。

---

## 来源链接

| 编号 | 来源 | URL |
| --- | --- | --- |
| S1 | Tauri 官方文档：前置条件（Prerequisites） | https://tauri.app/start/prerequisites/ |
| S2 | Tauri v2 官方文档：GitHub Actions 分发流水线 | https://v2.tauri.app/distribute/pipelines/github/ |
| S3 | tauri-apps/tauri-action GitHub 仓库 README | https://github.com/tauri-apps/tauri-action |
| S4 | Swatinem/rust-cache GitHub 仓库 README | https://github.com/Swatinem/rust-cache |
| S5 | pnpm/action-setup GitHub 仓库 README | https://github.com/pnpm/action-setup |
| S6 | Tauri 2.0.0-alpha.3 发布说明（webkit2gtk-4.1 迁移） | https://v2.tauri.app/blog/tauri-2-0-0-alpha-3/ |
| S7 | github/roadmap#980：Ubuntu 24 on GitHub-hosted runners (GA) | https://github.com/github/roadmap/issues/980 |

---

## 一、三平台依赖清单

### 1. Linux（Ubuntu）

**结论：需要显式安装 apt 包，tauri-action 不会自动处理。** tauri-action README 与官方 GitHub Actions 指南的 workflow 示例中都包含一个独立的「install dependencies (ubuntu only)」步骤（S2、S3）。

**Tauri v2 的 WebKitGTK 版本要求**：Tauri v2 自 2.0.0-alpha.3 起从 WebKit2GTK 4.0 迁移到 **4.1**，包名必须用 `libwebkit2gtk-4.1-dev`（4.0 与 4.1 的底层 soup 库不同：4.0 用 soup2，4.1 用 soup3；迁移背景是支持 Flatpak 与修复 soup2 的 bug）（S6）。v1 时代的 `libwebkit2gtk-4.0-dev` 不可用于 Tauri v2。

**官方 workflow 使用的 apt 包（最小构建集）**（S2、S3，两个来源的清单完全一致）：

```
libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf xdg-utils
```

**Prerequisites 页面的完整开发依赖集（Debian/Ubuntu 章节）**（S1）：

```
libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev \
libayatana-appindicator3-dev librsvg2-dev
```

两个来源对 appindicator 包的命名不同：官方 workflow 用 `libappindicator3-dev`，prerequisites 页面用 `libayatana-appindicator3-dev`。在 Ubuntu 22.04+ 上二者兼容（appindicator 库已由 ayatana 维护者接替，官方 workflow 示例在 ubuntu-22.04 上原样可用）（S1、S2）。CI 中按官方 workflow 原样照抄即可。

**基线系统选择**：官方 GitHub Actions 指南示例固定使用 **`ubuntu-22.04`**（而非 `ubuntu-latest`），并在矩阵条件中写死 `matrix.platform == 'ubuntu-22.04'`（S2）。原因：Ubuntu 22.04 与 Debian 12 是「提供 libwebkit2gtk-4.1-dev 的最老基线」，在更新的系统上构建会抬高产物要求的 glibc 版本，导致在旧系统上运行报 `GLIBC_2.33 not found` 之类的错误；Tauri 官方建议在打算支持的最老基线系统上构建（S2、S6 引述的分发包页面结论）。

`ubuntu-latest` 是浮动标签：GitHub 自 2024 年 9 月起将 `ubuntu-latest` 从 22.04 渐进切换到 24.04，并可能再次变更（S7）。上述 apt 包在 24.04 的官方源中同样存在（libwebkit2gtk-4.1-dev 自 22.04 起进入 Ubuntu 仓库），但「验证可构建」类 CI 建议固定 `ubuntu-22.04` 与官方基线保持一致，避免浮动标签引发意外（S2、S7）。

**可直接复制的 Linux 依赖安装步骤 YAML**（官方 workflow 原样，S2）：

```yaml
- name: install dependencies (ubuntu only)
  if: matrix.platform == 'ubuntu-22.04' || matrix.platform == 'ubuntu-22.04-arm'
  run: |
    sudo apt-get update
    sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf xdg-utils
```

### 2. macOS

**结论：GitHub `macos-latest` runner 自带 Xcode，无需显式处理；无额外 brew 依赖。**

- 官方文档原文："Tauri uses Xcode and various macOS and iOS development dependencies."（S1）。仅构建桌面端时不必装完整 Xcode，安装 Command Line Tools 即可（命令 `xcode-select --install`），完整 Xcode 只在需要 iOS 目标时必需（S1）。
- 官方 GitHub Actions workflow 中 macOS 条目（`macos-latest`）**没有任何额外的系统依赖步骤**——runner 镜像已预装 Xcode/CLT，构建所需的 SDK 开箱即用（S2）。
- 无需额外 brew 依赖：Homebrew 与 CocoaPods 只出现在 iOS/移动端构建的前置条件中（S1）。
- 唯一显式步骤是 rustup targets：在 macOS runner 上为 M1（`aarch64-apple-darwin`）与 Intel（`x86_64-apple-darwin`）分别加 target（S2）：
  ```yaml
  - name: install Rust stable
    uses: dtolnay/rust-toolchain@stable
    with:
      targets: ${{ matrix.platform == 'macos-latest' && 'aarch64-apple-darwin,x86_64-apple-darwin' || '' }}
  ```
- 签名（可选）：仅「验证可构建」场景不需要签名；若产物要分发，无证书时官方建议配置 ad-hoc signing identity，可避免 Apple Silicon 构建包被系统标记为损坏（S2）。

### 3. Windows

**结论：GitHub `windows-latest` runner 自带 MSVC 构建工具与 WebView2，无需显式处理。**

- 本地开发前置条件：官方文档原文 "Tauri uses the Microsoft C++ Build Tools for development as well as Microsoft Edge WebView2"——需安装 C++ Build Tools（勾选 "Desktop development with C++"）；WebView2 在 Windows 10（1803 起）及更新系统上已预装（S1）。
- 官方 GitHub Actions workflow 中 Windows 条目（`windows-latest`）同样**没有任何额外步骤**——runner 镜像预装 MSVC 工具链与 WebView2（S2）。
- Rust 侧注意：Windows 上默认工具链应为 MSVC（`rustup default stable-msvc`）（S1）；`dtolnay/rust-toolchain@stable` 在 Windows runner 上会默认选择 MSVC 工具链（S2 示例未做额外处理）。
- 打包注意（仅 `msi`/`all` 目标）：MSI 打包需要 Windows 可选功能 **VBSCRIPT**，缺少时 `tauri build` 报 "failed to run light.exe"；该功能正被弃用（S1）。仅验证可构建（默认不打包或只打 nsis）时不受影响。

### 4. 通用前置（Rust / Node）

- **Rust**：三平台均需 Rust（S1）。CI 中官方推荐 `dtolnay/rust-toolchain@stable`（S2），并在其后紧跟 `Swatinem/rust-cache@v2`（顺序不可颠倒，原因见第三节）。
- **Node.js**：仅使用 JS 前端框架时需要，官方建议 LTS（`node-version: lts/*`）（S1、S2）。本仓库前端为 Vue + Vite，属于需要 Node 的情形。

---

## 二、tauri-action vs 手写 `cargo tauri build`（仅验证可构建场景）

「仅验证可构建」= 不签名、不创建 release、不上传资产的 CI 校验（PR 或 push 时三平台各编一遍）。两种做法对比：

| 维度 | `tauri-apps/tauri-action@v1` | 手写 `cargo tauri build` |
| --- | --- | --- |
| 构建速度 | 无实质差异——最终都执行 `tauri build`，同样的依赖安装与 rust-cache 在前置步骤完成 | 无实质差异 |
| 缓存 | 与手写完全一致，都依赖前置的 `Swatinem/rust-cache@v2` 步骤（S2） | 同左 |
| CLI 版本维护 | action 自动处理 `@tauri-apps/cli`（S3），无需在 workflow 中装 CLI，跟随 action 版本升级 | 需自行安装 CLI（`cargo install tauri-cli` 或 `npm i -D @tauri-apps/cli`）并自行维护版本与项目的 Tauri 版本匹配 |
| 仓库权限 | **仅构建时不需要 `contents: write`**——省略 `tagName`/`releaseName`/`releaseId` 三个参数即进入纯构建模式，不上传任何资产（S3）；发布场景才需要 `permissions: contents: write`（S2、S3） | 不需要任何写权限 |
| 产物处理 | 纯构建模式下产物留在工作区，可自行用 `actions/upload-artifact` 保存；另有 `uploadWorkflowArtifacts: true` 选项直接上传 workflow 工件（该选项可能随 actions/upload-artifact#331 变更，S3） | 产物在 `src-tauri/target/release/bundle/` 下，同样自行上传 |
| 参数透传 | 通过 `args:` 透传（如 `--target`、`--no-bundle`）（S3） | 直接写在命令行 |
| 维护性 | 官方文档与 tauri-action README 均以其为推荐入口，官方 workflow 即此形态（S2、S3） | 少一层抽象，但需手动跟进 CLI 版本与配置变化 |

**推荐结论：采用 `tauri-apps/tauri-action@v1`，省略 `tagName`/`releaseName`/`releaseId` 即得到纯构建模式。** 理由：

1. 官方 GitHub Actions 指南（S2）与 tauri-action README（S3）的标准 workflow 都是该形态，替换触发条件为 `pull_request`/`push` 即可用于验证构建，官方明确说明「You may freely modify the workflow name, change its triggers」且可添加 lint/test 步骤（S2）；
2. 免去 CLI 安装与版本匹配的维护负担（S3：action 自行处理 CLI）；
3. 不发布时不需要写权限，与仅验证场景的权限模型匹配（S3：省略三个参数即不创建 release）。

对「验证可构建」的 workflow，可再加 `args: ${{ matrix.args }}` 透传 `--no-bundle`（如需省去打包耗时，仅编译验证）——该项为可选项，官方示例默认打包（S2、S3）。

---

## 三、配套步骤推荐

### 1. `Swatinem/rust-cache@v2`

- **cache key 自动构成**：GitHub job_id（默认开启，可用 `add-job-id-key` 关闭）+ rustc release/host/hash（所有已安装工具链）+ Cargo.lock/Cargo.toml（仓库内全部）+ 根目录 `rust-toolchain`/`rust-toolchain.toml` 与 `.cargo/config.toml` 的 hash；当 `add-rust-environment-hash-key` 为 true（默认）时再叠加 RUSTFLAGS 等编译器相关环境变量 hash（默认匹配前缀 `CARGO CC CFLAGS CXX CMAKE RUST`）（S4）。
- **顺序要求**：工具链安装必须在 rust-cache **之前**（cache key 使用当前 rustc 版本），官方示例即 `dtolnay/rust-toolchain@stable` → `swatinem/rust-cache@v2`（S2、S4）。
- **与 fmt/clippy/test 配合**：rust-cache 恢复 `~/.cargo` 与 `./target` 后，同 job 后续所有 cargo 子命令（fmt/clippy/test/build）共享命中；多 job 间默认各 job 独立 key，需要共享时用 `shared-key`（跨 job 稳定，示例：`shared-key: "fmt-clippy-test"`）（S4）。
- **注意**：只缓存依赖编译产物（"only caches the dependencies of a crate"），工作区自身 crate 不缓存——clippy/test 主 crate 的编译每次重跑，复用的是依赖产物；PR 可读基线分支缓存但跨无关分支不共享（S4）。
- **monorepo 场景**：tauri 工程前端在根、Rust 在 `src-tauri/` 时，官方示例用 `workspaces: './src-tauri -> target'` 指定 target 目录（S2）——本项目若同布局应照此配置。
- 其他可选项：`cache-on-failure: "true"`（失败时也保存）、`save-if: ${{ github.ref == 'refs/heads/master' }}`（仅 master 保存）（S4）。

### 2. `pnpm/action-setup`

- **version 指定方式**：推荐**省略 `version` 输入**，把 pnpm 版本写入 package.json 的 `packageManager` 字段（如 `"packageManager": "pnpm@10.x.x"`），action 自动读取；若 package.json 无该字段则 `version` 必填。显式指定时支持 npm 版本语义（`10`、`^10.9.8`、`latest`）（S5）。
- **注意**：该 action **不安装 Node.js**，必须配合 `actions/setup-node` 使用（S5）。
- **run_install**：默认 `null`（不装依赖）；设 `true` 则自动 `pnpm install`（recursive）；也可传对象/数组精细控制（`recursive`/`cwd`/`args`）。官方示例仍然显式 `pnpm install`（S5）。
- **缓存**：`cache: true` 时自动缓存 pnpm store，无需手动 `pnpm store prune`（S5）。
- **版本警示**：pnpm v11+ 应改用 `pnpm/setup`（同一发布渠道的新 action，可同一步安装 Node.js/Bun/Deno）（S5）——选择包管理器版本时需注意当前 pnpm 主版本号。

### 3. 完整「仅验证构建」workflow 骨架（官方形态改写，S2）

```yaml
name: build-check
on:
  pull_request:
  push:
    branches: [dev]

jobs:
  build-check:
    strategy:
      fail-fast: false
      matrix:
        include:
          - platform: 'ubuntu-22.04'
          - platform: 'macos-latest'
          - platform: 'windows-latest'
    runs-on: ${{ matrix.platform }}
    steps:
      - uses: actions/checkout@v4

      - name: install dependencies (ubuntu only)
        if: matrix.platform == 'ubuntu-22.04'
        run: |
          sudo apt-get update
          sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf xdg-utils

      - uses: actions/setup-node@v4
        with:
          node-version: lts/*
          cache: 'pnpm'

      - uses: pnpm/action-setup@v6
        with:
          version: 10  # 或省略，改用 package.json 的 packageManager 字段
          cache: true

      - uses: dtolnay/rust-toolchain@stable

      - uses: swatinem/rust-cache@v2
        with:
          workspaces: './src-tauri -> target'

      - run: pnpm install --frozen-lockfile
      - run: pnpm build

      - name: tauri build（纯构建模式，不发布）
        uses: tauri-apps/tauri-action@v1
        with:
          args: --no-bundle  # 可选：跳过打包，仅编译验证
```

> 注：checkout/setup-node/setup-node 的 `@v4` 为官方示例当前引用的主版本（S2 示例为 checkout@v4 代际；tauri-action 示例引 checkout@v4，官方 v2 指南已更新到 checkout@v7/setup-node@v6，使用时取当前主版本即可）；`pnpm/action-setup@v6` 与 `pnpm/setup` 的取舍见上文。

---

## 四、结论要点

1. **Linux**：依赖需显式安装（tauri-action 不代装），核心为 `libwebkit2gtk-4.1-dev`（v2 必须 4.1，4.0 不行）+ `libappindicator3-dev` + `librsvg2-dev` + `patchelf` + `xdg-utils`；runner 建议固定 `ubuntu-22.04`（官方基线，`ubuntu-latest` 是浮动标签）。
2. **macOS**：runner 自带 Xcode，零额外系统依赖；只需为 `aarch64-apple-darwin`/`x86_64-apple-darwin` 加 rustup targets。
3. **Windows**：runner 自带 MSVC 与 WebView2，零额外处理；仅 msi 打包需 VBSCRIPT 可选功能。
4. **推荐 tauri-action 纯构建模式**（省略 tagName/releaseName/releaseId），免 CLI 维护、不要求写权限，官方文档即此形态。
5. **配套**：rust-cache 在工具链之后、cargo 命令之前，monorepo 用 `workspaces: './src-tauri -> target'`；pnpm 版本走 `packageManager` 字段。
