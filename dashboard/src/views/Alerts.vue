<template>
  <div>
    <h2 class="text-2xl font-bold mb-6">Alerts</h2>
    <div v-if="alerts.length === 0" class="text-gray-400">No alerts yet.</div>
    <div v-else class="space-y-3">
      <div v-for="alert in alerts" :key="alert.id" class="card flex items-start gap-3">
        <span class="text-lg">{{ alert.alert_type === 'failure' ? '🔴' : alert.alert_type === 'recovery' ? '🟢' : '⏰' }}</span>
        <div>
          <div class="text-sm font-medium">{{ alert.alert_type }} — Agent #{{ alert.agent_id }}</div>
          <div class="text-xs text-gray-500">{{ alert.channel }} · {{ formatTime(alert.created_at) }}</div>
          <div v-if="alert.message" class="text-xs text-gray-600 mt-1">{{ alert.message }}</div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { api } from '../api.js'

const alerts = ref([])

onMounted(async () => {
  alerts.value = await api.alerts()
})

function formatTime(ts) {
  if (!ts) return ''
  return new Date(ts).toLocaleString()
}
</script>
