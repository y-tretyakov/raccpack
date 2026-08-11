import DefaultTheme from 'vitepress/theme'
import { h, onMounted, watch } from 'vue'
import { useRoute } from 'vitepress'
import mediumZoom from 'medium-zoom'
import SidebarBrand from './components/SidebarBrand.vue'
import './custom.css'

let zoom: { detach: () => void } | null = null

function initZoom(): void {
  zoom?.detach()
  zoom = mediumZoom('.vp-doc img:not(.VPImage)', {
    background: 'rgba(0, 0, 0, 0.85)',
    margin: 24,
  })
}

export default {
  extends: DefaultTheme,
  Layout: () =>
    h(DefaultTheme.Layout, null, {
      'sidebar-nav-before': () => h(SidebarBrand),
    }),
  setup(): void {
    const route = useRoute()
    onMounted(() => {
      initZoom()
      watch(() => route.path, () => initZoom())
    })
  },
}