<script setup lang="ts">
const { loadMicaSetting } = useMicaSetting()

onMounted(() => {
  loadMicaSetting()
})
</script>

<template>
  <UApp>
    <NuxtPage />
  </UApp>
</template>
