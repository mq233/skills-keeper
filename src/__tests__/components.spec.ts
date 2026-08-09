// 矩阵组件测试：状态徽章四态、矩阵摘要（计数 / 扫描按钮 / 加载态）、
// 矩阵表格（行 × 列、未接入列、invalid 标记）。

import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";

import { matrixFixture } from "./fixtures";
import MatrixSummary from "../components/MatrixSummary.vue";
import StatusBadge from "../components/StatusBadge.vue";
import StatusMatrixTable from "../components/StatusMatrixTable.vue";

describe("StatusBadge 四态", () => {
  it.each([
    ["consistent", "一致"],
    ["pending", "待分发"],
    ["modified", "被工具修改"],
    ["missing", "缺失"],
  ] as const)("%s → %s", (status, text) => {
    const wrapper = mount(StatusBadge, { props: { status } });
    expect(wrapper.text()).toBe(text);
    expect(wrapper.classes()).toContain(`badge--${status}`);
  });
});

describe("MatrixSummary 摘要与扫描按钮", () => {
  it("渲染非零计数，零计数不显示", () => {
    const wrapper = mount(MatrixSummary, {
      props: { modified: 2, pending: 1, missing: 0, loading: false },
    });
    expect(wrapper.get('[data-testid="count-modified"]').text()).toBe(
      "被工具修改 2",
    );
    expect(wrapper.get('[data-testid="count-pending"]').text()).toBe(
      "待分发 1",
    );
    expect(wrapper.find('[data-testid="count-missing"]').exists()).toBe(false);
  });

  it("全零时显示「一切一致」", () => {
    const wrapper = mount(MatrixSummary, {
      props: { modified: 0, pending: 0, missing: 0, loading: false },
    });
    expect(wrapper.text()).toContain("一切一致");
  });

  it("点击扫描按钮触发 scan 事件；loading 时按钮禁用并显示扫描中", async () => {
    const wrapper = mount(MatrixSummary, {
      props: { modified: 0, pending: 0, missing: 0, loading: false },
    });
    await wrapper.get('[data-testid="scan-button"]').trigger("click");
    expect(wrapper.emitted("scan")).toHaveLength(1);

    await wrapper.setProps({ loading: true });
    const btn = wrapper.get('[data-testid="scan-button"]');
    expect(btn.attributes("disabled")).toBeDefined();
    expect(btn.text()).toBe("扫描中…");
  });
});

describe("StatusMatrixTable 矩阵渲染", () => {
  it("行 × 列渲染：2 行 × 4 工具列 + Skill 列", () => {
    const wrapper = mount(StatusMatrixTable, {
      props: { tools: matrixFixture.tools, rows: matrixFixture.rows },
    });
    const rows = wrapper.findAll('[data-testid="matrix-row"]');
    expect(rows).toHaveLength(2);
    const headers = wrapper.findAll("thead th");
    expect(headers).toHaveLength(5); // Skill + 4 工具
    expect(headers[1].text()).toContain("Claude Code");
  });

  it("未接入列：列头「未接入」提示，单元格无四态", () => {
    const wrapper = mount(StatusMatrixTable, {
      props: { tools: matrixFixture.tools, rows: matrixFixture.rows },
    });
    const disconnected = wrapper.find('[data-testid="disconnected-col"]');
    expect(disconnected.text()).toContain("未接入");
    expect(disconnected.text()).toContain("配置路径后接入");
    // workbuddy 列（第 3 个工具列）单元格渲染「—」而非徽章
    const workbuddyCells = wrapper
      .findAll("tbody tr")
      .map((tr) => tr.findAll("td.status-cell")[2].text());
    expect(workbuddyCells).toEqual(["—", "—"]);
  });

  it("状态徽章四态渲染（含 invalid 行级标记）", () => {
    const wrapper = mount(StatusMatrixTable, {
      props: { tools: matrixFixture.tools, rows: matrixFixture.rows },
    });
    const badges = wrapper.findAll(".badge");
    const badgeTexts = badges.map((b) => b.text());
    expect(badgeTexts).toContain("一致");
    expect(badgeTexts).toContain("待分发");
    expect(badgeTexts).toContain("被工具修改");
    expect(badgeTexts).toContain("缺失");

    const invalid = wrapper.find('[data-testid="invalid-mark"]');
    expect(invalid.text()).toContain("缺少 name");
  });

  it("显示名称 / 版本 / slug", () => {
    const wrapper = mount(StatusMatrixTable, {
      props: { tools: matrixFixture.tools, rows: matrixFixture.rows },
    });
    const firstRow = wrapper.findAll('[data-testid="matrix-row"]')[0];
    expect(firstRow.text()).toContain("问候助手");
    expect(firstRow.text()).toContain("v1.0");
    expect(firstRow.text()).toContain("greeting");
  });
});

describe("StatusMatrixTable 空态", () => {
  it("矩阵无行时渲染空表格（不崩溃）", () => {
    const wrapper = mount(StatusMatrixTable, {
      props: { tools: matrixFixture.tools, rows: [] },
    });
    expect(wrapper.findAll("tbody tr")).toHaveLength(0);
    expect(wrapper.findAll("thead th")).toHaveLength(5);
  });
});
