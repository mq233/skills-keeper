// api 层测试：命令契约对接 + 统一错误解析（{code, message} → 抛带 code 的 Error）。

import { invoke } from "@tauri-apps/api/core";
import { describe, expect, it, vi } from "vitest";

import {
  deploy,
  getStatusMatrix,
  listSkills,
  scan,
  type ApiError,
  type DeployResult,
} from "../api";
import { matrixFixture } from "./fixtures";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const invokeMock = vi.mocked(invoke);

describe("api 封装", () => {
  it("getStatusMatrix 调用 get_status_matrix 并返回契约数据", async () => {
    invokeMock.mockResolvedValue(matrixFixture);
    await expect(getStatusMatrix()).resolves.toEqual(matrixFixture);
    expect(invokeMock).toHaveBeenCalledWith("get_status_matrix");
  });

  it("scan 调用 scan 命令并返回最新矩阵", async () => {
    invokeMock.mockResolvedValue(matrixFixture);
    await expect(scan()).resolves.toEqual(matrixFixture);
    expect(invokeMock).toHaveBeenCalledWith("scan");
  });

  it("listSkills 调用 list_skills", async () => {
    invokeMock.mockResolvedValue(matrixFixture.rows.map((r) => r.skill));
    await expect(listSkills()).resolves.toHaveLength(2);
    expect(invokeMock).toHaveBeenCalledWith("list_skills");
  });

  it("契约错误 {code, message} → 抛带 code 的 Error，message 中文直传", async () => {
    invokeMock.mockRejectedValue({ code: "Io", message: "读取文件失败：xxx" });
    await expect(getStatusMatrix()).rejects.toSatisfy((e: ApiError) => {
      expect(e).toBeInstanceOf(Error);
      expect(e.code).toBe("Io");
      expect(e.message).toBe("读取文件失败：xxx");
      return true;
    });
  });

  it("非契约错误（invoke 自身异常）→ 兜底中文文案 + Internal", async () => {
    invokeMock.mockRejectedValue("command not found");
    await expect(scan()).rejects.toSatisfy((e: ApiError) => {
      expect(e.code).toBe("Internal");
      expect(e.message).toBe("操作失败，请重试");
      return true;
    });
  });

  it("deploy 调用 deploy 命令并透传 {tool_id, skill_slugs} 参数（S2 契约）", async () => {
    const result: DeployResult = {
      ok: [{ tool_id: "codex", skill_slug: "greeting" }],
      failed: [
        {
          tool_id: "codex",
          skill_slug: "broken",
          code: "InvalidSkill",
          message: "缺少 description",
        },
      ],
    };
    invokeMock.mockResolvedValue(result);
    await expect(
      deploy({ tool_id: "codex", skill_slugs: ["greeting", "broken"] }),
    ).resolves.toEqual(result);
    expect(invokeMock).toHaveBeenCalledWith("deploy", {
      tool_id: "codex",
      skill_slugs: ["greeting", "broken"],
    });
  });

  it("deploy 分发级错误（重扫中止 InvalidState）→ 抛带 code 的 Error", async () => {
    invokeMock.mockRejectedValue({
      code: "InvalidState",
      message:
        "分发前重扫发现目标工具「codex」的以下 Skill 已被外部修改：\n- codex/greeting",
    });
    await expect(
      deploy({ tool_id: "codex", skill_slugs: ["greeting"] }),
    ).rejects.toSatisfy((e: ApiError) => {
      expect(e.code).toBe("InvalidState");
      expect(e.message).toContain("已被外部修改");
      return true;
    });
  });
});
