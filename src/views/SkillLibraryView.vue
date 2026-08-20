<script setup lang="ts">
// Skill 库主视图：矩阵顶部摘要（扫描按钮 + 计数）+ 状态矩阵表（勾选 / 分发全部）
// + 底部批量条（分发所选 / 部分成功结果条）+ 分发级错误 modal（spec §10 组件清单）。

import { storeToRefs } from "pinia";
import { onMounted } from "vue";

import DeployBar from "../components/DeployBar.vue";
import MatrixSummary from "../components/MatrixSummary.vue";
import StatusMatrixTable from "../components/StatusMatrixTable.vue";
import { useMatrixStore } from "../stores/matrix";

const store = useMatrixStore();
const {
  tools,
  rows,
  loading,
  error,
  summary,
  selectedSlugs,
  selectedCount,
  deploying,
  deployResult,
  deployError,
} = storeToRefs(store);

onMounted(() => {
  void store.loadMatrix();
});
</script>

<template>
  <div class="view">
    <h1>Skill 库</h1>
    <MatrixSummary
      :modified="summary.modified"
      :pending="summary.pending"
      :missing="summary.missing"
      :loading="loading"
      @scan="store.scan"
    />
    <p v-if="error" class="error" data-testid="error-message">{{ error }}</p>
    <p v-if="loading" class="loading" data-testid="loading-hint">正在扫描…</p>
    <StatusMatrixTable
      v-else
      :tools="tools"
      :rows="rows"
      :selected-slugs="selectedSlugs"
      :deploying="deploying"
      @toggle-select="store.toggleSelect"
      @deploy-all="store.deployAll"
    />
    <DeployBar
      :count="selectedCount"
      :deploying="deploying"
      :result="deployResult"
      @cancel="store.clearSelection"
      @deploy-selected="store.deploySelected"
      @dismiss-result="store.dismissResult"
    />
    <!-- 分发级错误（重扫中止等）：modal 展示中文提示 + 被修改清单，确认后再次触发 -->
    <div v-if="deployError" class="modal-overlay" data-testid="abort-modal">
      <div class="modal" role="alertdialog" aria-label="分发未完成">
        <h2 class="modal-title">分发未完成</h2>
        <pre class="modal-body">{{ deployError }}</pre>
        <button
          class="modal-close"
          data-testid="dismiss-deploy-error"
          @click="store.dismissDeployError"
        >
          知道了
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.view h1 {
  font-size: 20px;
  margin: 0 0 16px;
}

.loading {
  color: #57606a;
  font-size: 13px;
}

.error {
  color: #d1242f;
  font-size: 13px;
  background: #ffebe9;
  border: 1px solid #ffcecb;
  border-radius: 6px;
  padding: 8px 12px;
}

.modal-overlay {
  position: fixed;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgb(0 0 0 / 40%);
  z-index: 100;
}

.modal {
  max-width: 480px;
  width: 90%;
  padding: 18px 20px;
  background: #ffffff;
  border-radius: 10px;
  box-shadow: 0 8px 24px rgb(0 0 0 / 20%);
}

.modal-title {
  margin: 0 0 10px;
  font-size: 16px;
}

.modal-body {
  margin: 0 0 14px;
  font-size: 13px;
  line-height: 1.7;
  color: #d1242f;
  white-space: pre-wrap;
  word-break: break-word;
  font-family: inherit;
  background: #fff8f8;
  border: 1px solid #ffcecb;
  border-radius: 6px;
  padding: 10px 12px;
}

.modal-close {
  padding: 5px 18px;
  border: 1px solid #1f6feb;
  border-radius: 6px;
  background: #1f6feb;
  color: #ffffff;
  font-size: 13px;
  cursor: pointer;
}

.modal-close:hover {
  background: #1857c4;
}

@media (prefers-color-scheme: dark) {
  .modal {
    background: #22262c;
  }

  .modal-body {
    background: #2d1c1e;
    border-color: #5c2327;
  }
}
</style>
