import { reactive } from 'vue'

export interface LightboxState {
  open: boolean
  src: string
  alt: string
}

export const lightboxState = reactive<LightboxState>({
  open: false,
  src: '',
  alt: '',
})

function onKeydown(e: KeyboardEvent): void {
  if (e.key === 'Escape') {
    closeLightbox()
  }
}

export function openLightbox(src: string, alt: string): void {
  if (lightboxState.open) return
  lightboxState.src = src
  lightboxState.alt = alt
  lightboxState.open = true
  document.body.classList.add('iol-lightbox-lock')
  document.addEventListener('keydown', onKeydown)
}

export function closeLightbox(): void {
  if (!lightboxState.open) return
  lightboxState.open = false
  lightboxState.src = ''
  lightboxState.alt = ''
  document.body.classList.remove('iol-lightbox-lock')
  document.removeEventListener('keydown', onKeydown)
}

export function resetLightboxLock(): void {
  document.body.classList.remove('iol-lightbox-lock')
}