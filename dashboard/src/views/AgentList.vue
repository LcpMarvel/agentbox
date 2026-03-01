<template>
  <div>
    <h2 class="text-2xl font-bold mb-6">Agents</h2>
    <div v-if="loading" class="text-gray-400">Loading...</div>
    <div v-else-if="agents.length === 0" class="text-gray-400">No agents registered yet.</div>
    <div v-else class="grid gap-4">
      <div v-for="agent in agents" :key="agent.id" class="card flex items-center justify-between">
        <div>
          <router-link :to="`/agents/${agent.id}`" class="text-lg font-semibold text-blue-600 hover:underline">
            {{ agent.name }}
          </router-link>
          <div class="text-sm text-gray-500 mt-0.5 font-mono">{{ agent.command }}</div>
          <div class="text-xs text-gray-400 mt-1">
            <span v-if="agent.cron_expr">⏰ {{ agent.cron_expr }}</span>
            <span v-if="agent.interval_secs"> · every {{ agent.interval_secs }}s</span>
            <span v-if="agent.last_run_at"> · last: {{ timeAgo(agent.last_run_at) }}</span>
          </div>
        </div>
        <div class="flex items-center gap-2">
          <span :class="badgeClass(agent)">{{ agent.paused ? 'paused' : agent.status }}</span>
          <button v-if="agent.paused" class="btn-primary" @click="resume(agent)">Resume</button>
          <button v-else class="btn-secondary" @click="pause(agent)">Pause</button>
          <button class="btn-primary" @click="run(agent)">▶ Run</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { api } from '../api.js'

const agents = ref([])
const loading = ref(true)

async function load() {
  agents.value = await api.agents()
  loading.value = false
}

function badgeClass(agent) {
  if (agent.paused) return 'badge-paused'
  if (agent.status === 'running') return 'badge-running'
  if (agent.status === 'error') return 'badge-error'
  if (agent.status === 'idle') return 'badge-idle'
  return 'badge-idle'
}

function timeAgo(ts) {
  const diff = Date.now() - new Date(ts).getTime()
  const mins = Math.floor(diff / 60000)
  if (mins < 1) return 'just now'
  if (mins < 60) return `${mins}m ago`
  const hours = Math.floor(mins / 60)
  if (hours < 24) return `${hours}h ago`
  return `${Math.floor(hours / 24)}d ago`
}

async function run(agent) {
  await api.run(agent.id)
  await load()
}

async function pause(agent) {
  await api.pause(agent.id)
  await load()
}

async function resume(agent) {
  await api.resume(agent.id)
  await load()
}

onMounted(load)
</script>
