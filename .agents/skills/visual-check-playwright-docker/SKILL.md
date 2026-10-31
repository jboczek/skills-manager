---
name: visual-check-playwright-docker
description: Run visual regression checks for local HTML or TUI screenshot fixtures through Playwright Chromium in Docker. Use when validating terminal row height, clipped glyphs, selection artifacts, or README screenshot changes; requires a running Docker daemon and a healthy symphony-playwright container.
---

# Visual checks in Docker

Use this skill for screenshot validation. The browser and its Playwright client must both come from the Docker container; do not mix the host Playwright version with the container's browser version.

## Prerequisites

Run this from the repository root before creating or accepting a screenshot:

```sh
bash .agents/skills/visual-check-playwright-docker/scripts/require-docker-playwright.sh
```

The check must pass all of these conditions:

- Docker Desktop/daemon is running (`docker info` succeeds).
- `symphony-playwright` is running and healthy.
- The container has Playwright installed at `/usr/local/lib/node_modules/playwright`.
- The container's Playwright server accepts a connection on port `3000`.

If the existing container is stopped, start it and check again:

```sh
docker start symphony-playwright
bash .agents/skills/visual-check-playwright-docker/scripts/require-docker-playwright.sh
```

If the container does not exist, provision the repository's current image and server with Docker running:

```sh
docker run -d \
  --name symphony-playwright \
  --shm-size=2gb \
  -p 3000:3000 \
  symphony-playwright:1.63.0-alpha-2026-08-05 \
  playwright run-server --host 0.0.0.0 --port 3000 --max-clients 20
```

Do not proceed when Docker or the browser server is unavailable. Do not silently fall back to a host browser.

## Capture workflow

1. Render the fixture or app at the exact target viewport. For the README inventory image, use `1600x780` with `deviceScaleFactor: 1`.
2. Serve the fixture from the host, for example:

   ```sh
   python3 -m http.server 8765 --directory /absolute/path/to/visual-fixture
   ```

3. Connect from Node inside the running container. Use `host.docker.internal` for a host-served fixture on Docker Desktop:

   ```sh
   docker exec symphony-playwright node -e '
   const { chromium } = require("/usr/local/lib/node_modules/playwright");
   (async () => {
     const browser = await chromium.connect("ws://127.0.0.1:3000/");
     const page = await browser.newPage({
       viewport: { width: 1600, height: 780 },
       deviceScaleFactor: 1,
     });
     await page.goto("http://host.docker.internal:8765", { waitUntil: "networkidle" });
     await page.screenshot({ path: "/tmp/skills-manager-inventory.png" });
     await browser.close();
   })().catch((error) => {
     console.error(error);
     process.exit(1);
   });
   '
   docker cp symphony-playwright:/tmp/skills-manager-inventory.png docs/assets/skills-manager-inventory.png
   ```

4. Inspect the PNG at native size and in a zoomed crop of every expanded group. Keep the source fixture and screenshot in the same coordinate system; do not repair the PNG with a canvas mask or by painting over a selection band.

## README inventory fixture

The expanded list must keep this exact data:

```text
prompt-lab · skills/prompt-lab       3 skills
  [x] code-review                    ✓  ✓  –  global  symlink,symlink
  [ ] release-notes                  –  ✓  ✓  global  symlink,symlink
  [ ] security-audit                 –  –  –  –       not exposed
team-toolkit · skills/team-toolkit   2 skills
```

The rows in the 1600x780 capture use a 20px step. The expanded `code-review`, `release-notes`, and `security-audit` rows must each contain their complete glyph height; `code-review` and `release-notes` are the regression cases. Use `line-height: 20px` and never clip a row with a smaller fixed height or `overflow: hidden`. The group headers remain visible, and the screenshot must not contain the purple selected-row background.

When a row-height check fails, fix the fixture/layout and re-render the complete screenshot. Do not copy pixels from the old PNG: the previous selection cleanup removed the lower glyph pixels of adjacent rows because it overwrote a rectangular band without reconstructing the row baselines.

## Acceptance checklist

- The prerequisite script passes.
- The screenshot was made by the container Playwright client and Chromium.
- The viewport is `1600x780`, with no unexpected device scale factor.
- The three skill labels, two group labels, and two group counts match the fixture above.
- Every expanded row has a complete top and bottom glyph edge at native and zoomed resolution.
- No purple selection band or clipped text remains.
