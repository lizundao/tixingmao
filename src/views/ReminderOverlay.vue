<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { Reminder } from "../types";

const reminder = ref<Reminder | null>(null);
const isPaused = ref(false);
const closing = ref(false);
let unlisten: UnlistenFn | undefined;

function pause() {
  isPaused.value = true;
}

function resume() {
  isPaused.value = false;
}

async function closeReminder() {
  if (!reminder.value || closing.value) return;
  closing.value = true;
  try {
    await invoke("dismiss_overlay", { id: reminder.value.id });
  } finally {
    closing.value = false;
  }
}

onMounted(async () => {
  unlisten = await listen<Reminder>("reminder-due", (event) => {
    reminder.value = event.payload;
    isPaused.value = false;
  });
  const active = await invoke<Reminder | null>("get_active_overlay");
  if (active && !reminder.value) reminder.value = active;
});

onUnmounted(() => {
  if (unlisten) unlisten();
});
</script>

<template>
  <div
    v-if="reminder"
    class="overlay-shell"
    :class="{ 'is-paused': isPaused }"
    role="alertdialog"
    aria-live="assertive"
    @mouseenter="pause"
    @mouseleave="resume"
  >
    <div class="overlay-scene" aria-hidden="true">
      <svg class="cat-illustration" viewBox="0 0 280 280">
        <ellipse cx="135" cy="249" rx="78" ry="14" fill="rgba(31, 22, 17, 0.16)" />

        <g class="cat-tail">
          <path
            d="M197 199c37-22 64 0 48 27-11 18-28 18-38 9"
            fill="none"
            stroke="#3a3430"
            stroke-width="17"
            stroke-linecap="round"
          />
          <path
            d="M197 199c37-22 64 0 48 27-11 18-28 18-38 9"
            fill="none"
            stroke="#f8f0e4"
            stroke-width="11"
            stroke-linecap="round"
          />
        </g>

        <g class="cat-body">
          <ellipse cx="138" cy="204" rx="67" ry="55" fill="#f8f0e4" stroke="#3a3430" stroke-width="5" />
          <path d="M106 158c7 23 8 47 7 69M169 159c-6 24-6 48-5 68" fill="none" stroke="#ded0bf" stroke-width="4" stroke-linecap="round" />
        </g>

        <g class="cat-paw">
          <ellipse cx="92" cy="222" rx="21" ry="13" fill="#fbf6ee" stroke="#3a3430" stroke-width="4" />
          <path d="M83 220h1M92 218h1M101 220h1" stroke="#a49687" stroke-width="2.5" stroke-linecap="round" />
        </g>
        <ellipse cx="177" cy="228" rx="22" ry="13" fill="#fbf6ee" stroke="#3a3430" stroke-width="4" />

        <g class="cat-head">
          <path d="M83 135 75 77l48 30c6-2 12-3 18-3s12 1 18 3l48-30-8 58c11 15 17 31 17 49 0 37-34 58-75 58s-76-21-76-58c0-18 6-34 18-49Z" fill="#fbf6ee" stroke="#3a3430" stroke-width="5" stroke-linejoin="round" />
          <path d="M83 135 75 77l48 30" fill="#edc8bd" opacity="0.65" />
          <path d="M207 135l8-58-48 30" fill="#edc8bd" opacity="0.65" />
          <path d="M105 96c8 6 17 10 27 12M181 108c11-2 20-6 28-12" fill="none" stroke="#e3d6c7" stroke-width="4" stroke-linecap="round" />

          <g class="cat-eyes">
            <ellipse cx="106" cy="150" rx="10" ry="13" fill="#3a3430" />
            <ellipse cx="171" cy="150" rx="10" ry="13" fill="#3a3430" />
            <circle cx="109" cy="145" r="3" fill="#fff" />
            <circle cx="174" cy="145" r="3" fill="#fff" />
          </g>

          <ellipse cx="96" cy="173" rx="14" ry="7" fill="#f3c7bf" opacity="0.65" />
          <ellipse cx="182" cy="173" rx="14" ry="7" fill="#f3c7bf" opacity="0.65" />
          <path d="M132 168l8 7-8 7-8-7 8-7Z" fill="#d88d86" />
          <path d="M139 181c-4 7-12 11-20 5M139 181c4 7 12 11 20 5" fill="none" stroke="#5a4b41" stroke-width="3" stroke-linecap="round" />
          <path d="M87 170l-28-5M88 179l-28 5M189 170l28-5M188 179l28 5" fill="none" stroke="#8d7d70" stroke-width="2.5" stroke-linecap="round" />
        </g>

        <g class="cat-bell">
          <path d="M110 194c5 9 11 14 24 17M166 194c-5 9-11 14-24 17" fill="none" stroke="#d17d68" stroke-width="5" stroke-linecap="round" />
          <circle cx="138" cy="214" r="12" fill="#e7b24b" stroke="#3a3430" stroke-width="3.5" />
          <circle cx="138" cy="211" r="2.5" fill="#7d5b1d" />
          <path d="M138 218v4" stroke="#7d5b1d" stroke-width="2.5" stroke-linecap="round" />
        </g>
      </svg>
    </div>

    <section class="notice-card">
      <button
        type="button"
        class="close-button"
        :disabled="closing"
        aria-label="关闭提醒"
        @click="closeReminder"
      >
        ×
      </button>
      <p class="notice-eyebrow">提醒猫来啦</p>
      <h2 class="notice-title">{{ reminder.title }}</h2>
      <p v-if="reminder.message" class="notice-message">{{ reminder.message }}</p>
    </section>

    <span class="hover-hint">移开鼠标继续，点 × 关闭</span>
  </div>
</template>

