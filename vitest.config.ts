import vue from "@vitejs/plugin-vue";
import { defineConfig } from "vitest/config";

// Vitest 配置：组件测试用 happy-dom 环境（spec §11 依赖清单）
export default defineConfig({
  plugins: [vue()],
  test: {
    environment: "happy-dom",
  },
});
