import { defineVitestConfig } from '@nuxt/test-utils/config'

export default defineVitestConfig({
    test: {
        environment: 'happy-dom',
        globals: true,
        // Don't pick up Playwright E2E tests
        exclude: ['tests/e2e/**'],
        coverage: {
            provider: 'v8',
            reporter: ['text', 'json'],
            include: [
                'app/components/Entry.vue',
                'app/components/TagEditor.vue',
                'app/utils/dbQueries.ts',
                'app/utils/entryColumns.ts',
                'stores/**/*.ts',
            ],
            thresholds: {
                lines: 90,
                functions: 90,
                statements: 90,
            },
            exclude: [
                'node_modules/',
                '.nuxt/',
                // keep generic tests/ excluded from coverage but allow unit tests under app/
                'tests/',
                '*.config.ts',
                'app/app.vue',
                'app/app.config.ts',
                'app/assets/**',
                'app/layouts/**',
                'app/pages/**',
                'app/utils/entry.ts',
                'app/utils/metadata.ts',
                'app/utils/sensor.ts',
                'app/utils/sequence.ts',
                'app/utils/topic.ts',
                'app/components/entryInfo.vue',
                'app/components/EntryPlugins.vue',
                'app/components/PluginStatusHeader.vue',
                'app/components/sequence.vue',
                'app/components/table.vue',
                'plugins/**',
            ],
        },
    },
})
