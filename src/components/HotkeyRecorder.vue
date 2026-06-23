<script setup lang="ts">
import { ref } from 'vue';

defineProps<{
  modelValue: string;
}>();

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void;
}>();

const recording = ref(false);
const keys = ref<string[]>([]);

const modifierMap: Record<string, string> = {
  Control: 'Ctrl',
  Meta: 'Cmd',
  Alt: 'Alt',
  Shift: 'Shift',
};

function start() {
  recording.value = true;
  keys.value = [];
}

function stop() {
  recording.value = false;
  if (keys.value.length > 0) {
    emit('update:modelValue', keys.value.join('+'));
  }
  keys.value = [];
}

function onKeydown(e: KeyboardEvent) {
  if (!recording.value) return;
  e.preventDefault();
  e.stopPropagation();

  const k = e.key;
  if (modifierMap[k]) {
    const m = modifierMap[k];
    if (!keys.value.includes(m)) keys.value.push(m);
  } else if (k === ' ') {
    // space alone is not a shortcut
  } else {
    keys.value.push(k === ' ' ? 'Space' : k);
    stop();
  }
}

function onKeyup(e: KeyboardEvent) {
  if (!recording.value) return;
  e.preventDefault();
  e.stopPropagation();
}
</script>

<template>
  <div
    class="hotkey-recorder"
    tabindex="0"
    @keydown="onKeydown"
    @keyup="onKeyup"
  >
    <button
      v-if="!recording"
      class="hk-btn"
      @click="start"
    >
      {{ modelValue || 'Set shortcut' }}
    </button>
    <div v-else class="hk-recording">
      Press shortcut...
    </div>
  </div>
</template>

<style scoped>
.hotkey-recorder {
  outline: none;
}

.hk-btn {
  padding: 4px 12px;
  border: 1px solid #555;
  border-radius: 4px;
  background: #222;
  color: #ccc;
  cursor: pointer;
  font-size: 12px;
}

.hk-btn:hover {
  border-color: #888;
}

.hk-recording {
  padding: 4px 12px;
  border: 1px solid #4caf50;
  border-radius: 4px;
  background: #1a3a1a;
  color: #4caf50;
  font-size: 12px;
  animation: hk-blink 1s step-end infinite;
}

@keyframes hk-blink {
  50% { opacity: 0.5; }
}
</style>
