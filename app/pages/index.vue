<script setup lang="ts">
import TorrentDashboard from '~/components/TorrentDashboard.vue'

const { appName } = useRuntimeConfig().public

useSeoMeta({
  title: appName,
  description: 'A focused desktop interface for qBittorrent.',
})
</script>

<template>
  <TorrentDashboard />
</template>
