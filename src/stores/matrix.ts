// 矩阵状态 store：state { tools, rows, loading, error } + actions loadMatrix / scan + summary getter。
//
// 单 store 承载 Skill 库矩阵全部状态（spec §10）；组件只读 storeToRefs，不直接调 api。

import { defineStore } from "pinia";
import { computed, ref } from "vue";

import {
  deploy as apiDeploy,
  getStatusMatrix,
  scan as apiScan,
  type DeployOkItem,
  type DeployResult,
  type MatrixRow,
  type StatusMatrix,
  type ToolInfo,
} from "../api";

export const useMatrixStore = defineStore("matrix", () => {
  const tools = ref<ToolInfo[]>([]);
  const rows = ref<MatrixRow[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);

  // ---- S2 分发交互状态（会话态不落库）----
  /** 行勾选（Skill → 全部已接入目标工具）；勾选为会话态，刷新/重开后清空 */
  const selectedSlugs = ref<Set<string>>(new Set());
  /** 分发进行中（循环逐工具 loading） */
  const deploying = ref(false);
  /** 部分成功结果（批量条结果条；任意交互清除） */
  const deployResult = ref<DeployResult | null>(null);
  /** 分发级错误（重扫中止 / 快照失败等，modal 展示；整体 Err 停止循环） */
  const deployError = ref<string | null>(null);
  /** 已选数量 */
  const selectedCount = computed(() => selectedSlugs.value.size);

  /** 状态摘要计数：被工具修改 / 待分发 / 缺失（矩阵顶部摘要） */
  const summary = computed(
    (): { modified: number; pending: number; missing: number } => {
      let modified = 0;
      let pending = 0;
      let missing = 0;
      for (const row of rows.value) {
        for (const status of Object.values(row.statuses)) {
          if (status === "modified") {
            modified += 1;
          } else if (status === "pending") {
            pending += 1;
          } else if (status === "missing") {
            missing += 1;
          }
        }
      }
      return { modified, pending, missing };
    },
  );

  /** 应用（重新）矩阵：get_status_matrix */
  async function loadMatrix(): Promise<void> {
    await applyMatrix(getStatusMatrix);
  }

  /** 手动触发扫描并刷新矩阵：scan */
  async function scan(): Promise<void> {
    await applyMatrix(apiScan);
  }

  /** 共用执行器：错误存 error（中文文案），不向组件抛原始异常 */
  async function applyMatrix(
    fetch: () => Promise<StatusMatrix>,
  ): Promise<void> {
    loading.value = true;
    error.value = null;
    try {
      const matrix = await fetch();
      tools.value = matrix.tools;
      rows.value = matrix.rows;
    } catch (e) {
      error.value = e instanceof Error ? e.message : "操作失败，请重试";
    } finally {
      loading.value = false;
    }
  }

  // ---- S2 分发动作 ----

  /** 勾选 / 取消勾选某行（会话态 Set 整体替换，保持响应式） */
  function toggleSelect(slug: string): void {
    const next = new Set(selectedSlugs.value);
    if (next.has(slug)) {
      next.delete(slug);
    } else {
      next.add(slug);
    }
    selectedSlugs.value = next;
  }

  /** 清空勾选（底部批量条「取消」） */
  function clearSelection(): void {
    selectedSlugs.value = new Set();
  }

  /** 分发所选：对每个已接入目标工具串行调用；整体 Err 立即停止（modal 展示） */
  async function deploySelected(): Promise<void> {
    if (selectedSlugs.value.size === 0) {
      return;
    }
    await deployLoop(
      tools.value.filter((t) => t.connected).map((t) => t.id),
      () => [...selectedSlugs.value],
    );
  }

  /** 列头分发全部：单工具一次，前端算该工具列全部行（含 invalid 行）slug 全集 */
  async function deployAll(toolId: string): Promise<void> {
    await deployLoop([toolId], () => rows.value.map((r) => r.skill.skill.slug));
  }

  /** 共用分发循环：逐工具 deploy → 合并部分成功 → 每工具完成即置该工具列 ok 行 */
  async function deployLoop(
    toolIds: string[],
    slugsOf: () => string[],
  ): Promise<void> {
    deploying.value = true;
    deployResult.value = null;
    deployError.value = null;
    const merged: DeployResult = { ok: [], failed: [] };
    try {
      for (const toolId of toolIds) {
        const r = await apiDeploy({ tool_id: toolId, skill_slugs: slugsOf() });
        merged.ok.push(...r.ok);
        merged.failed.push(...r.failed);
        // 分发后置一致（引擎返回 ok = 落盘 + 记录成功，不重扫）；failed 保持原状态
        applyOk(merged.ok);
      }
      deployResult.value = merged;
    } catch (e) {
      // 分发级失败（重扫中止 / 快照失败）→ 停止循环，modal 展示中文提示
      deployError.value = e instanceof Error ? e.message : "分发失败，请重试";
    } finally {
      deploying.value = false;
    }
  }

  /** ok 条目 → 对应行×列直接置「一致」（引擎侧保证下次扫描必一致，不重扫） */
  function applyOk(ok: DeployOkItem[]): void {
    for (const item of ok) {
      const row = rows.value.find(
        (r) => r.skill.skill.slug === item.skill_slug,
      );
      if (row && row.statuses[item.tool_id]) {
        row.statuses[item.tool_id] = "consistent";
      }
    }
  }

  /** 关闭部分成功结果条（任意交互清除） */
  function dismissResult(): void {
    deployResult.value = null;
  }

  /** 关闭分发级错误 modal（用户处理后再次触发分发） */
  function dismissDeployError(): void {
    deployError.value = null;
  }

  return {
    tools,
    rows,
    loading,
    error,
    summary,
    loadMatrix,
    scan,
    // S2 分发
    selectedSlugs,
    selectedCount,
    deploying,
    deployResult,
    deployError,
    toggleSelect,
    clearSelection,
    deploySelected,
    deployAll,
    dismissResult,
    dismissDeployError,
  };
});
