<script setup lang="ts">
import MainApp from "./views/MainApp.vue";
import ReminderOverlay from "./views/ReminderOverlay.vue";

const isOverlay = window.location.hash.includes("overlay");
</script>

<template>
  <ReminderOverlay v-if="isOverlay" />
  <MainApp v-else />
</template>

