// frontend/stores/logs.ts
import { defineStore } from 'pinia'

export interface LogItem {
  id: number
  type: 'info' | 'warn' | 'error'
  message: string
  time: string
}

export const useLogsStore = defineStore('logs', {
  state: () => ({
    logs: [] as LogItem[],
  }),

  actions: {
    addLog(type: 'info' | 'warn' | 'error', message: string) {
      this.logs.unshift({
        id: this.logs.length + 1,
        type,
        message,
        time: new Date().toLocaleTimeString(),
      })
    },
  },
})
