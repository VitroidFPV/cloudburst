<script setup lang="ts">
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
