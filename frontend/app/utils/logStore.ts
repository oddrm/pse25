// Damit die Logs nicht verschwinden wenn man die Seite wechselt werden sie global gespeichert
import { reactive } from 'vue';

export const logsStore = reactive({
  logs: [] as { id: number; type: string | undefined; message: string; time: string; }[]
});
