// frontend/stores/plugins.ts
import { defineStore } from 'pinia'
import { useLogsStore } from './logsStore'


export interface PluginItem {
  id: number
  name: string
  description: string
  isRunning?: boolean // optional für Fortschritt
  progress?: number   // optional für Fortschritt
}

export interface RunningPlugin {
  pluginName: string
  entryName: string
  progress: number
}

export const usePluginsStore = defineStore('plugins', {
  state: () => ({
    plugins: [] as PluginItem[],
    runningPlugin: null as RunningPlugin | null, // für das aktuelle Plugin-Task
  }),
  actions: {
    loadTestPlugins() {
      this.plugins = [
        { id: 1, name: 'Dateien komprimieren', description: 'Komprimiert Rosbag-Dateien.' },
        { id: 2, name: 'Text generieren', description: 'Extrahiert Metadaten.' },
        { id: 3, name: 'Datenbank indexieren', description: 'Indexiert Inhalte.' },
      ]
    },

    // NEU: Ein Plugin starten (von Entry aus)
    startPlugin(plugin: PluginItem, entryName: string) {
    // LogsStore erst hier holen, wenn Pinia aktiv ist
    const logsStore = useLogsStore()

    this.runningPlugin = {
    pluginName: plugin.name,
    entryName,
    progress: 0,
   }

   // Fortschritt simulieren
   const interval = setInterval(() => {
    if (!this.runningPlugin) {
      clearInterval(interval)
      return
    }

    if (this.runningPlugin.progress < 100) {
      this.runningPlugin.progress += 2
    } else {
      clearInterval(interval)

      // Log jetzt nach Abschluss
      logsStore.addLog('info', `Plugin "${plugin.name}" auf Datei "${entryName}" abgeschlossen.`)

      this.runningPlugin = null
    }
  }, 100)
  }

  },
})

export default usePluginsStore
