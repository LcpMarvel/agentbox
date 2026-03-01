<template>
  <div>
    <router-link to="/agents" class="text-sm text-gray-500 hover:text-gray-700 mb-4 inline-block">← Back to Agents</router-link>

    <div v-if="!agent" class="text-gray-400">Loading...</div>
    <template v-else>
      <div class="flex items-center gap-3 mb-6">
        <h2 class="text-2xl font-bold">{{ agent.name }}</h2>
        <span :class="agent.paused ? 'badge-paused' : agent.status === 'error' ? 'badge-error' : agent.status === 'running' ? 'badge-running' : 'badge-idle'">
          {{ agent.paused ? 'paused' : agent.status }}
        </span>
      </div>

      <div class="card mb-6">
        <h3 class="text-sm font-semibold text-gray-500 mb-2">Configuration</h3>
        <div class="text-sm space-y-1">
          <div><span class="text-gray-500">Command:</span> <code class="bg-gray-100 px-1 rounded">{{ agent.command }}</code></div>
          <div v-if="agent.cron_expr"><span class="text-gray-500">Schedule:</span> {{ agent.cron_expr }}</div>
          <div v-if="agent.interval_secs"><span class="text-gray-500">Interval:</span> {{ agent.interval_secs }}s</div>
          <div v-if="agent.timeout_secs"><span class="text-gray-500">Timeout:</span> {{ agent.timeout_secs }}s</div>
          <div v-if="agent.max_retries"><span class="text-gray-500">Retries:</span> {{ agent.max_retries }} ({{ agent.retry_strategy ?? 'fixed' }}, delay {{ agent.retry_delay_secs ?? 30 }}s)</div>
        </div>
      </div>

      <!-- Runs -->
      <div class="card mb-6">
        <h3 class="text-sm font-semibold text-gray-500 mb-3">Recent Runs</h3>
        <div v-if="runs.length === 0" class="text-sm text-gray-400">No runs yet.</div>
        <table v-else class="w-full text-sm">
          <thead>
            <tr class="text-left text-gray-500 border-b">
              <th class="pb-2">ID</th>
              <th class="pb-2">Status</th>
              <th class="pb-2">Trigger</th>
              <th class="pb-2">Started</th>
              <th class="pb-2">Duration</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="run in runs" :key="run.id" class="border-b border-gray-100 hover:bg-gray-50">
              <td class="py-1.5">#{{ run.id }}</td>
              <td>
                <span :class="run.status === 'success' ? 'badge-success' : run.status === 'running' ? 'badge-running' : 'badge-error'">
                  {{ run.status }}
                </span>
              </td>
              <td class="text-gray-500">{{ run.trigger_type }}</td>
              <td class="text-gray-500">{{ formatTime(run.started_at) }}</td>
              <td class="text-gray-500">{{ duration(run) }}</td>
            </tr>
          </tbody>
        </table>
      </div>

      <!-- Logs -->
      <div class="card">
        <div class="flex items-center justify-between mb-3">
          <h3 class="text-sm font-semibold text-gray-500">Logs</h3>
          <div class="flex gap-2">
            <input v-model="logQuery" placeholder="Search logs..." class="text-sm border rounded px-2 py-1 w-48" @input="searchLogs" />
            <select v-model="logLevel" class="text-sm border rounded px-2 py-1" @change="searchLogs">
              <option value="">All levels</option>
              <option value="stdout">stdout</option>
              <option value="stderr">stderr</option>
            </select>
          </div>
        </div>
        <div class="max-h-96 overflow-y-auto font-mono text-xs bg-gray-900 text-gray-100 rounded p-3">
          <div v-if="logs.length === 0" class="text-gray-500">No logs.</div>
          <div v-for="log in logs" :key="log.id" class="leading-5">
            <span class="text-gray-500">{{ log.created_at?.split('T')[1]?.slice(0,8) }}</span>
            <span :class="log.level === 'stderr' ? 'text-red-400' : 'text-green-400'" class="mx-1">[{{ log.level }}]</span>
            <span>{{ log.message }}</span>
          </div>
        </div>
      </div>
    </template>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { api } from '../api.js'

const route = useRoute()
const id = route.params.id

const agent = ref(null)
const runs = ref([])
const logs = ref([])
const logQuery = ref('')
const logLevel = ref('')

async function load() {
  const agents = await api.agents()
  agent.value = agents.find(a => String(a.id) === String(id))
  runs.value = await api.runs(id)
  await searchLogs()
}

async function searchLogs() {
  const params = {}
  if (logQuery.value) params.q = logQuery.value
  if (logLevel.value) params.level = logLevel.value
  logs.value = await api.logs(id, params)
}

function formatTime(ts) {
  if (!ts) return '-'
  return new Date(ts).toLocaleString()
}

function duration(run) {
  if (!run.finished_at || !run.started_at) return '-'
  const ms = new Date(run.finished_at) - new Date(run.started_at)
  const secs = Math.floor(ms / 1000)
  if (secs < 60) return `${secs}s`
  return `${Math.floor(secs / 60)}m ${secs % 60}s`
}

onMounted(load)
</script>
