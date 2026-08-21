import DefaultTheme from 'vitepress/theme'
import { h, nextTick, onBeforeUnmount, onMounted, watch } from 'vue'
import { useRoute } from 'vitepress'
import SidebarBrand from './components/SidebarBrand.vue'
import ImageLightbox from './components/ImageLightbox.vue'
import DenPipeline from './components/DenPipeline.vue'
import RiskBadge from './components/RiskBadge.vue'
import AppearanceTrigger from './components/AppearanceTrigger.vue'
import DenPlexus from './components/DenPlexus.vue'
import { initAppearance } from './components/appearance-store'
import { lightboxState, openLightbox } from './components/lightbox-store'
import './custom.css'

const ICON_SVG =
  '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="7"/><path d="m21 21-4.35-4.35"/><path d="M12 8.5v7M8.5 12h7"/></svg>'

const NOWRAP_HEADERS = new Set([
  'Параметр',
  'Флаг',
  'Команда',
  'Parameter',
  'Flag',
  'Command',
])

function tagDocImages(): void {
  document
    .querySelectorAll<HTMLImageElement>('.vp-doc img:not(.VPImage)')
    .forEach((img) => {
      if (img.closest('.iol-wrap') || img.closest('a')) return
      const wrap = document.createElement('span')
      wrap.className = 'iol-wrap'
      const icon = document.createElement('span')
      icon.className = 'iol-icon'
      icon.innerHTML = ICON_SVG
      wrap.appendChild(icon)
      img.replaceWith(wrap)
      wrap.appendChild(img)
    })
}

function tagTables(): void {
  document
    .querySelectorAll<HTMLTableElement>('.vp-doc table')
    .forEach((table) => {
      if (!table.closest('.den-table-wrap')) {
        const wrap = document.createElement('div')
        wrap.className = 'den-table-wrap'
        table.replaceWith(wrap)
        wrap.appendChild(table)
      }
      table.querySelectorAll<HTMLTableCellElement>('th').forEach((th, index) => {
        const headerText = th.textContent?.trim() ?? ''
        if (!NOWRAP_HEADERS.has(headerText)) return
        const column = index + 1
        table
          .querySelectorAll<HTMLTableCellElement>(
            `thead th:nth-child(${column}), tbody td:nth-child(${column})`,
          )
          .forEach((cell) => cell.classList.add('den-nowrap-cell'))
      })
    })
}

function onDocumentClick(e: MouseEvent): void {
  const target = e.target
  if (!(target instanceof Element)) return
  if (lightboxState.open) return
  const wrap = target.closest('.iol-wrap')
  const img = wrap
    ? wrap.querySelector<HTMLImageElement>('img')
    : target instanceof HTMLImageElement
      ? target
      : null
  if (!img) return
  if (!img.closest('.vp-doc')) return
  if (img.classList.contains('VPImage')) return
  if (img.closest('a')) return
  e.preventDefault()
  e.stopPropagation()
  openLightbox(img)
}

export default {
  extends: DefaultTheme,
  Layout: () =>
    h(DefaultTheme.Layout, null, {
      'sidebar-nav-before': () => h(SidebarBrand),
      'nav-bar-content-after': () => h(AppearanceTrigger),
      'home-hero-before': () => h(DenPlexus),
      'layout-bottom': () => h(ImageLightbox),
    }),
  enhanceApp({ app }) {
    app.component('DenPipeline', DenPipeline)
    app.component('RiskBadge', RiskBadge)
  },
  setup(): void {
    const route = useRoute()
    const tag = () =>
      nextTick(() => {
        requestAnimationFrame(() => {
          tagDocImages()
          tagTables()
        })
      })
    onMounted(() => {
      document.addEventListener('click', onDocumentClick)
      initAppearance()
      tag()
    })
    watch(() => route.path, tag)
    onBeforeUnmount(() => document.removeEventListener('click', onDocumentClick))
  },
}