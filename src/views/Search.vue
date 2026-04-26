<script setup>
import SearchPanel from '../components/SearchPanel.vue';
</script>

<template>
  <main class="inline-search-page">
    <SearchPanel mode="inline" />
  </main>
</template>

<style scoped>
.inline-search-page {
  height: calc(100vh - 30px);
  min-height: 0;
  overflow: hidden;
  background: var(--st-content-bg);
}
</style>
