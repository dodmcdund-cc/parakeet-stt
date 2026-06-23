<script setup lang="ts">
defineProps<{
  level: number;
}>();
</script>

<template>
  <div class="audio-level-meter">
    <div class="level-track">
      <div
        class="level-fill"
        :style="{ width: `${Math.min(Math.max(level, 0), 1) * 100}%` }"
      />
    </div>
  </div>
</template>

<style scoped>
.audio-level-meter {
  width: 100%;
  padding: 4px 0;
}

.level-track {
  height: 6px;
  background: #333;
  border-radius: 3px;
  overflow: hidden;
}

.level-fill {
  height: 100%;
  background: linear-gradient(90deg, #4caf50, #ff9800, #f44336);
  border-radius: 3px;
  transition: width 80ms linear;
}
</style>
