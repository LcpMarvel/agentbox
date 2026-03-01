const BASE = '/api'

async function fetchJSON(url) {
  const res = await fetch(BASE + url)
  return res.json()
}

async function postJSON(url) {
  const res = await fetch(BASE + url, { method: 'POST' })
  return res.json()
}

export const api = {
  agents: () => fetchJSON('/agents'),
  runs: (id) => fetchJSON(`/agents/${id}/runs`),
  logs: (id, params = {}) => {
    const qs = new URLSearchParams(params).toString()
    return fetchJSON(`/agents/${id}/logs${qs ? '?' + qs : ''}`)
  },
  stats: () => fetchJSON('/dashboard/stats'),
  alerts: () => fetchJSON('/alerts'),
  run: (id) => postJSON(`/agents/${id}/run`),
  pause: (id) => postJSON(`/agents/${id}/pause`),
  resume: (id) => postJSON(`/agents/${id}/resume`),
  logStream: (runId) => new EventSource(`${BASE}/runs/${runId}/logs/stream`),
}
