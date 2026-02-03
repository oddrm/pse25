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
  }),
  actions: {
    loadTestPlugins() {
      if (this.plugins.length > 0) return
      this.plugins = [
        { id: 1, name: 'Dateien komprimieren', description: 'Reduziert die Größe von Rosbag-Dateien.' },
        { id: 2, name: 'Text generieren', description: 'Extrahiert Metadaten aus Inhalten.' },
        { id: 3, name: 'Datenbank indexieren', description: 'Optimiert die Suche in der Datenbank.' },
      ]
    },

    async startPlugin(pluginId: number, entryName?: string) {
  const logsStore = useLogsStore()
  const plugin = this.plugins.find(p => p.id === pluginId)
  if (!plugin) return

  if (entryName) {
    // Prüfen, ob exakt dieser Lauf schon existiert (Doppelklick-Schutz)
    if (this.runningPlugins.some(r => r.pluginName === plugin.name && r.entryName === entryName)) return
    
    const currentRunId = `${pluginId}-${entryName}-${Date.now()}`
    const running: RunningPlugin = { 
      runId: currentRunId, 
      pluginName: plugin.name, 
      entryName, 
      progress: 0 
    }
    
    this.runningPlugins.push(running)

    const interval = setInterval(() => {
      const item = this.runningPlugins.find(r => r.runId === currentRunId)
      if (item) {
        if (item.progress < 100) {
          item.progress += 5
        } else {
          clearInterval(interval)
          setTimeout(() => {
            this.runningPlugins = this.runningPlugins.filter(r => r.runId !== currentRunId)
            logsStore.addLog('info', `Plugin "${plugin.name}" auf "${entryName}" beendet.`)
          }, 500)
        }
      } else {
        clearInterval(interval)
      }
    }, 150)

      } else {
        // --- GLOBALER MODUS (Tabelle) ---
        if (plugin.isGlobalRunning) return
        plugin.isGlobalRunning = true
        plugin.globalProgress = 0

        const interval = setInterval(() => {
          if (plugin.globalProgress! < 100) {
            plugin.globalProgress! += 5
          } else {
            clearInterval(interval)
            setTimeout(() => {
              plugin.isGlobalRunning = false
              plugin.globalProgress = 0
              logsStore.addLog('info', `Plugin "${plugin.name}" global abgeschlossen.`)
            }, 500)
          }
        }, 150)
      }
    }
  }
})