// Tauri command 调用封装层：统一 invoke 入口、`{code, message}` 错误解析。
//
// 前端仅通过本层与 Rust 引擎对话（不直接裸 invoke），见 docs/technical-plan.md §4.7。
// 类型为手写契约镜像（与 Rust 命令契约一一对应），不引入类型生成器（spec §10）。

import { invoke } from "@tauri-apps/api/core";

// ---- 契约类型镜像（docs/specs/s1-matrix.md §9）----

/** Sidecar（伴生元数据）——契约子集：source / targets（schemaVersion 1） */
export interface Sidecar {
  source: string | null;
  targets: string[];
}

/** Skill 实体：slug = 目录名；version 为字符串原文（YAML 标量陷阱规避） */
export interface Skill {
  slug: string;
  name: string;
  description: string;
  version: string | null;
  sidecar: Sidecar;
}

/** Vault 读取结果：invalid 为行级标记原因（中文文案），null = 合规 */
export interface SkillEntry {
  skill: Skill;
  invalid: string | null;
}

/** 状态徽章四态（契约：snake_case） */
export type Status = "consistent" | "pending" | "modified" | "missing";

/** 目标工具列信息：connected = false 即「未接入」（列级属性） */
export interface ToolInfo {
  id: string;
  connected: boolean;
}

/** 矩阵行：Skill + 各已接入工具状态（未接入工具不在 statuses 中） */
export interface MatrixRow {
  skill: SkillEntry;
  statuses: Record<string, Status>;
}

/** 状态矩阵（契约：get_status_matrix / scan 同形状） */
export interface StatusMatrix {
  tools: ToolInfo[];
  rows: MatrixRow[];
}

/** 目标工具显示名（契约 id → UI 文案） */
export const TOOL_LABELS: Record<string, string> = {
  "claude-code": "Claude Code",
  codex: "Codex",
  workbuddy: "WorkBuddy",
  trae: "Trae",
};

// ---- 统一错误解析：{code, message} → 抛带 code 属性的 Error ----

/** 引擎错误：message 为中文文案可直接展示，code 为契约错误码 */
export interface ApiError extends Error {
  code: string;
}

/** 判断 invoke 拒绝值是否为契约形状 `{code, message}` */
function isContractError(e: unknown): e is { code: string; message: string } {
  if (typeof e !== "object" || e === null) {
    return false;
  }
  const obj = e as Record<string, unknown>;
  return typeof obj.code === "string" && typeof obj.message === "string";
}

/** 统一错误解析：契约错误传 message，其余兜底为通用中文文案（不暴露技术堆栈） */
function toApiError(e: unknown): ApiError {
  if (isContractError(e)) {
    const err = new Error(e.message) as ApiError;
    err.code = e.code;
    return err;
  }
  const err = new Error("操作失败，请重试") as ApiError;
  err.code = "Internal";
  return err;
}

/** 调用 Tauri 命令并统一错误解析（{code, message} → 抛带 code 的 Error） */
async function invokeCommand<T>(command: string): Promise<T> {
  try {
    return await invoke<T>(command);
  } catch (e) {
    throw toApiError(e);
  }
}

// ---- 命令封装 ----

/** 全部 Skill 列表（S1 前端不调用，S4 导入器使用） */
export function listSkills(): Promise<SkillEntry[]> {
  return invokeCommand<SkillEntry[]>("list_skills");
}

/** 手动触发扫描并返回最新矩阵（一次往返） */
export function scan(): Promise<StatusMatrix> {
  return invokeCommand<StatusMatrix>("scan");
}

/** 当前状态矩阵 */
export function getStatusMatrix(): Promise<StatusMatrix> {
  return invokeCommand<StatusMatrix>("get_status_matrix");
}
