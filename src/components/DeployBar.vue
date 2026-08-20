<script setup lang="ts">
import { ref } from "vue";

import type { DeployResult } from "../api";

// 底部批量条（S2）：勾选非空时显示「已选 N 项 / 取消 / 分发所选」；
// 分发完成后原地变结果条「成功 N 项 / 失败 M 项」（失败项可展开，中文原因）。
// 结果条任意交互清除（dismiss-result 由父级触发）。

defineProps<{
  count: number;
  deploying: boolean;
  result: DeployResult | null;
}>();

const emit = defineEmits<{
  (e: "cancel"): void;
  (e: "deploy-selected"): void;
  (e: "dismiss-result"): void;
}>();

/** 失败详情展开态（默认收起） */
const expanded = ref(false);
</script>

<template>
  <div v-if="count > 0 || result" class="deploy-bar" data-testid="deploy-bar">
    <!-- 批量条：勾选态 -->
    <template v-if="!result">
      <span class="count" data-testid="selected-count"
        >已选 {{ count }} 项</span
      >
      <button
        class="ghost"
        :disabled="deploying"
        data-testid="cancel-select"
        @click="emit('cancel')"
      >
        取消
      </button>
      <button
        class="primary"
        :disabled="deploying"
        data-testid="deploy-selected"
        @click="emit('deploy-selected')"
      >
        {{ deploying ? "正在分发…" : "分发所选" }}
      </button>
    </template>
    <!-- 结果条：部分成功反馈 -->
    <template v-else>
      <span class="count" data-testid="deploy-summary">
        成功 {{ result.ok.length }} 项 / 失败 {{ result.failed.length }} 项
      </span>
      <button
        v-if="result.failed.length > 0"
        class="ghost"
        data-testid="toggle-failed"
        @click="expanded = !expanded"
      >
        {{ expanded ? "收起" : `失败详情（${result.failed.length}）` }}
      </button>
      <ul
        v-if="expanded && result.failed.length > 0"
        class="failed-list"
        data-testid="failed-list"
      >
        <li v-for="f in result.failed" :key="`${f.tool_id}-${f.skill_slug}`">
          <span class="failed-slug">{{ f.skill_slug }}</span>
          <span class="failed-tool">（{{ f.tool_id }}）</span>
          <span class="failed-msg">{{ f.message }}</span>
        </li>
      </ul>
      <button
        class="primary"
        data-testid="dismiss-result"
        @click="emit('dismiss-result')"
      >
        关闭
      </button>
    </template>
  </div>
</template>

<style scoped>
.deploy-bar {
  position: sticky;
  bottom: 0;
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
  margin-top: 12px;
  padding: 10px 14px;
  background: #ffffff;
  border: 1px solid #d0d7de;
  border-radius: 8px;
  box-shadow: 0 -2px 8px rgb(0 0 0 / 6%);
}

.count {
  font-size: 13px;
  font-weight: 600;
  color: #373c43;
}

button {
  font-size: 13px;
  padding: 4px 14px;
  border-radius: 6px;
  cursor: pointer;
}

button:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.primary {
  border: 1px solid #1f6feb;
  background: #1f6feb;
  color: #ffffff;
}

.primary:hover:not(:disabled) {
  background: #1857c4;
}

.ghost {
  border: 1px solid #d0d7de;
  background: #f6f8fa;
  color: #373c43;
}

.ghost:hover:not(:disabled) {
  background: #eaeef2;
}

.failed-list {
  flex-basis: 100%;
  margin: 4px 0 0;
  padding: 8px 12px;
  list-style: none;
  background: #fff8f8;
  border: 1px solid #ffcecb;
  border-radius: 6px;
}

.failed-list li {
  font-size: 12px;
  line-height: 1.7;
  color: #d1242f;
}

.failed-slug {
  font-weight: 600;
}

.failed-tool {
  color: #57606a;
}

.failed-msg {
  color: #57606a;
}

@media (prefers-color-scheme: dark) {
  .deploy-bar {
    background: #22262c;
    border-color: #33383f;
  }

  .count {
    color: #d4d7dc;
  }

  .ghost {
    border-color: #33383f;
    background: #2c3138;
    color: #d4d7dc;
  }

  .failed-list {
    background: #2d1c1e;
    border-color: #5c2327;
  }
}
</style>
