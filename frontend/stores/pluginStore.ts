import { defineStore } from 'pinia'
import { useLogsStore } from './logsStore'

export interface PluginItem {
  id: number
  name: string
  description: string
  isGlobalRunning?: boolean
  globalProgress?: number
  isRunning?: boolean // Status für das Dropdown-Feedback
}

export interface RunningPlugin {
  runId: string // Eindeutige ID (z.B. "1-datei_x-171234567")
  pluginName: string
  entryName: string
  progress: number
}

export const usePluginsStore = defineStore('plugins', {
  state: () => ({
    plugins: [] as PluginItem[],
    runningPlugins: [] as RunningPlugin[],
    _pollInterval: null as any,
  }),
  actions: {
    async loadPlugins() {
      if (this.plugins.length > 0) return
      try {
        const res = await fetch('/backend/plugins/registered')
        if (!res.ok) throw new Error('Failed to load plugins')
        const data = await res.json()
        this.plugins = data.map((p: any, idx: number) => ({
          id: idx + 1,
          name: p.name,
          description: p.description,
          isGlobalRunning: false,
          globalProgress: 0,
        }))

        // start polling running instances
        this.startPollingRunning()
      } catch (err) {
        // fallback: keep empty list
        console.error('Error loading plugins from backend:', err)
      }
    },

    startPollingRunning() {
      if (this._pollInterval) return
      this._pollInterval = setInterval(async () => {
        try {
          const res = await fetch('/backend/plugin/instances')
          if (!res.ok) return
          const data = await res.json()

          // map to runningPlugins (preserve any entryName we seeded locally)
          const existingById: Record<string, RunningPlugin> = {}
          for (const r of this.runningPlugins) existingById[r.runId] = r

          const newRunning: RunningPlugin[] = data.map((p: any) => {
            const id = String(p.instance_id ?? '')
            const existing = existingById[id]
            return {
              runId: id,
              pluginName: p.name,
              entryName: existing ? existing.entryName : '',
              progress: p.state === 'Completed' ? 100 : (p.state === 'Failed' ? 100 : 0),
            }
          })

          this.runningPlugins = newRunning
        } catch (e) {
          // ignore polling errors
        }
      }, 300)
    },

    async startPlugin(pluginId: number, entryName?: string) {
      const logsStore = useLogsStore()
      const plugin = this.plugins.find(p => p.id === pluginId)
      if (!plugin) return

      // Prevent duplicate start for same entry
      if (entryName && this.runningPlugins.some(r => r.pluginName === plugin.name && r.entryName === entryName)) return

      try {
        const res = await fetch(`/backend/plugins/${encodeURIComponent(plugin.name)}/start`, {
          method: 'POST',
        })
        if (!res.ok) throw new Error('Failed to start plugin')
        const instId = await res.json()

        const runId = String(instId)
        this.runningPlugins.push({ runId, pluginName: plugin.name, entryName: entryName ?? '', progress: 0 })

        // mark global flag if started without entry
        if (!entryName) {
          plugin.isGlobalRunning = true
        }
      } catch (err: any) {
        console.error('Error starting plugin:', err)
      }
    },

    stopPolling() {
      if (this._pollInterval) {
        clearInterval(this._pollInterval)
        this._pollInterval = null
      }
    }
  }
})