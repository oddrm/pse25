import { defineStore } from 'pinia'
import { useLogsStore } from './logsStore'

export interface PluginItem {
  id: number
  name: string
  description: string
  isGlobalRunning?: boolean
  globalProgress?: number
  isRunning?: boolean // Status für das Dropdown-Feedback
  recentInstances?: RunningPlugin[]
}

export interface RunningPlugin {
  runId: number // Eindeutige ID (z.B. "1-datei_x-171234567")
  pluginName: string
  entryName: string
  progress: number
  state?: string
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
            const state = p.state ?? ''
            const progress = Math.floor((p.progress ?? 0) * 100)
            return {
              runId: id,
              pluginName: p.name,
              entryName: existing ? existing.entryName : '',
              progress,
              state,
            }
          })

          this.runningPlugins = newRunning

          // attach recent instances to each known plugin and update global flags
          for (const plugin of this.plugins) {
            const instances = newRunning.filter(i => i.pluginName === plugin.name)
            plugin.recentInstances = instances.sort((a, b) => b.runId - a.runId).slice(0, 3)

            const globalInstances = instances.filter(i => i.entryName === '')
            if (globalInstances.length > 0) {
              plugin.isGlobalRunning = globalInstances.some(i => i.state === 'Running' || i.state === 'Paused')
              // const avg = Math.round(globalInstances.reduce((s, i) => s + i.progress, 0) / globalInstances.length)
              plugin.globalProgress = globalInstances[0]?.progress || 0
            } else {
              plugin.isGlobalRunning = false
              plugin.globalProgress = 0
            }
          }
        } catch (e) {
          // ignore polling errors
        }
      }, 300)
    },

    async startPlugin(pluginId: number, entryPath?: string, payload?: any) {
      const logsStore = useLogsStore()
      const plugin = this.plugins.find(p => p.id === pluginId)
      if (!plugin) return


      if (entryPath && this.runningPlugins.some(r => r.pluginName === plugin.name && r.entryName === entryPath)) return

      try {
        console.debug('[plugins] starting plugin', { plugin: plugin.name, entryName: entryPath, payload })

        const res = await fetch(`/backend/plugins/${encodeURIComponent(plugin.name)}/start`, {
          method: 'POST',
          body: JSON.stringify({ entry_path: entryPath, payload }),
        })

        const text = await res.text()
        console.debug('[plugins] start response', { ok: res.ok, status: res.status, text })

        if (!res.ok) throw new Error(`Failed to start plugin: ${text}`)
        const instId = JSON.parse(text)

        this.runningPlugins.push({ runId: instId, pluginName: plugin.name, entryName: entryPath ?? '', progress: 0 })

        if (!entryPath) {
          plugin.isGlobalRunning = true
        }
      } catch (err: any) {
        console.error('Error starting plugin:', err)
      }
    },

    async registerPlugins() {
      try {
        const res = await fetch('/backend/plugins/register', { method: 'PUT' })
        if (!res.ok) throw new Error('Failed to register plugins')

        // force reload of registered plugins
        this.plugins = []
        await this.loadPlugins()
      } catch (err) {
        console.error('Error registering plugins:', err)
      }
    },

    async stopInstance(runId: number) {
      try {
        const res = await fetch(`/backend/plugins/${encodeURIComponent(runId)}/stop`, {
          method: 'PUT',
        })
        if (!res.ok) throw new Error('Failed to stop instance')

        // optimistic update
        const idx = this.runningPlugins.findIndex(r => r.runId === runId)
        if (idx !== -1) {
          if (!this.runningPlugins[idx]) {
            console.warn('Instance not found in local state after stop:', runId)
            return;
          }
          this.runningPlugins[idx].state = 'Stopped'
          this.runningPlugins[idx].progress = 100
        }
      } catch (err) {
        console.error('Error stopping plugin instance:', err)
      }
    },

    async pauseInstance(runId: number) {
      try {
        const res = await fetch(`/backend/plugins/${encodeURIComponent(runId)}/pause`, {
          method: 'PUT',
        })
        if (!res.ok) throw new Error('Failed to pause instance')

        const idx = this.runningPlugins.findIndex(r => r.runId === runId)
        if (idx !== -1) {
          if (!this.runningPlugins[idx]) {
            console.warn('Instance not found in local state after pause:', runId)
            return;
          }
          this.runningPlugins[idx].state = 'Paused'
        }
      } catch (err) {
        console.error('Error pausing plugin instance:', err)
      }
    },

    async resumeInstance(runId: number) {
      try {
        const res = await fetch(`/backend/plugins/${encodeURIComponent(runId)}/resume`, {
          method: 'PUT',
        })
        if (!res.ok) throw new Error('Failed to resume instance')

        const idx = this.runningPlugins.findIndex(r => r.runId === runId)
        if (idx !== -1) {
          if (!this.runningPlugins[idx]) {
            console.warn('Instance not found in local state after resume:', runId)
            return;
          }
          this.runningPlugins[idx].state = 'Running'
        }
      } catch (err) {
        console.error('Error resuming plugin instance:', err)
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