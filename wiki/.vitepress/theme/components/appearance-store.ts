import { reactive, watch } from 'vue'

export type TextSize = 'small' | 'standard' | 'large'
export type ContentWidth = 'small' | 'standard' | 'wide'
export type ColorMode = 'automatic' | 'light' | 'dark'

export interface AppearanceState {
  text: TextSize
  width: ContentWidth
  color: ColorMode
}

const STORAGE_KEY = 'raccpack-wiki-appearance'

const TEXT_SIZES: TextSize[] = ['small', 'standard', 'large']
const WIDTHS: ContentWidth[] = ['small', 'standard', 'wide']
const COLORS: ColorMode[] = ['automatic', 'light', 'dark']

const defaults: AppearanceState = {
  text: 'standard',
  width: 'standard',
  color: 'dark',
}

function load(): AppearanceState {
  if (typeof localStorage === 'undefined') {
    return { ...defaults }
  }
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (!raw) {
      return { ...defaults }
    }
    const parsed = JSON.parse(raw) as Partial<AppearanceState>
    return {
      text: TEXT_SIZES.includes(parsed.text as TextSize)
        ? (parsed.text as TextSize)
        : defaults.text,
      width: WIDTHS.includes(parsed.width as ContentWidth)
        ? (parsed.width as ContentWidth)
        : defaults.width,
      color: COLORS.includes(parsed.color as ColorMode)
        ? (parsed.color as ColorMode)
        : defaults.color,
    }
  } catch {
    return { ...defaults }
  }
}

export const appearance = reactive<AppearanceState>(load())

function prefersDark(): boolean {
  return (
    typeof window.matchMedia === 'function' &&
    window.matchMedia('(prefers-color-scheme: dark)').matches
  )
}

export function applyAppearance(state: AppearanceState = appearance): void {
  if (typeof document === 'undefined') {
    return
  }
  const root = document.documentElement
  root.dataset.denText = state.text
  root.dataset.denWidth = state.width

  if (state.color === 'dark') {
    root.classList.add('dark')
    root.classList.remove('light')
  } else if (state.color === 'light') {
    root.classList.remove('dark')
    root.classList.add('light')
  } else if (prefersDark()) {
    root.classList.add('dark')
    root.classList.remove('light')
  } else {
    root.classList.remove('dark')
    root.classList.remove('light')
  }
}

function onSystemThemeChange(): void {
  if (appearance.color === 'automatic') {
    applyAppearance()
  }
}

function onStorageChange(event: StorageEvent): void {
  if (event.key !== STORAGE_KEY) {
    return
  }
  try {
    const next = JSON.parse(event.newValue ?? '{}') as Partial<AppearanceState>
    if (TEXT_SIZES.includes(next.text as TextSize)) {
      appearance.text = next.text as TextSize
    }
    if (WIDTHS.includes(next.width as ContentWidth)) {
      appearance.width = next.width as ContentWidth
    }
    if (COLORS.includes(next.color as ColorMode)) {
      appearance.color = next.color as ColorMode
    }
  } catch {
    /* ignore malformed storage payload from another tab */
  }
}

export function initAppearance(): void {
  if (typeof window === 'undefined') {
    return
  }
  applyAppearance()
  const media = window.matchMedia('(prefers-color-scheme: dark)')
  if (typeof media.addEventListener === 'function') {
    media.addEventListener('change', onSystemThemeChange)
  } else if (typeof (media as { addListener?: unknown }).addListener === 'function') {
    ;(media as unknown as { addListener: (fn: () => void) => void }).addListener(
      onSystemThemeChange,
    )
  }
  window.addEventListener('storage', onStorageChange)
}

watch(
  appearance,
  (state) => {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(state))
    } catch {
      /* storage unavailable */
    }
    applyAppearance(state)
  },
  { deep: true },
)
