<template>
  <div class="min-h-screen">
    <!-- left sidebar -->
    <aside class="fixed left-0 top-5 h-screen w-80 bg-base-200 p-4 pt-6 overflow-y-auto">
      <ul class="menu">
        <li class="menu-title"><span>Navigation</span></li>

        <li><a href="#allgemein">General</a></li>
        <li>
          <details open>
            <summary>
              <a href="#table">Table</a>
            </summary>
            <ul>
              <li><a href="#table-overview">Overview</a></li>
              <li><a href="#table-entry-info">Entry Info</a></li>
              <li><a href="#table-sequences">Sequences</a></li>
            </ul>
          </details>
        </li>

        <li>
          <details open>
            <summary>
              <a href="#plugins">Plugins</a>
            </summary>
            <ul>
              <li><a href="#plugins-how">How It Works &amp; Control</a></li>
              <li><a href="#plugins-triggers">Triggers</a></li>
              <li><a href="#plugins-monitoring">Status &amp; Monitoring</a></li>
              <li><a href="#plugins-admin">Administration</a></li>
            </ul>
          </details>
        </li>

        <li><a href="#logs">Logs</a></li>
      </ul>
    </aside>

    <!-- right content -->
    <main class="ml-80 pt-15 px-6 py-14">
      <div class="max-w-4xl mx-auto">
        <h1 class="text-3xl font-bold mb-6">
          ROSBag Manager — User Guide
        </h1>

        <!-- General -->
        <section id="allgemein" class="mb-10 scroll-mt-10">
          <h2 class="text-2xl font-semibold mb-4">General</h2>
          <p class="mb-4">1. Open the Table section to view entries.</p>
          <p class="mb-4">2. Use Plugins when additional processing is required.</p>
          <p class="mb-4">3. Check the Logs to ensure actions were executed without errors.</p>
        </section>

        <!-- Table -->
        <section id="table" class="mb-10 scroll-mt-10">
          <h2 class="text-2xl font-semibold mb-4">1. Table</h2>

          <h3 id="table-overview" class="scroll-mt-10 text-xl font-semibold mt-6 mb-2">
            Overview
          </h3>
          <p class="mb-4">
            The <b>Table</b> section displays all available MCAP files in the system. Each row represents one entry.
          </p>
          <p class="mb-4">
            Each entry includes:
            status, name, link, file size, platform, and tags (you can assign an unlimited number of tags).
          </p>
          <p class="mb-4">
            Status indicators are color-coded:
            <br />
            • Green – Complete<br />
            • Yellow – Incomplete<br />
            • Red – Error detected(MCAP not readable)<br />
            • Black – Not filled
          </p>
          <p class="mb-4">
            You can sort entries by status, name, link, or platform. To quickly find a specific entry, use the search
            bar.
          </p>
          <p class="mb-4">
            Plugins can be executed per entry. For more details, see the
            <a href="#plugins" class="text-blue-600 underline">Plugins</a> section.
          </p>

          <h3 id="table-entry-info" class="scroll-mt-10 text-xl font-semibold mt-6 mb-2">
            Entry Info
          </h3>
          <p class="mb-4">
            Click any row to open the <b>Entry Info</b> panel on the left side. It contains general metadata for the
            selected entry.
          </p>
          <p class="mb-4">
            In the Entry Info panel you can:
            <br />
            • View general information<br />
            • Add or edit the platform information<br />
            • Add weather information<br />
            • View topics<br />
            • Add or edit a description<br />
            • Add sensors (select from existing sensors or create new ones)
          </p>

          <h3 id="table-sequences" class="scroll-mt-10 text-xl font-semibold mt-6 mb-2">
            Sequences
          </h3>
          <p class="mb-4">
            On the right side of each entry row, there is a dropdown button. Clicking it shows all sequences linked to
            that entry.
          </p>
          <p class="mb-4">
            In the sequences view you can create new sequences and delete existing ones.
          </p>
        </section>

        <!-- Plugins -->
        <section id="plugins" class="mb-10 scroll-mt-10">
          <h2 class="text-2xl font-semibold mb-4">2. Plugins</h2>

          <p class="mb-4">
            Plugins extend the ROSBag Manager with additional functionality such as automated analyses,
            data exports (e.g., YAML export), or compression of MCAP files. They allow complex tasks to be
            executed directly from the web interface without requiring manual access to the file system.
          </p>

          <h3 id="plugins-how" class="text-xl font-semibold mt-6 mb-2">
            How It Works &amp; Control
          </h3>
          <p class="mb-4">
            Each plugin runs in an isolated environment to ensure the stability of the main system.
            You can manage the lifecycle of a plugin instance via the dashboard:
          </p>
          <ul class="list-disc pl-6 mb-4 space-y-2">
            <li><b>Start</b> — Manual plugins can be started directly from the detail view of an entry.</li>
            <li><b>Pause &amp; Resume</b> — If a plugin supports cooperative multitasking, you can pause and resume it
              later.</li>
            <li><b>Stop</b> — You can stop a running instance at any time; the system will attempt graceful termination.
            </li>
          </ul>

          <h3 id="plugins-triggers" class="text-xl font-semibold mt-6 mb-2">
            Triggers
          </h3>
          <p class="mb-4">
            Plugins are started based on specific events. The corresponding trigger is displayed in the plugin overview:
          </p>
          <ul class="list-disc pl-6 mb-4 space-y-1">
            <li><b>Manual</b> — Runs only when you explicitly click “Run” in the frontend.</li>
            <li><b>OnEntryCreate</b> — Triggered when a new MCAP file is registered in the system.</li>
            <li><b>OnEntryUpdate / Delete</b> — Triggered when a MCAP file is modified (this does not trigger on delete
              or create)</li>
            <li><b>OnSchedule (Scheduled)</b> — Runs on a fixed schedule (e.g., every night at 2:00 AM), using a cron
              expression.</li>
          </ul>

          <h3 id="plugins-monitoring" class="text-xl font-semibold mt-6 mb-2">
            Status &amp; Monitoring
          </h3>
          <p class="mb-4">
            You can monitor the real-time progress of your plugins in the instances list:
          </p>
          <ul class="list-disc pl-6 mb-4 space-y-2">
            <li><b>Status Indicator</b> — Shows whether a plugin is running, paused, completed, or failed.</li>
            <li><b>Progress Bar</b> — Many plugins report progress (0%–100%) directly to the UI.</li>
            <li><b>Logs</b> — Error messages and status reports are forwarded to the central log panel.</li>
          </ul>

          <h3 id="plugins-admin" class="text-xl font-semibold mt-6 mb-2">
            Administration
          </h3>
          <p>
            Before a plugin can be used, it must be registered and enabled in the system.
            Administrators can enable or disable plugins through a central configuration file.
            Invalid plugins — for example, those missing required components — are automatically marked and cannot be
            started.
          </p>
        </section>

        <!-- Logs -->
        <section id="logs" class="mb-10 scroll-mt-10">
          <h2 class="text-2xl font-semibold mb-4">3. Logs</h2>
          <p class="mb-3">
            The <b>Logs</b> section displays possible errors or warnings.
          </p>
          <p class="mb-3">
            It is recommended to regularly check the logs to ensure that all actions were executed successfully.
            All logs can be filtered in the top-right corner by category:
            "Debug & Above", "Info & Above", "Warn & Above" and "Error".
          </p>
          <p class="mb-3">
            Additionally, the log view does not display all at once.
            You can choose to display only the most recent 50, 100, 500, or 1000 log entries.
          </p>
        </section>
      </div>
    </main>
  </div>
</template>

<script setup lang="ts">
</script>