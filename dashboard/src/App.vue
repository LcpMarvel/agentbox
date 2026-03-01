<template>
  <div class="min-h-screen bg-gray-50">
    <nav class="bg-white border-b border-gray-200 px-6 py-3 flex items-center gap-6">
      <h1 class="text-lg font-bold text-gray-900">📦 AgentBox</h1>
      <router-link to="/" class="text-sm text-gray-600 hover:text-gray-900" active-class="text-blue-600 font-medium">Dashboard</router-link>
      <router-link to="/agents" class="text-sm text-gray-600 hover:text-gray-900" active-class="text-blue-600 font-medium">Agents</router-link>
      <router-link to="/alerts" class="text-sm text-gray-600 hover:text-gray-900" active-class="text-blue-600 font-medium">Alerts</router-link>
    </nav>
    <main class="max-w-6xl mx-auto p-6">
      <router-view />
    </main>
  </div>
</template>
