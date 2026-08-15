import { defineConfig } from 'vitepress'
import type { DefaultTheme } from 'vitepress'

const rus = {
  nav: [
    { text: 'Введение', link: '/introduction' },
    {
      text: 'Начало работы',
      items: [
        { text: 'Установка', link: '/installation' },
        { text: 'Быстрый старт', link: '/quick-start' },
      ],
    },
    {
      text: 'Использование',
      items: [
        { text: 'Использование CLI', link: '/cli-usage' },
        { text: 'Stash — секреты в age', link: '/stash' },
        { text: 'Конфигурация', link: '/configuration' },
        { text: 'TUI', link: '/tui-usage' },
        { text: 'Desktop', link: '/desktop-usage' },
      ],
    },
    {
      text: 'Справочник',
      items: [
        { text: 'Основные понятия', link: '/concepts' },
        { text: 'Что поддерживается', link: '/supported' },
        { text: 'Архитектура', link: '/architecture' },
        { text: 'Facade API', link: '/facade-api' },
      ],
    },
    { text: 'Дорожная карта', link: '/roadmap' },
    { text: 'Устранение неполадок', link: '/troubleshooting' },
  ],
  sidebar: [
    {
      text: 'Введение',
      items: [{ text: 'Введение', link: '/introduction' }],
    },
    {
      text: 'Начало работы',
      items: [
        { text: 'Установка', link: '/installation' },
        { text: 'Быстрый старт', link: '/quick-start' },
      ],
    },
    {
      text: 'Использование',
      items: [
        { text: 'CLI', link: '/cli-usage' },
        { text: 'Stash — секреты в age', link: '/stash' },
        { text: 'Конфигурация', link: '/configuration' },
        { text: 'TUI', link: '/tui-usage' },
        { text: 'Desktop', link: '/desktop-usage' },
      ],
    },
    {
      text: 'Справочник',
      items: [
        { text: 'Основные понятия', link: '/concepts' },
        { text: 'Что поддерживается', link: '/supported' },
        { text: 'Архитектура', link: '/architecture' },
        { text: 'Facade API', link: '/facade-api' },
      ],
    },
    { text: 'Дорожная карта', link: '/roadmap' },
    { text: 'Устранение неполадок', link: '/troubleshooting' },
  ],
} satisfies DefaultTheme.Config

const eng = {
  nav: [
    { text: 'Introduction', link: '/en/introduction' },
    { text: 'Supported', link: '/en/supported' },
    { text: 'Русская версия', link: '/' },
  ],
  sidebar: [
    {
      text: 'First steps',
      items: [{ text: 'Introduction', link: '/en/introduction' }],
    },
    {
      text: 'Reference',
      items: [{ text: 'Supported', link: '/en/supported' }],
    },
  ],
} satisfies DefaultTheme.Config

export default defineConfig({
  lang: 'ru-RU',
  title: 'raccpack',
  description:
    'Инструмент для поиска секретов, очистки мусора сборки и упаковки проектов в защищённое хранилище den.',
  base: '/raccpack/',
  cleanUrls: false,
  head: [
    ['link', { rel: 'icon', href: '/raccpack/favicon.ico', type: 'image/x-icon' }],
  ],
  lastUpdated: true,
  markdown: {
    container: {
      tipLabel: 'СОВЕТ',
      warningLabel: 'ПРЕДУПРЕЖДЕНИЕ',
      dangerLabel: 'ОПАСНОСТЬ',
      infoLabel: 'ИНФО',
      detailsLabel: 'Дополнительно'
    }
  },
  locales: {
    root: {
      label: 'Русский',
      lang: 'ru-RU',
      link: '/',
      title: 'raccpack',
      description:
        'Инструмент для поиска секретов, очистки мусора сборки и упаковки проектов в защищённое хранилище den.',
      themeConfig: {
        ...rus,
        outline: { label: 'На этой странице' },
        docFooter: { prev: 'Предыдущая страница', next: 'Следующая страница' },
        lastUpdated: { text: 'Обновлено' },
      },
    },
    en: {
      label: 'English',
      lang: 'en-US',
      link: '/en/',
      title: 'raccpack',
      description:
        'Tool for scanning projects, finding secrets, cleaning build trash, and packing projects into a den.',
      themeConfig: {
        ...eng,
        outline: { label: 'On this page' },
        docFooter: { prev: 'Previous page', next: 'Next page' },
        lastUpdated: { text: 'Last updated' },
      },
    },
  },
  themeConfig: {
    appearance: 'dark',
    i18nRouting: false,
    logo: '/logo.webp',
    siteTitle: 'raccpack',
    socialLinks: [
      { icon: 'github', link: 'https://github.com/y-tretyakov/raccpack' },
    ],
    search: { provider: 'local' },
  },
})