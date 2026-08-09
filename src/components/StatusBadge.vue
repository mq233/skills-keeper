<script setup lang="ts">
import type { Status } from "../api";

// 状态徽章四态（一致 / 待分发 / 被工具修改 / 缺失）；文案中文直写，
// data-i18n 语义 key 预留（S6 抽语言包，spec §10）
defineProps<{ status: Status }>();

const STATUS_TEXT: Record<Status, string> = {
  consistent: "一致",
  pending: "待分发",
  modified: "被工具修改",
  missing: "缺失",
};
</script>

<template>
  <span
    class="badge"
    :class="`badge--${status}`"
    :data-i18n="`status.${status}`"
  >
    {{ STATUS_TEXT[status] }}
  </span>
</template>

<style scoped>
.badge {
  display: inline-block;
  padding: 2px 10px;
  border-radius: 999px;
  font-size: 12px;
  line-height: 20px;
  white-space: nowrap;
}

.badge--consistent {
  color: #1a7f37;
  background: #dafbe1;
}

.badge--pending {
  color: #9a6700;
  background: #fff8c5;
}

.badge--modified {
  color: #bc4c00;
  background: #ffe1cc;
}

.badge--missing {
  color: #57606a;
  background: #e5e7eb;
}

@media (prefers-color-scheme: dark) {
  .badge--consistent {
    color: #7ee2a8;
    background: #1a4d2e;
  }

  .badge--pending {
    color: #eac54f;
    background: #4a3c11;
  }

  .badge--modified {
    color: #ffa657;
    background: #4e2a0e;
  }

  .badge--missing {
    color: #9ba1a6;
    background: #33383f;
  }
}
</style>
