# Skills Keeper

跨平台桌面软件，在本地统一托管 AI Skill（技能）与说明文件（Instruction），并一键分发到 Claude Code、Codex、workbuddy、Trae 等目标工具（Target）。

技术栈：Tauri 2 + Vue 3 + TypeScript + Vite + pnpm，Rust 侧分层结构（db / engine / commands）。

## 开发环境

| 依赖    | 版本要求                                        |
| ------- | ----------------------------------------------- |
| Node.js | ≥ 20                                            |
| pnpm    | ≥ 10                                            |
| Rust    | stable（当前 1.97）                             |
| 平台    | Windows 10/11、macOS、Linux（Tauri 2 支持范围） |

首次开发前安装依赖：

```bash
pnpm install
```

## 常用命令

| 命令                                 | 说明                                        |
| ------------------------------------ | ------------------------------------------- |
| `pnpm tauri dev`                     | 启动开发模式（Vite + Rust 编译 + 桌面窗口） |
| `pnpm build`                         | 前端类型检查 + 产物构建                     |
| `pnpm lint` / `pnpm lint:fix`        | ESLint 检查 / 修复                          |
| `pnpm test`                          | Vitest 单测                                 |
| `pnpm format` / `pnpm format:check`  | Prettier 格式化 / 检查                      |
| `pnpm fmt:rust` / `pnpm clippy:rust` | rustfmt 检查 / clippy（-D warnings）        |
| `pnpm tauri build`                   | 打包安装产物                                |

## 目录结构

```text
src/                前端（Vue 3 + TS，<script setup>）
  api/              invoke 封装（前端访问 Rust 命令的唯一入口）
  components/       通用组件
  stores/           Pinia 状态
  views/            页面（库视图 / 导入向导 / 设置 / 快照时间线）
  i18n/             文案与语言资源（中文先行）
src-tauri/          Rust 后端（Tauri 2）
  src/db/           SQLite 迁移与连接
  src/engine/       分发引擎（扫描 / 判定 / 分发 / 快照 / 回滚）
  src/commands/     Tauri 命令层
docs/               技术规划、调研与原型记录
```

## CI

三段式 GitHub Actions（见 `.github/workflows/ci.yml`）：

- **test-rust**：三平台矩阵（ubuntu-22.04 / windows / macOS），fmt --check + clippy -D warnings + cargo test
- **test-frontend**：ubuntu 单跑，lint + test + build
- **build-tauri**：仅 master push 触发，三平台验证可构建（不签名不发布，安装包留待 Phase 5）

## 文档指引

- 领域术语表：[CONTEXT.md](CONTEXT.md)（术语以条目主词为准）
- 技术规划：[docs/technical-plan.md](docs/technical-plan.md)（架构 / Schema / 实施阶段）
- 实施状态：GitHub Issues 上的 wayfinder 地图（阶段 0 工程骨架已验收）
