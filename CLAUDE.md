# Abdulwahed's Design System — Green Edition

Design system reference for the Polaris Chronos frontend.

## Architecture

- **Tailwind CSS** (Play CDN) — all layout, spacing, typography, colors, backgrounds, borders, shadows, responsive breakpoints, hover/focus states via HTML utility classes
- **Lucide Icons** (CDN) — professional SVG icon set replacing emoji/inline SVG. Call `lucide.createIcons()` after dynamic DOM updates
- **style.css** (~350 lines) — CSS custom properties, JS-dependent class definitions, SVG dial styles, keyframe animations, toggle pseudo-elements, RTL overrides, responsive table overrides
- No Bootstrap — fully replaced by Tailwind

### Static Files

All frontend assets are embedded at compile time via `include_str!` in `src/server/static_files.rs`. After editing any file in `static/`, run:

```bash
touch src/server/static_files.rs   # Force recompile of embedded files
cargo build --release
```

### Deployment

- **Local**: `./target/release/polaris server --port 3000`
- **Docker**: Multi-stage build (`rust:1.83-slim` builder + `debian:bookworm-slim` runtime), port 7860
- **Hugging Face Spaces**: Docker SDK, auto-deploys on `git push hf main`
- **Live**: [abdulwahed-sweden-polaris-chronos.hf.space](https://abdulwahed-sweden-polaris-chronos.hf.space)

### Calendar Logic

- All code paths use `fetchGregorianMonth()` — the Hijri toggle only changes header display, never refetches data
- `findTodayIndex()` shared helper determines the starting row for week/day views
- `fetchRamadanMonth()` exists but is unused — kept for potential future use

---

## Design Tokens

### Colors — Background
| Token | Value | Usage |
|-------|-------|-------|
| `--bg-primary` | `#f8f9fb` | Page background (cool soft gray) |
| `--bg-surface` | `#ffffff` | Cards, tables, inputs |
| `--bg-secondary` | `#f1f3f5` | Table headers, subtle fills |

### Colors — Brand (Emerald Green)
| Token | Value | Usage |
|-------|-------|-------|
| `--color-accent` | `#059669` | Primary CTA, links, active states (Emerald 600) |
| `--color-accent-h` | `#047857` | Hover states (Emerald 700) |
| `--color-accent-lt` | `#d1fae5` | Light tint backgrounds (Emerald 100) |
| `--color-accent-dk` | `#065f46` | Deep accent, headings (Emerald 800) |

### Colors — Text (AAA Contrast)
| Token | Value | Ratio on #f8f9fb | Usage |
|-------|-------|-----------------|-------|
| `--text-primary` | `#1a1d23` | 15.8:1 | Body text |
| `--text-secondary` | `#4b5563` | 8.1:1 | Secondary labels |
| `--text-muted` | `#6b7280` | 5.6:1 | Captions, meta |

### Colors — Borders
| Token | Value |
|-------|-------|
| `--border-light` | `#e8eaed` |
| `--border-medium` | `#d1d5db` |

### Colors — Status
| Token | Value |
|-------|-------|
| `--success` | `#059669` |
| `--warning` | `#d97706` |
| `--danger` | `#dc2626` |
| `--info` | `#3b82f6` |

### Colors — Navbar
| Token | Value |
|-------|-------|
| Navbar bg | `#111318` |
| Navbar text | `#ffffff` |
| Navbar link hover | `rgba(255,255,255,0.7)` |
| Active indicator | `#059669` (green underline) |

---

## Typography

### Font Stack
| Role | Family | Weight | Fallback |
|------|--------|--------|----------|
| Body | `Inter` | 400/500/600 | system-ui, sans-serif |
| Headings | `Plus Jakarta Sans` | 700/800 | system-ui, sans-serif |
| Arabic | `Readex Pro` | 400–700 | sans-serif |
| Data/Mono | `JetBrains Mono` | 400/500 | monospace |

### Scale
| Element | Size | Weight | Notes |
|---------|------|--------|-------|
| Body | 15px | 400 | `--font-body` / Inter |
| H1 | 28px+ | 800 | Plus Jakarta Sans |
| H2 | 24px+ | 700 | Plus Jakarta Sans |
| H3 | 20px | 700 | Plus Jakarta Sans |
| Prayer times (table) | 18px | 700 | JetBrains Mono, `tabular-nums` |
| Labels/Meta | 13px | 600 | Uppercase, wide tracking |
| Minimum any text | 15px | — | AAA floor (except labels/badges) |

---

## Components

### Navbar
- Background: `#111318` (near-black)
- Text: white
- Active link: green underline (`#059669`), 2px bottom border
- Logo: white text, green star accent
- Sticky, z-index 1050

### Cards
- Background: `#ffffff`
- Border: `1px solid #e8eaed`
- Border-radius: `12px`
- Box-shadow: `0 1px 3px rgba(0,0,0,0.04)`

### Tables
- White background, rounded corners, subtle shadow
- Header: uppercase, bold, tracking-wide, `#f1f3f5` bg
- 9-column layout: Day | Gregorian | Hijri | Fajr | Sunrise | Dhuhr | Asr | Maghrib | Isha
- Zebra striping: `#f8f9fb` on even rows
- Friday rows: subtle emerald tint (`friday-row` class)
- Current prayer: green left border + `#d1fae5` bg
- Prayer times: 18px bold mono

### Buttons
- Primary: `bg #059669`, white text, hover `#047857`
- Border-radius: `10px`
- Font-weight: 600

---

## Rules
1. **No dark mode** — no `prefers-color-scheme`, no dark-mode toggle
2. **AAA contrast** — all text must pass WCAG AAA (7:1 for body, 4.5:1 for large)
3. **Minimum 15px** body font size (labels/badges can be 11–13px)
4. **Inter for body**, Plus Jakarta Sans for headings only
5. **Emerald Green** is the sole brand color — no gold/sand accents
6. **Tailwind for layout** — use utility classes in HTML, not CSS, for spacing/colors/typography
7. **Lucide for icons** — call `lucide.createIcons()` after any DOM update that adds `<i data-lucide>` elements
8. **Cache busting** — bump `?v=X.Y.Z` in index.html when updating CSS/JS to prevent stale browser cache
