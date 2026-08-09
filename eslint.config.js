// ESLint flat config——代码质量规则（社区 recommended 全开），格式统一交给 Prettier。
// 规则集：typescript-eslint recommended + eslint-plugin-vue recommended（docs/technical-plan.md §7）。
import tseslint from "typescript-eslint";
import pluginVue from "eslint-plugin-vue";
import importX from "eslint-plugin-import-x";
import prettier from "eslint-config-prettier";

export default [
  // 忽略：构建产物与 Rust 侧（Rust 走 rustfmt + clippy）
  { ignores: ["dist/**", "node_modules/**", "public/**", "src-tauri/**"] },

  // TS + Vue 双引擎，按官方 flat 推荐顺序组合
  ...tseslint.configs.recommended,
  ...pluginVue.configs["flat/recommended"],

  // 全局编码规范：导入排序（第三方 → 内部 → 同级/父级 → 样式，同类按字母序）
  {
    files: ["**/*.{ts,vue}"],
    plugins: { "import-x": importX },
    rules: {
      "import-x/order": [
        "error",
        {
          "newlines-between": "always",
          alphabetize: { order: "asc", caseInsensitive: true },
          groups: [
            ["builtin", "external"],
            "internal",
            ["parent", "sibling", "index"],
          ],
          pathGroups: [
            {
              pattern: "**/*.{css,scss,less}",
              group: "sibling",
              position: "after",
            },
          ],
        },
      ],
    },
  },

  // 模板根组件 App.vue 为单字组件名（Vue 官方 SFC 入口惯例），豁免 multi-word 校验
  {
    files: ["**/*.vue"],
    rules: { "vue/multi-word-component-names": "off" },
  },

  // 关闭与 Prettier 冲突的格式类规则，格式由 prettier --check 独立把关
  prettier,
];
