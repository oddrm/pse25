## Projekt starten
Diese Anleitung richtet sich an Benutzerinnen und Benutzer, die das Programm zum ersten Mal verwenden.

### Repository klonen oder aktualisieren
Beim ersten Start müssen Sie das Repository über den Link klonen:

`git clone https://github.com/oddrm/pse25.git`

Falls Sie das Repository bereits zuvor geklont haben und nur die neuesten Änderungen benötigen, führen Sie stattdessen folgenden Befehl aus:

`git pull origin main`

### Docker-Voraussetzung
Stellen Sie sicher, dass Ihr System Docker Compose mit x86-Architektur-Unterstützung verwendet.

### In das Projektverzeichnis wechseln
Wechseln Sie anschließend in das Projektverzeichnis:

`cd pse25`

### Logs Ordner
Erstellen Sie einen leeren Ordner im pse25-Verzeichnis (kann in Docker Compose konfiguriert werden).

### Plugins konfigurieren
Das muss auch gemacht werden, selbst wenn Sie keine Plugins verwenden. Zuerst muss ein Ordner plugins_dir in pse25 erstellt werden (kann ebenfalls in Docker Compose konfiguriert werden). In diesen Ordner muss die Datei plugin_base.py aus dem Ordner `pse25/backend/src/plugin_manager/plugins/plugin_base.py` kopiert werden. Außerdem werden in denselben Ordner (nicht in einen Unterordner) alle Plugins eingefügt. Beispiele finden Sie im Ordner `pse25/backend/src/plugin_manager/plugins/plugin_examples`. Dann muss im plugins_dir-Ordner ein Unterordner config mit einer Datei plugins.yaml erstellt werden. Die Konfiguration dafür kann aus folgendem Beispiel entnommen werden: `pse25/backend/src/plugin_manager/plugins/config/plugins.yaml`

### Anwendung starten (Development-Modus)
Starten Sie die Anwendung mit folgendem Befehl:

`DATA_PATH=/path/to/folder docker compose -f compose.dev.yaml up`

Wichtig: Ersetzen Sie /path/to/folder durch den Pfad zu Ihren Testdaten.

#### Alternative für Linux oder macOS
Unter Linux oder macOS können Sie alternativ folgendes Skript verwenden:

`DATA_PATH=/path/to/folder ./run.sh dev`

#### Production-Modus
Wenn Sie das Programm im Production-Modus starten möchten, verwenden Sie:

`./run.sh prod`

bzw.

`DATA_PATH=/path/to/folder docker compose -f compose.prod.yaml up`

## SAMBA/CIFS

Dafür Docker Volume konfigurieren. cifs-tools muss auf dem host installiert sein. Beispielkonfiguration:

```data:
    driver: local
    driver_opts:
      type: cifs
      device: "//127.0.0.1/Data"
      o: "username=samba,password=secret,vers=3.0"`
```