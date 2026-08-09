<script setup lang="ts">
// Skill 库主视图：矩阵顶部摘要（扫描按钮 + 计数）+ 状态矩阵表（spec §10 组件清单）。

import { storeToRefs } from "pinia";
import { onMounted } from "vue";

import MatrixSummary from "../components/MatrixSummary.vue";
import StatusMatrixTable from "../components/StatusMatrixTable.vue";
import { useMatrixStore } from "../stores/matrix";

const store = useMatrixStore();
const { tools, rows, loading, error, summary } = storeToRefs(store);

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
    <StatusMatrixTable v-else :tools="tools" :rows="rows" />
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
</style>
