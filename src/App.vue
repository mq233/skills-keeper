<script setup lang="ts">
import { RouterLink, RouterView } from "vue-router";

// 左侧栏四页导航（S1：Skill 库实现，其余占位），见 docs/technical-plan.md §6.1
const navItems = [
  { to: "/skills", label: "Skill 库" },
  { to: "/import", label: "导入" },
  { to: "/snapshots", label: "快照时间线" },
  { to: "/settings", label: "设置" },
];
</script>

<template>
  <div class="app">
    <nav class="sidebar" aria-label="主导航">
      <h1 class="brand">Skills Keeper</h1>
      <RouterLink
        v-for="item in navItems"
        :key="item.to"
        :to="item.to"
        class="nav-link"
      >
        {{ item.label }}
      </RouterLink>
    </nav>
    <main class="content">
      <RouterView />
    </main>
  </div>
</template>

<style>
:root {
  font-family:
    "Segoe UI", "PingFang SC", "Microsoft YaHei", Avenir, Helvetica, Arial,
    sans-serif;
  font-size: 15px;
  line-height: 1.6;
  color: #1f2328;
  background-color: #f6f8fa;
  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  -webkit-text-size-adjust: 100%;
}

* {
  box-sizing: border-box;
}

body {
  margin: 0;
}

.app {
  display: flex;
  min-height: 100vh;
}

.sidebar {
  flex: 0 0 200px;
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 16px 12px;
  background: #ffffff;
  border-right: 1px solid #e2e5e9;
}

.brand {
  font-size: 15px;
  font-weight: 600;
  margin: 0 8px 12px;
  color: #373c43;
}

.nav-link {
  padding: 7px 10px;
  border-radius: 6px;
  color: #57606a;
  text-decoration: none;
  font-size: 14px;
}

.nav-link:hover {
  background: #f0f2f5;
  color: #1f2328;
}

.nav-link.router-link-active {
  background: #e8edf5;
  color: #1a4fd8;
  font-weight: 500;
}

.content {
  flex: 1;
  padding: 24px 32px;
  min-width: 0;
}

@media (prefers-color-scheme: dark) {
  :root {
    color: #e6e8eb;
    background-color: #1b1e23;
  }

  .sidebar {
    background: #22262c;
    border-right-color: #33383f;
  }

  .brand {
    color: #d4d7dc;
  }

  .nav-link {
    color: #a8adb5;
  }

  .nav-link:hover {
    background: #2c3138;
    color: #e6e8eb;
  }

  .nav-link.router-link-active {
    background: #2b3a55;
    color: #7db3ff;
  }
}
</style>
