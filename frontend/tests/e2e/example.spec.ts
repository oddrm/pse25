import { test, expect } from '@playwright/test'

// Minimal, reliable smoke test that uses the configured `baseURL`.
test('homepage responds with a successful status', async ({ page }) => {
    const response = await page.goto('/')
    // Ensure we got a response and it's not an error status
    expect(response).not.toBeNull()
    // Response.status() exists when response is not null
    // assert it's a 2xx or 3xx
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    expect(response!.status()).toBeLessThan(400)
})

