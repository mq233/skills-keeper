// 矩阵组件测试：状态徽章四态、矩阵摘要（计数 / 扫描按钮 / 加载态）、
// 矩阵表格（行 × 列、未接入列、invalid 标记）。

import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";

import { matrixFixture } from "./fixtures";
import DeployBar from "../components/DeployBar.vue";
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
      props: {
        tools: matrixFixture.tools,
        rows: matrixFixture.rows,
        selectedSlugs: new Set<string>(),
        deploying: false,
      },
    });
    const rows = wrapper.findAll('[data-testid="matrix-row"]');
    expect(rows).toHaveLength(2);
    const headers = wrapper.findAll("thead th");
    expect(headers).toHaveLength(6); // 勾选 + Skill + 4 工具
    expect(headers[2].text()).toContain("Claude Code");
  });

  it("未接入列：列头「未接入」提示，单元格无四态", () => {
    const wrapper = mount(StatusMatrixTable, {
      props: {
        tools: matrixFixture.tools,
        rows: matrixFixture.rows,
        selectedSlugs: new Set<string>(),
        deploying: false,
      },
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
      props: {
        tools: matrixFixture.tools,
        rows: matrixFixture.rows,
        selectedSlugs: new Set<string>(),
        deploying: false,
      },
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
      props: {
        tools: matrixFixture.tools,
        rows: matrixFixture.rows,
        selectedSlugs: new Set<string>(),
        deploying: false,
      },
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
      props: {
        tools: matrixFixture.tools,
        rows: [],
        selectedSlugs: new Set<string>(),
        deploying: false,
      },
    });
    expect(wrapper.findAll("tbody tr")).toHaveLength(0);
    expect(wrapper.findAll("thead th")).toHaveLength(6);
  });
});

describe("StatusMatrixTable S2 分发交互", () => {
  it("行勾选 checkbox：勾选态渲染 + change 触发 toggle-select", async () => {
    const selected = new Set(["greeting"]);
    const wrapper = mount(StatusMatrixTable, {
      props: {
        tools: matrixFixture.tools,
        rows: matrixFixture.rows,
        selectedSlugs: selected,
        deploying: false,
      },
    });
    const checked = wrapper.get('[data-testid="select-greeting"]');
    expect((checked.element as HTMLInputElement).checked).toBe(true);
    const unchecked = wrapper.get('[data-testid="select-broken-skill"]');
    expect((unchecked.element as HTMLInputElement).checked).toBe(false);

    await unchecked.trigger("change");
    expect(wrapper.emitted("toggle-select")).toEqual([["broken-skill"]]);
  });

  it("列头分发全部：仅已接入列有按钮；点击触发 deploy-all", async () => {
    const wrapper = mount(StatusMatrixTable, {
      props: {
        tools: matrixFixture.tools,
        rows: matrixFixture.rows,
        selectedSlugs: new Set<string>(),
        deploying: false,
      },
    });
    const buttons = wrapper.findAll('[data-testid="deploy-all"]');
    expect(buttons, "3 个已接入工具列").toHaveLength(3);
    // 未接入列（workbuddy）不显示分发全部
    expect(wrapper.find('[data-testid="disconnected-col"]').exists()).toBe(
      true,
    );

    // 点击 codex 列（第 2 个已接入列）按钮
    await buttons[1].trigger("click");
    expect(wrapper.emitted("deploy-all")).toEqual([["codex"]]);
  });

  it("deploying 时勾选与分发全部按钮禁用", () => {
    const wrapper = mount(StatusMatrixTable, {
      props: {
        tools: matrixFixture.tools,
        rows: matrixFixture.rows,
        selectedSlugs: new Set<string>(),
        deploying: true,
      },
    });
    const checkbox = wrapper.get('[data-testid="select-greeting"]');
    expect(checkbox.attributes("disabled")).toBeDefined();
    for (const btn of wrapper.findAll('[data-testid="deploy-all"]')) {
      expect(btn.attributes("disabled")).toBeDefined();
    }
  });
});

describe("DeployBar 底部批量条与结果条", () => {
  it("勾选非空时显示「已选 N 项 / 取消 / 分发所选」", async () => {
    const wrapper = mount(DeployBar, {
      props: { count: 2, deploying: false, result: null },
    });
    expect(wrapper.get('[data-testid="selected-count"]').text()).toBe(
      "已选 2 项",
    );
    await wrapper.get('[data-testid="cancel-select"]').trigger("click");
    expect(wrapper.emitted("cancel")).toHaveLength(1);
    await wrapper.get('[data-testid="deploy-selected"]').trigger("click");
    expect(wrapper.emitted("deploy-selected")).toHaveLength(1);
  });

  it("勾选为空且无结果时不渲染", () => {
    const wrapper = mount(DeployBar, {
      props: { count: 0, deploying: false, result: null },
    });
    expect(wrapper.find('[data-testid="deploy-bar"]').exists()).toBe(false);
  });

  it("deploying 时按钮禁用并显示「正在分发…」", () => {
    const wrapper = mount(DeployBar, {
      props: { count: 1, deploying: true, result: null },
    });
    expect(
      wrapper.get('[data-testid="deploy-selected"]').attributes("disabled"),
    ).toBeDefined();
    expect(wrapper.get('[data-testid="deploy-selected"]').text()).toBe(
      "正在分发…",
    );
  });

  it("结果条：成功/失败计数 + 失败详情展开列表（中文原因）", async () => {
    const result = {
      ok: [{ tool_id: "codex", skill_slug: "greeting" }],
      failed: [
        {
          tool_id: "codex",
          skill_slug: "broken-skill",
          code: "InvalidSkill",
          message: "缺少 name",
        },
      ],
    };
    const wrapper = mount(DeployBar, {
      props: { count: 0, deploying: false, result },
    });
    expect(wrapper.get('[data-testid="deploy-summary"]').text()).toBe(
      "成功 1 项 / 失败 1 项",
    );
    // 失败详情默认收起，展开后显示中文原因
    expect(wrapper.find('[data-testid="failed-list"]').exists()).toBe(false);
    await wrapper.get('[data-testid="toggle-failed"]').trigger("click");
    expect(wrapper.get('[data-testid="failed-list"]').text()).toContain(
      "缺少 name",
    );
    // 关闭 → dismiss-result
    await wrapper.get('[data-testid="dismiss-result"]').trigger("click");
    expect(wrapper.emitted("dismiss-result")).toHaveLength(1);
  });

  it("全部成功时无失败详情按钮", () => {
    const wrapper = mount(DeployBar, {
      props: {
        count: 0,
        deploying: false,
        result: {
          ok: [{ tool_id: "codex", skill_slug: "greeting" }],
          failed: [],
        },
      },
    });
    expect(wrapper.find('[data-testid="toggle-failed"]').exists()).toBe(false);
    expect(wrapper.get('[data-testid="deploy-summary"]').text()).toBe(
      "成功 1 项 / 失败 0 项",
    );
  });
});
