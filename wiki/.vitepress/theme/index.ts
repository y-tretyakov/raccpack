import DefaultTheme from 'vitepress/theme'
import { h } from 'vue'
import SidebarBrand from './components/SidebarBrand.vue'
import './custom.css'

export default {
  extends: DefaultTheme,
  Layout: () =>
    h(DefaultTheme.Layout, null, {
      'sidebar-nav-before': () => h(SidebarBrand),
    }),
}