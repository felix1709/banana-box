<script setup lang="ts">
import { createPinia, setActivePinia } from 'pinia'
import App from '@/App.vue'

setActivePinia(createPinia())
</script>

<template>
  <App />
</template>
