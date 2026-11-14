<script setup lang="ts">
useSystemColors()
const { loadAppearanceSetting } = useAppearanceSetting()

onMounted(() => {
  loadAppearanceSetting()
})
</script>

<template>
  <UApp>
    <NuxtPage />
  </UApp>
</template>
