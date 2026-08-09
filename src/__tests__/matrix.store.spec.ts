// matrix store 测试：loadMatrix / scan 行为与 summary getter（spec §10）。

import { invoke } from "@tauri-apps/api/core";
import { createPinia, setActivePinia } from "pinia";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { matrixFixture } from "./fixtures";
import { useMatrixStore } from "../stores/matrix";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const invokeMock = vi.mocked(invoke);

describe("matrix store", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    invokeMock.mockReset();
  });

  it("loadMatrix 填充 tools/rows，summary 统计计数", async () => {
    invokeMock.mockResolvedValue(matrixFixture);
    const store = useMatrixStore();
    await store.loadMatrix();

    expect(store.tools).toHaveLength(4);
    expect(store.rows).toHaveLength(2);
    expect(invokeMock).toHaveBeenCalledWith("get_status_matrix");

    // summary：被工具修改 2、待分发 1、缺失 2（broken 的 trae + greeting 的 trae）
    expect(store.summary).toEqual({ modified: 2, pending: 1, missing: 2 });
  });

  it("scan 调用 scan 命令并刷新矩阵", async () => {
    invokeMock.mockResolvedValue(matrixFixture);
    const store = useMatrixStore();
    await store.scan();
    expect(invokeMock).toHaveBeenCalledWith("scan");
    expect(store.rows).toHaveLength(2);
  });

  it("失败时 error 存中文文案（契约 message），loading 复位", async () => {
    invokeMock.mockRejectedValue({ code: "Io", message: "读取文件失败：xxx" });
    const store = useMatrixStore();
    await store.loadMatrix();
    expect(store.error).toBe("读取文件失败：xxx");
    expect(store.loading).toBe(false);
    expect(store.rows).toHaveLength(0);
  });

  it("非契约失败 → 兜底文案", async () => {
    invokeMock.mockRejectedValue("boom");
    const store = useMatrixStore();
    await store.loadMatrix();
    expect(store.error).toBe("操作失败，请重试");
  });

  it("loading 在请求期间为 true", async () => {
    let resolve!: (v: unknown) => void;
    invokeMock.mockReturnValue(new Promise((r) => (resolve = r)));
    const store = useMatrixStore();
    const pending = store.loadMatrix();
    expect(store.loading).toBe(true);
    resolve(matrixFixture);
    await pending;
    expect(store.loading).toBe(false);
  });
});
