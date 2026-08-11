import DefaultTheme from 'vitepress/theme'
import { h, onBeforeUnmount, onMounted } from 'vue'
import SidebarBrand from './components/SidebarBrand.vue'
import ImageLightbox from './components/ImageLightbox.vue'
import { openLightbox } from './components/lightbox-store'
import './custom.css'

function onDocumentClick(e: MouseEvent): void {
  const target = e.target as HTMLElement | null
  if (!target || !(target instanceof HTMLImageElement)) return
  if (!target.closest('.vp-doc')) return
  if (target.classList.contains('VPImage')) return
  e.preventDefault()
  e.stopPropagation()
  openLightbox(target.currentSrc || target.src, target.alt || '')
}

export default {
  extends: DefaultTheme,
  Layout: () =>
    h(DefaultTheme.Layout, null, {
      'sidebar-nav-before': () => h(SidebarBrand),
      'layout-bottom': () => h(ImageLightbox),
    }),
  setup(): void {
    onMounted(() => document.addEventListener('click', onDocumentClick))
    onBeforeUnmount(() => document.removeEventListener('click', onDocumentClick))
  },
}