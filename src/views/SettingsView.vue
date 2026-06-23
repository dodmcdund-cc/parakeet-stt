<script setup lang="ts">
import { ref } from 'vue';
import { useAppStore } from '../stores/app';
import HotkeyRecorder from '../components/HotkeyRecorder.vue';
import { invoke } from '@tauri-apps/api/core';

const store = useAppStore();
const activeTab = ref(0);
const hotkey = ref('Ctrl+Shift+R');

const tabs = ['General', 'Audio', 'Transcription', 'About'] as const;

type ToggleKey = 'launchAtStartup' | 'minimizeToTray';

function toggle(key: ToggleKey) {
  store.updateSettings({ [key]: !store.settings[key] });
}

function quitApp() {
  invoke('quit_app');
}
</script>

<template>
  <div class="settings-view">
    <header class="header">
      <h1>Settings</h1>
    </header>

    <nav class="tabs">
      <button
        v-for="(tab, i) in tabs"
        :key="tab"
        :class="['tab', { active: activeTab === i }]"
        @click="activeTab = i"
      >
        {{ tab }}
      </button>
    </nav>

    <div class="tab-content">
      <!-- General -->
      <div v-if="activeTab === 0" class="tab-pane">
        <div class="setting-row">
          <label>Launch at startup</label>
          <button
            :class="['toggle', { on: store.settings.launchAtStartup }]"
            @click="toggle('launchAtStartup')"
          >
            {{ store.settings.launchAtStartup ? 'On' : 'Off' }}
          </button>
        </div>
        <div class="setting-row">
          <label>Minimize to tray</label>
          <button
            :class="['toggle', { on: store.settings.minimizeToTray }]"
            @click="toggle('minimizeToTray')"
          >
            {{ store.settings.minimizeToTray ? 'On' : 'Off' }}
          </button>
        </div>
        <div class="setting-row">
          <label>Global hotkey</label>
          <HotkeyRecorder v-model="hotkey" />
        </div>
      </div>

      <!-- Audio -->
      <div v-if="activeTab === 1" class="tab-pane">
        <div class="setting-row">
          <label>Input device</label>
          <input
            v-model="store.settings.inputDevice"
            type="text"
            class="text-input"
            placeholder="default"
          />
        </div>
        <div class="setting-row">
          <label>Silence threshold</label>
          <div class="range-group">
            <input
              v-model.number="store.settings.silenceThreshold"
              type="range"
              min="0"
              max="1"
              step="0.05"
            />
            <span class="range-val">{{ store.settings.silenceThreshold.toFixed(2) }}</span>
          </div>
        </div>
      </div>

      <!-- Transcription -->
      <div v-if="activeTab === 2" class="tab-pane">
        <div class="setting-row">
          <label>Language</label>
          <select v-model="store.settings.language" class="select-input">
            <option value="en">English</option>
            <option value="fr">French</option>
            <option value="es">Spanish</option>
            <option value="de">German</option>
            <option value="it">Italian</option>
            <option value="pt">Portuguese</option>
            <option value="ja">Japanese</option>
            <option value="zh">Chinese</option>
            <option value="ko">Korean</option>
          </select>
        </div>
        <div class="setting-row">
          <label>Model size</label>
          <select v-model="store.settings.modelSize" class="select-input">
            <option value="tiny">Tiny</option>
            <option value="small">Small</option>
            <option value="medium">Medium</option>
            <option value="large">Large</option>
          </select>
        </div>
        <div class="setting-row">
          <label>Server URL</label>
          <input
            v-model="store.settings.serverUrl"
            type="text"
            class="text-input"
            placeholder="http://localhost:8080"
          />
        </div>
      </div>

      <!-- About -->
      <div v-if="activeTab === 3" class="tab-pane about-pane">
        <div class="app-info">
          <h2>ParakeetSTT</h2>
          <p class="version">v0.1.0</p>
          <p class="desc">Speech-to-text desktop application.</p>
          <p class="tech">Built with Tauri &bull; Vue 3 &bull; Rust</p>
        </div>
      </div>
    </div>

    <footer class="footer">
      <button class="quit-btn" @click="quitApp">Quit</button>
    </footer>
  </div>
</template>

<style scoped>
.settings-view {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: #1a1a1a;
  color: #eee;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  padding: 16px;
  box-sizing: border-box;
}

.header h1 {
  margin: 0 0 16px;
  font-size: 18px;
  font-weight: 600;
}

.tabs {
  display: flex;
  gap: 2px;
  margin-bottom: 16px;
  border-bottom: 1px solid #2a2a2a;
}

.tab {
  padding: 8px 16px;
  border: none;
  background: none;
  color: #666;
  cursor: pointer;
  font-size: 13px;
  border-bottom: 2px solid transparent;
  transition: color 0.15s, border-color 0.15s;
}

.tab.active {
  color: #eee;
  border-bottom-color: #4caf50;
}

.tab:hover:not(.active) {
  color: #999;
}

.tab-content {
  flex: 1;
  overflow-y: auto;
}

.tab-pane {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.setting-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.setting-row label {
  font-size: 13px;
  color: #ccc;
}

.toggle {
  padding: 4px 12px;
  border: 1px solid #444;
  border-radius: 4px;
  background: #222;
  color: #888;
  cursor: pointer;
  font-size: 12px;
  min-width: 48px;
  transition: all 0.15s;
}

.toggle.on {
  background: #1a3a1a;
  border-color: #4caf50;
  color: #4caf50;
}

.text-input,
.select-input {
  padding: 4px 8px;
  border: 1px solid #444;
  border-radius: 4px;
  background: #222;
  color: #ccc;
  font-size: 12px;
  max-width: 200px;
  outline: none;
}

.text-input:focus,
.select-input:focus {
  border-color: #4caf50;
}

.range-group {
  display: flex;
  align-items: center;
  gap: 8px;
}

.range-val {
  font-size: 12px;
  color: #aaa;
  min-width: 32px;
  text-align: right;
}

.about-pane {
  flex: 1;
  display: flex;
  justify-content: center;
  align-items: center;
}

.app-info {
  text-align: center;
}

.app-info h2 {
  margin: 0 0 6px;
  font-size: 20px;
  font-weight: 600;
}

.app-info .version {
  font-size: 12px;
  color: #666;
  margin: 0 0 10px;
}

.app-info .desc {
  font-size: 13px;
  color: #aaa;
  margin: 0 0 4px;
}

.app-info .tech {
  font-size: 11px;
  color: #555;
  margin: 0;
}

.footer {
  padding-top: 12px;
  border-top: 1px solid #2a2a2a;
  display: flex;
  justify-content: flex-end;
}

.quit-btn {
  padding: 6px 16px;
  border: 1px solid #444;
  border-radius: 4px;
  background: #222;
  color: #bbb;
  cursor: pointer;
  font-size: 12px;
}

.quit-btn:hover {
  border-color: #f44336;
  color: #f44336;
}
</style>
