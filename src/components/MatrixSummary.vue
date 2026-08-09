<script setup lang="ts">
// 矩阵顶部摘要：手动扫描按钮 + 状态计数（被工具修改 / 待分发 / 缺失）
defineProps<{
  modified: number;
  pending: number;
  missing: number;
  loading: boolean;
}>();

const emit = defineEmits<{ scan: [] }>();
</script>

<template>
  <div class="summary">
    <button
      class="scan-button"
      :disabled="loading"
      data-testid="scan-button"
      @click="emit('scan')"
    >
      {{ loading ? "扫描中…" : "扫描" }}
    </button>
    <div class="counts" aria-label="状态摘要">
      <span
        v-if="modified > 0"
        class="count count--modified"
        data-testid="count-modified"
      >
        被工具修改 {{ modified }}
      </span>
      <span
        v-if="pending > 0"
        class="count count--pending"
        data-testid="count-pending"
      >
        待分发 {{ pending }}
      </span>
      <span
        v-if="missing > 0"
        class="count count--missing"
        data-testid="count-missing"
      >
        缺失 {{ missing }}
      </span>
      <span
        v-if="modified === 0 && pending === 0 && missing === 0"
        class="count count--ok"
      >
        一切一致
      </span>
    </div>
  </div>
</template>

<style scoped>
.summary {
  display: flex;
  align-items: center;
  gap: 16px;
  margin-bottom: 16px;
}

.scan-button {
  padding: 6px 18px;
  border: 1px solid #1a4fd8;
  border-radius: 6px;
  background: #1a4fd8;
  color: #ffffff;
  font-size: 14px;
  cursor: pointer;
}

.scan-button:hover:not(:disabled) {
  background: #1240b8;
}

.scan-button:disabled {
  opacity: 0.6;
  cursor: default;
}

.counts {
  display: flex;
  gap: 12px;
  font-size: 13px;
}

.count--modified {
  color: #bc4c00;
}

.count--pending {
  color: #9a6700;
}

.count--missing {
  color: #57606a;
}

.count--ok {
  color: #1a7f37;
}
</style>
