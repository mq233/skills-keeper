// 契约镜像 fixtures：与 Rust 命令契约 JSON 一一对应（spec §Testing Decisions 接缝②）。
// 覆盖矩阵展示分支：正常 Skill、中文名、invalid（缺 name）、未接入列、四态。

import type { StatusMatrix } from "../api";

/** 两行 × 四列矩阵：greeting（一致/待分发/未接入/缺失）+ broken（被工具修改 ×2 / invalid） */
export const matrixFixture: StatusMatrix = {
  tools: [
    { id: "claude-code", connected: true },
    { id: "codex", connected: true },
    { id: "workbuddy", connected: false }, // 未接入
    { id: "trae", connected: true },
  ],
  rows: [
    {
      skill: {
        skill: {
          slug: "greeting",
          name: "问候助手",
          description: "生成友好问候语",
          version: "1.0",
          sidecar: { source: null, targets: ["claude-code", "codex", "trae"] },
        },
        invalid: null,
      },
      statuses: {
        "claude-code": "consistent",
        codex: "pending",
        trae: "missing",
      },
    },
    {
      skill: {
        skill: {
          slug: "broken-skill",
          name: "",
          description: "只有描述",
          version: null,
          sidecar: { source: null, targets: [] },
        },
        invalid: "缺少 name",
      },
      statuses: {
        "claude-code": "modified",
        codex: "modified",
        trae: "missing",
      },
    },
  ],
};
