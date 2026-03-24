<template>
  <a-layout class="app-layout">
    <a-layout-sider
      :collapsed="appStore.sidebarCollapsed"
      collapsible
      breakpoint="lg"
      @collapse="appStore.toggleSidebar"
      :width="220"
      :collapsed-width="48"
    >
      <div class="logo">
        <span v-if="!appStore.sidebarCollapsed">Workflow</span>
        <span v-else>W</span>
      </div>
      <sidebar-menu />
    </a-layout-sider>
    <a-layout>
      <a-layout-header class="app-header">
        <header-bar />
      </a-layout-header>
      <a-layout-content class="app-content">
        <router-view />
      </a-layout-content>
    </a-layout>
  </a-layout>
</template>

<script setup lang="ts">
import { useAppStore } from '../../stores/app'
import SidebarMenu from './sidebar-menu.vue'
import HeaderBar from './header-bar.vue'

const appStore = useAppStore()
</script>

<style scoped>
.app-layout {
  height: 100vh;
}
.logo {
  height: 48px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 18px;
  font-weight: 700;
  color: var(--color-text-1);
  border-bottom: 1px solid var(--color-border);
}
.app-header {
  display: flex;
  align-items: center;
  padding: 0 20px;
  height: 48px;
  border-bottom: 1px solid var(--color-border);
  background: var(--color-bg-2);
}
.app-content {
  padding: 20px;
  overflow-y: auto;
  background: var(--color-fill-2);
}
</style>
