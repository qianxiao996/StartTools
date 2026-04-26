<script setup>
import '../css/white.css';
import SearchPanel from '../components/SearchPanel.vue';
</script>

<template>
  <main class="global-search-page">
    <SearchPanel mode="global" />
  </main>
</template>

<style scoped>
.global-search-page {
  width: 100vw;
  height: auto;
  min-height: 0;
  margin: 0;
  padding: 0;
  overflow: hidden;
  background: transparent;
}

:global(html),
:global(body) {
  background: transparent !important;
}

:global(#app) {
  background: var(--st-content-bg) !important;
}

:global(.global-search-page .search-tools-content),
:global(.global-search-page .search-tool-dropdown),
:global(.global-search-page .search-tool-button) {
  background: #ffffff !important;
  background-color: #ffffff !important;
}

:global(html.dark .global-search-page .search-tools-content),
:global(html.dark .global-search-page .search-tool-dropdown),
:global(html.dark .global-search-page .search-tool-button) {
  background: #15181d !important;
  background-color: #15181d !important;
}

:global(.global-search-page .search-tool-button:hover) {
  background: #f4f7fb !important;
  background-color: #f4f7fb !important;
}

:global(html.dark .global-search-page .search-tool-button:hover) {
  background: #222832 !important;
  background-color: #222832 !important;
}

:global(.global-search-page .search-tool-button.is-selected) {
  background: #edf4ff !important;
  background-color: #edf4ff !important;
}

:global(html.dark .global-search-page .search-tool-button.is-selected) {
  background: #243246 !important;
  background-color: #243246 !important;
}
</style>
