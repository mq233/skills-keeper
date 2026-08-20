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

describe("matrix store S2 分发", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    invokeMock.mockReset();
    // 默认：矩阵读取成功；deploy 返回空结果（各用例覆写）
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === "get_status_matrix" || cmd === "scan") {
        return Promise.resolve(matrixFixture);
      }
      if (cmd === "deploy") {
        return Promise.resolve({ ok: [], failed: [] });
      }
      return Promise.reject("unknown command");
    });
  });

  it("行勾选为会话态：toggleSelect / clearSelection / selectedCount", async () => {
    const store = useMatrixStore();
    await store.loadMatrix();
    expect(store.selectedCount).toBe(0);
    store.toggleSelect("greeting");
    expect(store.selectedSlugs.has("greeting")).toBe(true);
    expect(store.selectedCount).toBe(1);
    store.toggleSelect("greeting");
    expect(store.selectedCount, "再次点击应取消勾选").toBe(0);
    store.toggleSelect("broken-skill");
    store.clearSelection();
    expect(store.selectedCount, "取消应清空会话态").toBe(0);
  });

  it("分发所选：对每个已接入工具串行调用，ok 置一致、failed 保持原状态", async () => {
    invokeMock.mockImplementation((cmd: string, args?: unknown) => {
      if (cmd === "get_status_matrix") {
        return Promise.resolve(matrixFixture);
      }
      if (cmd === "deploy") {
        const req = args as { tool_id: string; skill_slugs: string[] };
        return Promise.resolve({
          ok: [{ tool_id: req.tool_id, skill_slug: "greeting" }],
          failed: [
            {
              tool_id: req.tool_id,
              skill_slug: "broken-skill",
              code: "InvalidSkill",
              message: "缺少 name",
            },
          ],
        });
      }
      return Promise.reject("unknown command");
    });
    const store = useMatrixStore();
    await store.loadMatrix();
    store.toggleSelect("greeting");
    store.toggleSelect("broken-skill");
    await store.deploySelected();

    // 3 个已接入工具串行调用（workbuddy 未接入跳过）
    const deployCalls = invokeMock.mock.calls.filter((c) => c[0] === "deploy");
    expect(deployCalls).toHaveLength(3);
    expect(
      deployCalls.map((c) => (c[1] as { tool_id: string }).tool_id),
    ).toEqual(["claude-code", "codex", "trae"]);
    // 每次调用都传勾选集（含 invalid 行——勾选即意图）
    for (const call of deployCalls) {
      const slugs = (call[1] as { skill_slugs: string[] }).skill_slugs;
      expect([...slugs].sort()).toEqual(["broken-skill", "greeting"]);
    }

    // ok 置一致（greeting × 3 工具：claude-code 本就一致、codex pending → 一致）
    expect(store.rows[0].statuses.codex).toBe("consistent");
    expect(store.rows[0].statuses.trae).toBe("consistent");
    // failed 保持原状态（broken-skill 的 claude-code 仍被工具修改）
    expect(store.rows[1].statuses["claude-code"]).toBe("modified");
    expect(store.rows[1].statuses.codex).toBe("modified");
    // 部分成功结果（结果条数据）
    expect(store.deployResult?.ok).toHaveLength(3);
    expect(store.deployResult?.failed).toHaveLength(3);
    expect(store.deploying).toBe(false);
  });

  it("分发全部：单工具一次，全集含 invalid 行", async () => {
    invokeMock.mockImplementation((cmd: string, args?: unknown) => {
      if (cmd === "get_status_matrix") {
        return Promise.resolve(matrixFixture);
      }
      if (cmd === "deploy") {
        const req = args as { tool_id: string };
        return Promise.resolve({
          ok: [{ tool_id: req.tool_id, skill_slug: "greeting" }],
          failed: [],
        });
      }
      return Promise.reject("unknown command");
    });
    const store = useMatrixStore();
    await store.loadMatrix();
    await store.deployAll("codex");
    const call = invokeMock.mock.calls.find((c) => c[0] === "deploy");
    expect(call).toBeDefined();
    expect((call![1] as { tool_id: string }).tool_id).toBe("codex");
    expect(
      (call![1] as { skill_slugs: string[] }).skill_slugs,
      "分发全部 = 矩阵全部行 slug 全集（含 invalid 行）",
    ).toEqual(["greeting", "broken-skill"]);
    expect(store.rows[0].statuses.codex).toBe("consistent");
    expect(invokeMock.mock.calls.filter((c) => c[0] === "deploy")).toHaveLength(
      1,
    );
  });

  it("分发级错误（重扫中止）→ deployError 填充、停止循环、modal 关闭复位", async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === "get_status_matrix") {
        return Promise.resolve(matrixFixture);
      }
      if (cmd === "deploy") {
        return Promise.reject({
          code: "InvalidState",
          message:
            "分发前重扫发现目标工具「claude-code」的以下 Skill 已被外部修改：\n- claude-code/greeting",
        });
      }
      return Promise.reject("unknown command");
    });
    const store = useMatrixStore();
    await store.loadMatrix();
    store.toggleSelect("greeting");
    await store.deploySelected();
    // 第一个工具即分发级失败 → 立即停止循环（只调用一次）
    expect(invokeMock.mock.calls.filter((c) => c[0] === "deploy")).toHaveLength(
      1,
    );
    expect(store.deployError).toContain("已被外部修改");
    expect(store.deployResult).toBeNull();
    expect(store.deploying).toBe(false);
    store.dismissDeployError();
    expect(store.deployError).toBeNull();
  });

  it("分发中 deploying 为 true，完成后复位", async () => {
    let resolveDeploy!: (v: unknown) => void;
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === "get_status_matrix") {
        return Promise.resolve(matrixFixture);
      }
      if (cmd === "deploy") {
        // 单工具（deployAll）单次调用：挂起观察 deploying 态
        return new Promise((r) => {
          resolveDeploy = r;
        });
      }
      return Promise.reject("unknown command");
    });
    const store = useMatrixStore();
    await store.loadMatrix();
    const pending = store.deployAll("codex");
    expect(store.deploying).toBe(true);
    resolveDeploy({ ok: [], failed: [] });
    await pending;
    expect(store.deploying).toBe(false);
  });
});
