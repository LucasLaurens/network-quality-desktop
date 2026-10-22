// Measures the windows as they render, in both themes (one Playwright project per theme): nothing
// overflows the window or a clipped box, and what should line up does.
import { expect, test, type Page } from '@playwright/test'
import tauriConfig from '../../src-tauri/tauri.conf.json' with { type: 'json' }

/** A window of `tauri.conf.json`, by its label; a missing one stops the run rather than measure 0 × 0. */
function findWindow(label: string): (typeof tauriConfig.app.windows)[number] {
  const window = tauriConfig.app.windows.find((entry) => entry.label === label)
  if (!window) throw new Error(`no ${label} window in tauri.conf.json`)
  return window
}

const MAIN = findWindow('main')
const POPOVER = findWindow('popover')
/** Half a pixel either way: sub-pixel layout, not a misalignment. */
const TOLERANCE = 0.5
const POPOVER_SCENARIOS = ['degraded', 'offline', 'undetermined']
/** A maximised window on a common desktop screen. */
const FULL_SCREEN = { width: 1920, height: 1080 }

/**
 * Every element that runs past the window's right edge unless a box within the window clips it,
 * and every text a clipping box cuts. O(e·d) for e elements d deep.
 */
function findOverflows(page: Page): Promise<string[]> {
  return page.evaluate((tolerance) => {
    const isClipping = (element: Element): boolean => getComputedStyle(element).overflowX !== 'visible'
    const isClippedWithinTheWindow = (element: Element): boolean => {
      for (let parent = element.parentElement; parent; parent = parent.parentElement) {
        if (isClipping(parent) && parent.getBoundingClientRect().right <= window.innerWidth + tolerance) return true
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
      if (box.right > window.innerWidth + tolerance && !isClippedWithinTheWindow(element)) {
        found.push(`past the edge: ${describe(element)}`)
      }
      const style = getComputedStyle(element)
      const cutsSideways = ['hidden', 'clip'].includes(style.overflowX) && element.scrollWidth > element.clientWidth + 1
      const cutsDownwards =
        ['hidden', 'clip'].includes(style.overflowY) && element.scrollHeight > element.clientHeight + 1
      if (cutsSideways || cutsDownwards) found.push(`cut: ${describe(element)}`)
    }
    return found
  }, TOLERANCE)
}

/** Whether the page needs scrolling down to show everything. */
function isTallerThanTheWindow(page: Page): Promise<boolean> {
  return page.evaluate(() => document.documentElement.scrollHeight > window.innerHeight)
}

/** The right edges of the elements `selector` finds. O(e). */
function readRightEdges(page: Page, selector: string): Promise<number[]> {
  return page
    .locator(selector)
    .evaluateAll((elements) => elements.map((element) => element.getBoundingClientRect().right))
}

/** The empty space left and right of the window's column. */
function readSideMargins(page: Page): Promise<[number, number]> {
  return page.locator('.window').evaluate((element) => {
    const box = element.getBoundingClientRect()
    return [box.left, window.innerWidth - box.right]
  })
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
  const [first, ...others] = edges
  expect(others.length).toBeGreaterThan(0)
  for (const edge of others) expect(Math.abs(edge - (first ?? edge))).toBeLessThanOrEqual(TOLERANCE)
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
    await page.setViewportSize({ width: POPOVER.width, height: POPOVER.height })

    // Act
    await page.goto(`/tests/preview/popover.html?scenario=${scenario}&tests=on`)
    await page.locator('[data-primary]').waitFor()
    const isTallerFolded = await isTallerThanTheWindow(page)
    await unfoldEverything(page)

    // Assert
    // Folded, it fits its window; a help the user unfolds may scroll.
    expect(isTallerFolded).toBe(false)
    expect(await findOverflows(page)).toEqual([])
    await expectTheme(page, String(testInfo.project.use.colorScheme))
  })
}

for (const size of [
  FULL_SCREEN,
  { width: MAIN.width, height: MAIN.height },
  { width: MAIN.minWidth ?? MAIN.width, height: MAIN.minHeight ?? MAIN.height },
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
    expectAligned(await readRightEdges(page, '.pages, .dashboard > section'))
    expectAligned(await readSideMargins(page))
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
    expectAligned(await readRightEdges(page, '.pages, .settings .rows'))
    expectAligned(await readSideMargins(page))
    await expectTheme(page, String(testInfo.project.use.colorScheme))
  })
}
