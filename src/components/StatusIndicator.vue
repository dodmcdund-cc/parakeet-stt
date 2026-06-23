<script setup lang="ts">
import type { AppStatus } from '../stores/app';

defineProps<{
  status: AppStatus;
}>();
</script>

<template>
  <div class="status-indicator">
    <span class="status-dot" :class="status" />
    <span class="status-label">{{ status }}</span>
  </div>
</template>

<style scoped>
.status-indicator {
  display: flex;
  align-items: center;
  gap: 8px;
}

.status-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
}

.status-dot.idle {
  background: #666;
}

.status-dot.listening {
  background: #4caf50;
  animation: pulse 1.5s ease-in-out infinite;
}

.status-dot.processing {
  background: #ff9800;
  animation: pulse 1s ease-in-out infinite;
}

.status-dot.error {
  background: #f44336;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}

.status-label {
  font-size: 12px;
  color: #888;
  text-transform: capitalize;
}
</style>
