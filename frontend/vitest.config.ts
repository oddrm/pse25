import { defineVitestConfig } from '@nuxt/test-utils/config'

export default defineVitestConfig({
    test: {
        environment: 'happy-dom',
        globals: true,
        // Don't pick up Playwright E2E tests
        exclude: ['tests/e2e/**'],
        coverage: {
            provider: 'v8',
            reporter: ['text', 'json', 'html'],
            exclude: [
                'node_modules/',
                '.nuxt/',
                // keep generic tests/ excluded from coverage but allow unit tests under app/
                'tests/',
                '*.config.ts',
            ],
        },
    },
})
