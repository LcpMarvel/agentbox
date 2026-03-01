import {
  defineConfig,
  presetUno,
  presetAttributify,
} from 'unocss'

export default defineConfig({
  presets: [presetUno(), presetAttributify()],
  shortcuts: {
    'card': 'bg-white rounded-lg shadow p-4',
    'badge-success': 'bg-green-100 text-green-800 px-2 py-0.5 rounded text-sm',
    'badge-error': 'bg-red-100 text-red-800 px-2 py-0.5 rounded text-sm',
    'badge-running': 'bg-blue-100 text-blue-800 px-2 py-0.5 rounded text-sm',
    'badge-idle': 'bg-gray-100 text-gray-600 px-2 py-0.5 rounded text-sm',
    'badge-paused': 'bg-yellow-100 text-yellow-800 px-2 py-0.5 rounded text-sm',
    'btn': 'px-3 py-1.5 rounded text-sm font-medium cursor-pointer transition-colors',
    'btn-primary': 'btn bg-blue-600 text-white hover:bg-blue-700',
    'btn-danger': 'btn bg-red-600 text-white hover:bg-red-700',
    'btn-secondary': 'btn bg-gray-200 text-gray-700 hover:bg-gray-300',
  },
})
