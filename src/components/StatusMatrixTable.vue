<script setup lang="ts">
import { TOOL_LABELS, type MatrixRow, type ToolInfo } from "../api";
import StatusBadge from "./StatusBadge.vue";

// 矩阵表格：行 = Skill（名称 / 版本 / 描述 / invalid 标记），列 = 目标工具（状态徽章）。
// 未接入列是列级属性：列头显示「未接入」+ 配置提示，单元格仅四态（spec §6）。
// S2 列头「分发全部」按钮占位、S3 行内 diff 展开占位——此处不实现。
defineProps<{
  tools: ToolInfo[];
  rows: MatrixRow[];
}>();

/** 契约工具 id → 显示名（未知 id 原样兜底） */
function toolLabel(id: string): string {
  return TOOL_LABELS[id] ?? id;
}
</script>

<template>
  <table class="matrix" data-testid="status-matrix">
    <thead>
      <tr>
        <th class="skill-head">Skill</th>
        <th v-for="tool in tools" :key="tool.id" class="tool-head">
          <span class="tool-label">{{ toolLabel(tool.id) }}</span>
          <span
            v-if="!tool.connected"
            class="disconnected"
            title="未配置可用的用户级目录路径，可在设置页配置后接入"
            data-testid="disconnected-col"
          >
            <span data-i18n="tool.disconnected">未接入</span>
            <span class="disconnected-hint" data-i18n="tool.disconnected-hint">
              配置路径后接入
            </span>
          </span>
        </th>
      </tr>
    </thead>
    <tbody>
      <tr
        v-for="{ skill: entry, statuses } in rows"
        :key="entry.skill.slug"
        data-testid="matrix-row"
      >
        <td class="skill-cell">
          <div class="skill-name">
            <span class="name">{{ entry.skill.name || entry.skill.slug }}</span>
            <span v-if="entry.skill.version" class="version"
              >v{{ entry.skill.version }}</span
            >
          </div>
          <div class="skill-slug">{{ entry.skill.slug }}</div>
          <div
            v-if="entry.invalid"
            class="invalid"
            :title="entry.invalid"
            data-testid="invalid-mark"
          >
            不合规：{{ entry.invalid }}
          </div>
        </td>
        <td v-for="tool in tools" :key="tool.id" class="status-cell">
          <StatusBadge
            v-if="tool.connected && statuses[tool.id]"
            :status="statuses[tool.id]"
          />
          <span v-else class="not-connected" aria-label="未接入">—</span>
        </td>
      </tr>
    </tbody>
  </table>
</template>

<style scoped>
.matrix {
  width: 100%;
  border-collapse: collapse;
  background: #ffffff;
  border: 1px solid #e2e5e9;
  border-radius: 8px;
  font-size: 13px;
}

th,
td {
  padding: 10px 14px;
  text-align: left;
  border-bottom: 1px solid #eceef1;
}

thead th {
  background: #fafbfc;
  font-weight: 600;
  color: #373c43;
  white-space: nowrap;
}

tbody tr:last-child td {
  border-bottom: none;
}

.skill-cell {
  min-width: 240px;
}

.skill-name {
  display: flex;
  align-items: baseline;
  gap: 8px;
}

.name {
  font-weight: 600;
}

.version {
  color: #57606a;
  font-size: 12px;
}

.skill-slug {
  color: #9ba1a6;
  font-size: 12px;
}

.invalid {
  margin-top: 4px;
  color: #bc4c00;
  font-size: 12px;
}

.tool-head {
  position: relative;
}

.tool-label {
  display: block;
}

.disconnected {
  display: block;
  margin-top: 2px;
  font-size: 11px;
  font-weight: 400;
  color: #9a6700;
}

.disconnected-hint {
  display: block;
  color: #9ba1a6;
}

.status-cell {
  text-align: center;
  white-space: nowrap;
}

.not-connected {
  color: #c3c8ce;
}

@media (prefers-color-scheme: dark) {
  .matrix {
    background: #22262c;
    border-color: #33383f;
  }

  th,
  td {
    border-bottom-color: #2c3138;
  }

  thead th {
    background: #262b32;
    color: #d4d7dc;
  }

  .version,
  .skill-slug {
    color: #8b9098;
  }

  .invalid {
    color: #ffa657;
  }
}
</style>
