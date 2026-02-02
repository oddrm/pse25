// frontend/stores/plugins.ts
import { defineStore } from 'pinia'

export interface PluginItem {
  id: number
  name: string
  description: string
  isRunning?: boolean // optional für Fortschritt
  progress?: number   // optional für Fortschritt
}

export const usePluginsStore = defineStore('plugins', {
  state: () => ({
    plugins: [] as PluginItem[],
  }),
  actions: {
    loadTestPlugins() {
      this.plugins = [
        {
          id: 1,
          name: 'Dateien komprimieren',
          description: 'Komprimiert Rosbag-Dateien, um Speicherplatz zu sparen.',
        },
        {
          id: 2,
          name: 'Text generieren',
          description: 'Extrahiert Metadaten aus Rosbag-Dateien und erzeugt zusammenfassenden Text.',
        },
        {
          id: 3,
          name: 'Datenbank indexieren',
          description: 'Erstellt einen Index der Inhalte für schnelles Suchen und Filtern.',
        },
      ]
    },
  },
})

export default usePluginsStore
