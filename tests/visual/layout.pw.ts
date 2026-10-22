// Measures the windows as they render, in both themes (one Playwright project per theme): nothing
// overflows the window or a clipped box, and what should line up does.
import { expect, test, type Page } from '@playwright/test'
import tauriConfig from '../../src-tauri/tauri.conf.json' with { type: 'json' }

const [MAIN, POPOVER] = tauriConfig.app.windows
/** Half a pixel either way: sub-pixel layout, not a misalignment. */
const TOLERANCE = 0.5
const POPOVER_SCENARIOS = ['degraded', 'offline', 'undetermined']

/**
 * Every element that runs past the window's right edge outside a scrolling box, or whose text a
 * clipping box cuts. O(e) for e elements.
 */
function findOverflows(page: Page): Promise<string[]> {
  return page.evaluate((tolerance) => {
    const isScrolling = (element: Element): boolean => ['auto', 'scroll'].includes(getComputedStyle(element).overflowX)
    const hasScrollingAncestor = (element: Element): boolean => {
      for (let parent = element.parentElement; parent; parent = parent.parentElement) {
        if (isScrolling(parent)) return true
      }
      return false
    }
    const describe = (element: Element): string =>
      `${element.tagName.toLowerCase()} « ${element.textContent.trim().slice(0, 40)} »`
    const found: string[] = []
    if (document.documentElement.scrollWidth > window.innerWidth) found.push('the page scrolls sideways')
    for (const element of document.querySelectorAll('body *')) {
      const box = element.getBoundingClientRect()
      if (box.width === 0) continue
      if (box.right > window.innerWidth + tolerance && !hasScrollingAncestor(element)) {
        found.push(`past the edge: ${describe(element)}`)
      }
      if (getComputedStyle(element).overflowX === 'hidden' && element.scrollWidth > element.clientWidth + 1) {
        found.push(`cut: ${describe(element)}`)
      }
    }
    return found
  }, TOLERANCE)
}

/** Opens every folded part (help, technical details, chart values): folded, it measures nothing. */
async function unfoldEverything(page: Page): Promise<void> {
  await page.locator('details').evaluateAll((elements) => {
    for (const element of elements) (element as HTMLDetailsElement).open = true
  })
}

/** The left edges of the elements `selector` finds. O(e). */
function readLeftEdges(page: Page, selector: string): Promise<number[]> {
  return page
    .locator(selector)
    .evaluateAll((elements) => elements.map((element) => element.getBoundingClientRect().left))
}

function expectAligned(edges: number[]): void {
  expect(edges.length).toBeGreaterThan(1)
  const first = edges[0] ?? 0
  for (const edge of edges) expect(Math.abs(edge - first)).toBeLessThanOrEqual(TOLERANCE)
}

/** Whether the page paints the theme the project asks for: a dark or a light background. */
async function readBackgroundLuminance(page: Page): Promise<number> {
  return page.evaluate(() => {
    const canvas = document.createElement('canvas').getContext('2d')
    if (!canvas) return Number.NaN
    canvas.fillStyle = getComputedStyle(document.body).backgroundColor
    canvas.fillRect(0, 0, 1, 1)
    const [red = 0, green = 0, blue = 0] = canvas.getImageData(0, 0, 1, 1).data
    return (0.2126 * red + 0.7152 * green + 0.0722 * blue) / 255
  })
}

async function expectTheme(page: Page, colorScheme: string): Promise<void> {
  const luminance = await readBackgroundLuminance(page)
  if (colorScheme === 'dark') expect(luminance).toBeLessThan(0.5)
  else expect(luminance).toBeGreaterThan(0.5)
}

for (const scenario of POPOVER_SCENARIOS) {
  test(`the popover holds in its window (${scenario})`, async ({ page }, testInfo) => {
    // Arrange
    await page.setViewportSize({ width: POPOVER?.width ?? 0, height: POPOVER?.height ?? 0 })

    // Act
    await page.goto(`/tests/preview/popover.html?scenario=${scenario}&tests=on`)
    await page.locator('[data-primary]').waitFor()
    await unfoldEverything(page)

    // Assert
    expect(await findOverflows(page)).toEqual([])
    await expectTheme(page, String(testInfo.project.use.colorScheme))
  })
}

for (const size of [
  { width: MAIN?.width ?? 0, height: MAIN?.height ?? 0 },
  { width: MAIN?.minWidth ?? 0, height: MAIN?.minHeight ?? 0 },
]) {
  test(`the dashboard lines up its sections at ${String(size.width)} px`, async ({ page }, testInfo) => {
    // Arrange
    await page.setViewportSize(size)

    // Act
    await page.goto('/tests/preview/dashboard.html')
    await page.locator('#dashboard-state').waitFor()
    await unfoldEverything(page)

    // Assert
    expect(await findOverflows(page)).toEqual([])
    expectAligned(await readLeftEdges(page, '.dashboard > section, .section-head h2'))
    await expectTheme(page, String(testInfo.project.use.colorScheme))
  })

  test(`the settings line up their rows at ${String(size.width)} px`, async ({ page }, testInfo) => {
    // Arrange
    await page.setViewportSize(size)
    await page.goto('/tests/preview/dashboard.html')

    // Act
    await page.locator('[data-page="settings"]').click()
    await page.locator('#settings-title').waitFor()

    // Assert
    expect(await findOverflows(page)).toEqual([])
    expectAligned(await readLeftEdges(page, '.settings h3'))
    expectAligned(await readLeftEdges(page, '.setting > :first-child'))
    await expectTheme(page, String(testInfo.project.use.colorScheme))
  })
}
