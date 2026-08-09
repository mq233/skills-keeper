// SkillLibraryView 全流程测试（spec §Testing Decisions 接缝②）：
// mock invoke 层（vi.mock('@tauri-apps/api/core')），契约镜像 fixtures 驱动
// 加载渲染 → 摘要计数 → 错误态 → 扫描按钮 → 矩阵刷新。

import { invoke } from "@tauri-apps/api/core";
import { flushPromises, mount } from "@vue/test-utils";
import { createPinia } from "pinia";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { matrixFixture } from "./fixtures";
import SkillLibraryView from "../views/SkillLibraryView.vue";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const invokeMock = vi.mocked(invoke);

function mountView() {
  return mount(SkillLibraryView, { global: { plugins: [createPinia()] } });
}

describe("SkillLibraryView 全流程", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("挂载即加载矩阵：渲染行 × 列与状态摘要", async () => {
    invokeMock.mockResolvedValue(matrixFixture);
    const wrapper = mountView();
    await flushPromises();

    expect(invokeMock).toHaveBeenCalledWith("get_status_matrix");
    expect(wrapper.findAll('[data-testid="matrix-row"]')).toHaveLength(2);
    // 摘要：modified 2 / pending 1 / missing 2
    expect(wrapper.get('[data-testid="count-modified"]').text()).toBe(
      "被工具修改 2",
    );
    expect(wrapper.get('[data-testid="count-pending"]').text()).toBe(
      "待分发 1",
    );
    expect(wrapper.get('[data-testid="count-missing"]').text()).toBe("缺失 2");
    // 未接入列：标签 + 配置提示
    expect(wrapper.find('[data-testid="disconnected-col"]').text()).toContain(
      "未接入",
    );
    expect(wrapper.find('[data-testid="disconnected-col"]').text()).toContain(
      "配置路径后接入",
    );
    // 加载完成后无加载提示
    expect(wrapper.find('[data-testid="loading-hint"]').exists()).toBe(false);
  });

  it("加载失败：展示契约 message 中文文案", async () => {
    invokeMock.mockRejectedValue({
      code: "Io",
      message: "读取文件失败：无法访问目录",
    });
    const wrapper = mountView();
    await flushPromises();

    const error = wrapper.get('[data-testid="error-message"]');
    expect(error.text()).toBe("读取文件失败：无法访问目录");
    expect(wrapper.findAll('[data-testid="matrix-row"]')).toHaveLength(0);
  });

  it("点击扫描按钮触发 scan 并刷新矩阵", async () => {
    invokeMock.mockResolvedValue(matrixFixture);
    const wrapper = mountView();
    await flushPromises();
    expect(invokeMock).toHaveBeenCalledTimes(1);

    // 第二次返回更新后的矩阵（greeting 全部一致）
    const updated = structuredClone(matrixFixture);
    updated.rows[0].statuses = {
      "claude-code": "consistent",
      codex: "consistent",
      trae: "consistent",
    };
    invokeMock.mockResolvedValue(updated);
    await wrapper.get('[data-testid="scan-button"]').trigger("click");
    await flushPromises();

    expect(invokeMock).toHaveBeenCalledTimes(2);
    expect(invokeMock).toHaveBeenLastCalledWith("scan");
    // 刷新后摘要更新：greeting 的 pending 消失（broken 行的 modified 仍在）
    expect(wrapper.find('[data-testid="count-modified"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="count-pending"]').exists()).toBe(false);
    expect(wrapper.get('[data-testid="count-modified"]').text()).toBe(
      "被工具修改 2",
    );
  });

  it("扫描期间显示加载反馈（按钮禁用 + 加载提示）", async () => {
    let resolveScan!: (v: unknown) => void;
    invokeMock
      .mockResolvedValueOnce(matrixFixture)
      .mockReturnValueOnce(new Promise((r) => (resolveScan = r)));
    const wrapper = mountView();
    await flushPromises();

    await wrapper.get('[data-testid="scan-button"]').trigger("click");
    await flushPromises();
    // 扫描未完成：加载提示可见、按钮禁用
    expect(wrapper.find('[data-testid="loading-hint"]').exists()).toBe(true);
    expect(
      wrapper.get('[data-testid="scan-button"]').attributes("disabled"),
    ).toBeDefined();

    resolveScan(matrixFixture);
    await flushPromises();
    expect(wrapper.find('[data-testid="loading-hint"]').exists()).toBe(false);
  });

  it("扫描失败：错误态展示中文文案，旧矩阵保留", async () => {
    invokeMock
      .mockResolvedValueOnce(matrixFixture)
      .mockRejectedValueOnce({ code: "Internal", message: "内部错误" });
    const wrapper = mountView();
    await flushPromises();

    await wrapper.get('[data-testid="scan-button"]').trigger("click");
    await flushPromises();

    expect(wrapper.get('[data-testid="error-message"]').text()).toBe(
      "内部错误",
    );
    // 旧矩阵仍渲染
    expect(wrapper.findAll('[data-testid="matrix-row"]')).toHaveLength(2);
  });
});
