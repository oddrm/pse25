import { defineConfig, devices } from '@playwright/test'

/**
 * See https://playwright.dev/docs/test-configuration.
 */
export default defineConfig({
    testDir: './tests/e2e',

    /* Run tests in files in parallel */
    fullyParallel: true,

    /* Fail the build on CI if you accidentally left test.only in the source code. */
    forbidOnly: !!process.env.CI,

    /* Retry on CI only */
    // retries: process.env.CI ? 2 : 0,
    retries: 0,
    /* Opt out of parallel tests on CI. */
    // workers: process.env.CI ? 1 : undefined,
    workers: 3,
    /* Reporter to use. See https://playwright.dev/docs/test-reporters */
    reporter: 'html',

    /* Shared settings for all the projects below. See https://playwright.dev/docs/api/class-testoptions. */
    use: {
        /* Base URL to use in actions like `await page.goto('/')`. */
        // Allow overriding in Docker/CI via BASE_URL env var (e.g. http://frontend:3000)
        baseURL: process.env.BASE_URL || 'http://127.0.0.1:3000',

        /* Collect trace when retrying the failed test. See https://playwright.dev/docs/trace-viewer */
        trace: 'on-first-retry',

        /* Screenshot on failure */
        screenshot: 'only-on-failure',
    },

    /* Configure projects for major browsers */
    projects: [
        {
            name: 'chromium',
            use: { ...devices['Desktop Chrome'] },
        },

        {
            name: 'firefox',
            use: { ...devices['Desktop Firefox'] },
        },

        {
            name: 'webkit',
            use: { ...devices['Desktop Safari'] },
        },

        /* Test against mobile viewports. */
        // {
        //   name: 'Mobile Chrome',
        //   use: { ...devices['Pixel 5'] },
        // },
        // {
        //   name: 'Mobile Safari',
        //   use: { ...devices['iPhone 12'] },
        // },
    ],

    /* Run your local dev server before starting the tests */
    webServer: process.env.CI ? undefined : {
        // Start Nuxt directly. Playwright will wait for the client build
        // to be available at /_nuxt/ before proceeding.
        command: 'npx nuxi dev --port 3000 --hostname 127.0.0.1',
        url: 'http://127.0.0.1:3000/_nuxt/',
        reuseExistingServer: !process.env.CI,
        // Allow ample time for dev server + client build
        timeout: 240 * 1000,
    },
})
