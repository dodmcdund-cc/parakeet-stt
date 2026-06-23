import { defineStore } from 'pinia';
import { ref } from 'vue';

export type AppStatus = 'idle' | 'listening' | 'processing' | 'error';

export interface AppSettings {
  launchAtStartup: boolean;
  minimizeToTray: boolean;
  inputDevice: string;
  silenceThreshold: number;
  language: string;
  modelSize: string;
  serverUrl: string;
}

export const useAppStore = defineStore('app', () => {
  const status = ref<AppStatus>('idle');
  const audioLevel = ref(0);
  const lastTranscription = ref('');
  const lastLanguage = ref('en');
  const serverReachable = ref(false);

  const settings = ref<AppSettings>({
    launchAtStartup: false,
    minimizeToTray: true,
    inputDevice: 'default',
    silenceThreshold: 0.5,
    language: 'en',
    modelSize: 'small',
    serverUrl: 'http://localhost:8080',
  });

  function setStatus(s: AppStatus) {
    status.value = s;
  }

  function setAudioLevel(level: number) {
    audioLevel.value = level;
  }

  function setTranscription(text: string, lang: string) {
    lastTranscription.value = text;
    lastLanguage.value = lang;
  }

  function setServerReachable(reachable: boolean) {
    serverReachable.value = reachable;
  }

  function updateSettings(partial: Partial<AppSettings>) {
    settings.value = { ...settings.value, ...partial };
  }

  return {
    status,
    audioLevel,
    lastTranscription,
    lastLanguage,
    serverReachable,
    settings,
    setStatus,
    setAudioLevel,
    setTranscription,
    setServerReachable,
    updateSettings,
  };
});
