// vue-router 四页导航：Skill 库实现，导入 / 快照时间线 / 设置占位（spec §10）。
// Tauri 打包为 file 协议加载，用 hash history 最稳。

import { createRouter, createWebHashHistory } from "vue-router";

import SkillLibraryView from "../views/SkillLibraryView.vue";

export const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: "/", redirect: "/skills" },
    {
      path: "/skills",
      name: "skills",
      component: SkillLibraryView,
      meta: { title: "Skill 库" },
    },
    {
      path: "/import",
      name: "import",
      component: () => import("../views/ImportView.vue"),
      meta: { title: "导入" },
    },
    {
      path: "/snapshots",
      name: "snapshots",
      component: () => import("../views/SnapshotTimelineView.vue"),
      meta: { title: "快照时间线" },
    },
    {
      path: "/settings",
      name: "settings",
      component: () => import("../views/SettingsView.vue"),
      meta: { title: "设置" },
    },
  ],
});
