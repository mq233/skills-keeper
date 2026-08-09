// 矩阵状态 store：state { tools, rows, loading, error } + actions loadMatrix / scan + summary getter。
//
// 单 store 承载 Skill 库矩阵全部状态（spec §10）；组件只读 storeToRefs，不直接调 api。

import { defineStore } from "pinia";
import { computed, ref } from "vue";

import {
  getStatusMatrix,
  scan as apiScan,
  type MatrixRow,
  type StatusMatrix,
  type ToolInfo,
} from "../api";

export const useMatrixStore = defineStore("matrix", () => {
  const tools = ref<ToolInfo[]>([]);
  const rows = ref<MatrixRow[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);

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

  return { tools, rows, loading, error, summary, loadMatrix, scan };
});
