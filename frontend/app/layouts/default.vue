<template>
  <div class="min-h-screen flex flex-col">

    <!-- Header -->
    <header class="w-full h-10 bg-base-200 flex items-center px-6 shadow fixed top-0 left-0 z-50">

      <!-- Title -->
      <div class="font-bold text-lg mr-10">
        ROSBag Database Manager
      </div>

      <!-- Navigation -->
      <nav class="flex gap-6">
        <NuxtLink
          to="/"
          class="tab"
          :class="{ 'tab-active': route.path === '/' }"
        >
          Table
        </NuxtLink>

        <NuxtLink
          to="/plugins"
          class="tab"
          :class="{ 'tab-active': route.path === '/plugins' }"
        >
          Plugins
        </NuxtLink>

        <NuxtLink
          to="/logs"
          class="tab"
          :class="{ 'tab-active': route.path === '/logs' }"
        >
          Logs
        </NuxtLink>
      </nav>

    </header>

    <!-- Content -->
    <main class="flex-1 pt-12">
      <slot />
    </main>

  </div>
</template>

<script setup lang="ts">
const route = useRoute()
</script>
