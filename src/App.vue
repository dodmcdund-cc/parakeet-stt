<script setup lang="ts">
import { ref } from "vue";
import MenuBarView from "./views/MenuBarView.vue";
import SettingsView from "./views/SettingsView.vue";

const params = new URLSearchParams(window.location.search);
const view = ref(params.get("view") || "menu");
</script>

<template>
  <MenuBarView
    v-if="view === 'menu'"
    @open-settings="view = 'settings'"
  />
  <SettingsView v-else-if="view === 'settings'" />
</template>
