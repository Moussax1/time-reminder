# Time Reminder — Brand Identity & Logo Guidelines

## 1. The Logo & Design Rationale
- **Core Idea**: Time Reminder is an ultra-lightweight, zero-bloat Windows HUD timer engineered in Rust for PC gamers and performance purists. The visual mark (**Concept T2: Curved Glass Bulb**) combines classic hourglass contours with razor-sharp arcade geometry.
- **Glass Contour**: Two symmetric circular-arc chambers taper naturally into a central aperture, avoiding the lateral squeezed distortion of early pod iterations.
- **The Spark**: A floating central grain represents the active passage of time, functioning as an optical anchor that remains crisp down to 16×16 px in the Windows taskbar tray.
- **Vector Integrity**: 100% pure vector paths using `fill-rule="evenodd"` without fake white knockouts, raster effects, or live font dependencies. All assets achieve a 100/100 production-readiness audit score.

---

## 2. Master Artwork & Delivery Hierarchy
All production assets are cataloged in `branding/dist/`:

| Directory | Format | Contents & Usage |
|---|---|---|
| `branding/` | SVG | Master sources: `time-reminder-symbol.svg`, `time-reminder-symbol-small.svg`, `time-reminder-horizontal.svg`, `time-reminder-stacked.svg` |
| `branding/dist/symbol/` | SVG & PNG | Standalone mark in Black, White, Cyber Mint (`#00FF9D`), Square Avatar, App Icon tile, and Favicon. PNG resolutions: 16, 32, 64, 128, 256, 512, 1024 px. |
| `branding/dist/symbol/` | ICO & PWA | Web and OS integration: `favicon.ico` (multi-res 16/32/48), `apple-touch-icon.png`, `icon-192.png`, `icon-512.png`, `maskable-512.png`, `site.webmanifest`, `head-snippet.html`. |
| `branding/dist/horizontal/` | SVG & PNG | Primary brand lockup (Symbol + Wordmark side-by-side) in Black, White, and Cyber Mint (`#00FF9D`). High-res PNGs: 600 px and 1200 px. |
| `branding/dist/stacked/` | SVG & PNG | Vertical lockup (Symbol over Wordmark) for square badges, store banners, and packaging. High-res PNGs: 600 px and 1200 px. |

---

## 3. Clear Space
To ensure legibility and impact, maintain a clear safety zone around the logo on all sides:
- **Safety Margin**: **$X$**, where $X = 25\%$ of the total symbol height (e.g. 64 px on a 256 px mark).
- No text, UI controls, HUD elements, or borders should intrude into this exclusion zone.

```
       +---------------------------------------+
       |                  [X]                  |
       |     +---------------------------+     |
   [X] | [X] |  TIME REMINDER LOCKUP     | [X] | [X]
       |     +---------------------------+     |
       |                  [X]                  |
       +---------------------------------------+
```

---

## 4. Minimum Sizes

| Mark Variant | Screen (Digital) | Print | Recommended File |
|---|---|---|---|
| **Horizontal Lockup** | 120 px wide | 32 mm wide | `time-reminder-horizontal.svg` / `-600.png` |
| **Stacked Lockup** | 80 px wide | 22 mm wide | `time-reminder-stacked.svg` / `-600.png` |
| **Standard Symbol** | 32 px height | 8 mm | `time-reminder-symbol.svg` / `-32.png` |
| **Micro / Tray Cut** | **16 px** height | 4 mm | `time-reminder-symbol-small.svg` / `favicon-16.png` |

> [!IMPORTANT]
> For any display context at or below 24×24 px (e.g., Windows taskbar tray, browser favicon), always use `time-reminder-symbol-small.svg` or `favicon-16.png`. This specialized cut features a +50% widened waist aperture and enlarged central grain to prevent fill-in blur.

---

## 5. Color Palette

The brand palette reflects stealth hardware aesthetics with high-visibility cyber accents:

| Role | Color Name | HEX | RGB | Use Case |
|---|---|---|---|---|
| **Primary Accent** | Cyber Mint | `#00FF9D` | `0, 255, 157` | Primary HUD highlights, active timers, neon icon variant |
| **Secondary Accent**| Electric Cyan | `#00E5FF` | `0, 229, 255` | Secondary telemetry, countdown warnings, gradient accents |
| **Dark Canvas** | Obsidian Black | `#0D0E12` | `13, 14, 18` | App background, tray menu, presentation tiles, dark UI |
| **Neutral White** | Crisp White | `#FFFFFF` | `255, 255, 255` | Reverse mark on dark backgrounds, primary body text |
| **Neutral Black** | Pitch Black | `#000000` | `0, 0, 0` | 1-color print, documentation on white paper |

### Approved Combinations
1. **Cyber Mint on Obsidian (`#00FF9D` on `#0D0E12`)**: Primary gaming HUD & dark-mode software interface.
2. **Crisp White on Obsidian (`#FFFFFF` on `#0D0E12`)**: High-contrast accessibility mode.
3. **Pitch Black on Pure White (`#000000` on `#FFFFFF`)**: Standard print and light GitHub documentation.
4. **Pure White on Cyber Mint (`#FFFFFF` on `#00FF9D`)**: High-energy promotion and stickers.

---

## 6. Typography
- **Wordmark Style**: Razor-sharp, all-caps geometric display letterforms with tight tracking and high stroke-to-height ratio.
- **Companion Monospace**: `Cascadia Code`, `JetBrains Mono`, `ui-monospace`, `Menlo`, or `Consolas`.
- **System UI Font**: `Segoe UI`, `-apple-system`, `system-ui`.

---

## 7. Logo Misuse & Don'ts
To maintain the professional integrity of the Time Reminder brand:
- **DO NOT** squeeze, stretch, or alter the aspect ratio of the hourglass chambers.
- **DO NOT** pinch or narrow the waist beyond the engineered vector curves.
- **DO NOT** add drop shadows, outer glows, bevels, or skeuomorphic glass reflections.
- **DO NOT** use rasterized low-resolution PNGs when SVGs can be rendered natively.
- **DO NOT** place the dark monochrome mark against low-contrast dark gray backgrounds.
- **DO NOT** re-type the wordmark using default fonts; always use the vector paths provided in the lockup files.

---

## 8. Presentation & Delivery Assets
- **Client Presentation Board**: `branding/presentation.html` (interactive HTML slide deck)
- **Slide Renders**: `branding/slides/slide-01.png` to `slide-05.png` (Title, Brief, T2 Spec, In-Use Mockups, Next Steps)
- **Windows Icon**: `branding/dist/symbol/favicon.ico`
