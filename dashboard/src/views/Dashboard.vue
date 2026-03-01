<template>
  <div>
    <h2 class="text-2xl font-bold mb-6">Dashboard</h2>
    <div class="grid grid-cols-2 md:grid-cols-4 gap-4 mb-8">
      <div class="card text-center">
        <div class="text-3xl font-bold text-gray-900">{{ stats.total_agents ?? '-' }}</div>
        <div class="text-sm text-gray-500 mt-1">Total Agents</div>
      </div>
      <div class="card text-center">
        <div class="text-3xl font-bold text-blue-600">{{ stats.running ?? '-' }}</div>
        <div class="text-sm text-gray-500 mt-1">Running</div>
      </div>
      <div class="card text-center">
        <div class="text-3xl font-bold text-red-600">{{ stats.error ?? '-' }}</div>
        <div class="text-sm text-gray-500 mt-1">Errors</div>
      </div>
      <div class="card text-center">
        <div class="text-3xl font-bold text-green-600">{{ stats.success_rate ?? '-' }}%</div>
        <div class="text-sm text-gray-500 mt-1">Success Rate (Today)</div>
      </div>
    </div>

    <div class="card">
      <h3 class="text-lg font-semibold mb-3">Recent Activity</h3>
      <p v-if="stats.today_runs === 0" class="text-gray-400 text-sm">No runs today.</p>
      <p v-else class="text-sm text-gray-600">
        {{ stats.today_runs }} runs today, {{ stats.today_success }} succeeded.
        {{ stats.paused }} agent(s) paused.
      </p>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { api } from '../api.js'

const stats = ref({})
onMounted(async () => {
  stats.value = await api.stats()
})
</script>
