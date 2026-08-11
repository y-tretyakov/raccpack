import { reactive } from 'vue'

export interface Rect {
  top: number
  left: number
  width: number
  height: number
}

export interface LightboxState {
  open: boolean
  src: string
  alt: string
  fromRect: Rect | null
  naturalWidth: number
  naturalHeight: number
}

export const lightboxState = reactive<LightboxState>({
  open: false,
  src: '',
  alt: '',
  fromRect: null,
  naturalWidth: 0,
  naturalHeight: 0,
})

function onKeydown(e: KeyboardEvent): void {
  if (e.key === 'Escape') {
    requestClose()
  }
}

export function openLightbox(img: HTMLImageElement): void {
  if (lightboxState.open) return
  const rect = img.getBoundingClientRect()
  lightboxState.src = img.currentSrc || img.src
  lightboxState.alt = img.alt || ''
  lightboxState.fromRect = {
    top: rect.top,
    left: rect.left,
    width: rect.width,
    height: rect.height,
  }
  lightboxState.naturalWidth = img.naturalWidth || 0
  lightboxState.naturalHeight = img.naturalHeight || 0
  lightboxState.open = true
  document.body.classList.add('iol-lightbox-lock')
  document.addEventListener('keydown', onKeydown)
}

export function requestClose(): void {
  if (!lightboxState.open) return
  lightboxState.open = false
  document.removeEventListener('keydown', onKeydown)
}

export function finishClose(): void {
  lightboxState.src = ''
  lightboxState.alt = ''
  lightboxState.fromRect = null
  lightboxState.naturalWidth = 0
  lightboxState.naturalHeight = 0
  document.body.classList.remove('iol-lightbox-lock')
}

export function resetLightboxLock(): void {
  document.body.classList.remove('iol-lightbox-lock')
  document.removeEventListener('keydown', onKeydown)
}