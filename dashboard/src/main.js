import 'virtual:uno.css'
import { createApp } from 'vue'
import { createRouter, createWebHistory } from 'vue-router'
import App from './App.vue'

const routes = [
  { path: '/', name: 'dashboard', component: () => import('./views/Dashboard.vue') },
  { path: '/agents', name: 'agents', component: () => import('./views/AgentList.vue') },
  { path: '/agents/:id', name: 'agent-detail', component: () => import('./views/AgentDetail.vue') },
  { path: '/alerts', name: 'alerts', component: () => import('./views/Alerts.vue') },
]

const router = createRouter({
  history: createWebHistory(),
  routes,
})

createApp(App).use(router).mount('#app')
