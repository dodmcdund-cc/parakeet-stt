<script setup lang="ts">
import { useAppStore } from '../stores/app';
import StatusIndicator from '../components/StatusIndicator.vue';
import AudioLevelMeter from '../components/AudioLevelMeter.vue';

const store = useAppStore();

const langLabels: Record<string, string> = {
  en: 'EN', fr: 'FR', es: 'ES', de: 'DE',
  it: 'IT', pt: 'PT', ja: 'JA', zh: 'ZH', ko: 'KO',
};

function copyText() {
  if (store.lastTranscription) {
    navigator.clipboard.writeText(store.lastTranscription);
  }
}
</script>

<template>
  <div class="menubar-view">
    <header class="header">
      <StatusIndicator :status="store.status" />
      <span class="hotkey-hint">Ctrl+Shift+R</span>
    </header>

    <section class="transcription">
      <p v-if="store.lastTranscription" class="text">
        {{ store.lastTranscription }}
      </p>
      <p v-else class="placeholder">
        Press the hotkey to start dictation.
      </p>
    </section>

    <AudioLevelMeter
      v-if="store.status === 'listening'"
      :level="store.audioLevel"
    />

    <footer class="footer">
      <span
        v-if="store.lastLanguage"
        class="lang-badge"
      >
        {{ langLabels[store.lastLanguage] || store.lastLanguage.toUpperCase() }}
      </span>
      <div class="actions">
        <button
          v-if="store.lastTranscription"
          class="action-btn"
          @click="copyText"
        >
          Copy
        </button>
        <button
          class="action-btn"
          @click="$emit('open-settings')"
        >
          Settings
        </button>
        <button
          class="action-btn quit"
          @click="$emit('quit')"
        >
          Quit
        </button>
      </div>
    </footer>
  </div>
</template>

<script lang="ts">
export default {
  emits: ['open-settings', 'quit'],
};
</script>

<style scoped>
.menubar-view {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: #1a1a1a;
  color: #eee;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  padding: 12px 16px;
  box-sizing: border-box;
  user-select: none;
}

.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}

.hotkey-hint {
  font-size: 11px;
  color: #555;
  font-family: monospace;
}

.transcription {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 60px;
}

.text {
  font-size: 14px;
  line-height: 1.5;
  text-align: center;
  word-break: break-word;
  margin: 0;
}

.placeholder {
  font-size: 12px;
  color: #555;
  text-align: center;
  margin: 0;
}

.footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-top: 10px;
  padding-top: 8px;
  border-top: 1px solid #2a2a2a;
}

.lang-badge {
  font-size: 11px;
  background: #333;
  padding: 2px 8px;
  border-radius: 4px;
  color: #aaa;
  font-weight: 600;
}

.actions {
  display: flex;
  gap: 6px;
}

.action-btn {
  padding: 4px 10px;
  border: 1px solid #444;
  border-radius: 4px;
  background: #222;
  color: #bbb;
  cursor: pointer;
  font-size: 11px;
}

.action-btn:hover {
  border-color: #777;
  color: #eee;
  background: #2a2a2a;
}

.action-btn.quit:hover {
  border-color: #f44336;
  color: #f44336;
}
</style>
