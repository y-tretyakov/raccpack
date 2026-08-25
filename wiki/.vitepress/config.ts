import { defineConfig } from 'vitepress'
import type { DefaultTheme } from 'vitepress'

const en = {
  nav: [
    { text: 'Introduction', link: '/introduction' },
    {
      text: 'Getting started',
      items: [
        { text: 'Installation', link: '/installation' },
        { text: 'Quick start', link: '/quick-start' },
      ],
    },
    {
      text: 'Usage',
      items: [
        { text: 'CLI usage', link: '/cli-usage' },
        { text: 'Sniff', link: '/sniff' },
        { text: 'Dig', link: '/dig' },
        { text: 'Pack', link: '/pack' },
        { text: 'Stash', link: '/stash' },
        { text: 'Rinse', link: '/rinse' },
        { text: 'Raid', link: '/raid' },
        { text: 'Init', link: '/init' },
        { text: 'Git, init & DX', link: '/git-and-dx' },
        { text: 'Configuration', link: '/configuration' },
        { text: 'Cookbook — scenarios & scripts', link: '/cookbook' },
        { text: 'TUI', link: '/tui-usage' },
        { text: 'Desktop', link: '/desktop-usage' },
      ],
    },
    {
      text: 'Reference',
      items: [
        { text: 'Concepts', link: '/concepts' },
        { text: 'Supported catalog', link: '/supported' },
        { text: 'Architecture', link: '/architecture' },
        { text: 'Facade API', link: '/facade-api' },
      ],
    },
    { text: 'Roadmap', link: '/roadmap' },
    { text: 'Troubleshooting', link: '/troubleshooting' },
  ],
  sidebar: [
    {
      text: 'Introduction',
      items: [{ text: 'Introduction', link: '/introduction' }],
    },
    {
      text: 'Getting started',
      items: [
        { text: 'Installation', link: '/installation' },
        { text: 'Quick start', link: '/quick-start' },
      ],
    },
    {
      text: 'Usage',
      items: [
        { text: 'CLI', link: '/cli-usage' },
        { text: 'Sniff', link: '/sniff' },
        { text: 'Dig', link: '/dig' },
        { text: 'Pack', link: '/pack' },
        { text: 'Stash', link: '/stash' },
        { text: 'Rinse', link: '/rinse' },
        { text: 'Raid', link: '/raid' },
        { text: 'Init', link: '/init' },
        { text: 'Git, init & DX', link: '/git-and-dx' },
        { text: 'Configuration', link: '/configuration' },
        { text: 'Cookbook — scenarios & scripts', link: '/cookbook' },
        { text: 'TUI', link: '/tui-usage' },
        { text: 'Desktop', link: '/desktop-usage' },
      ],
    },
    {
      text: 'Reference',
      items: [
        { text: 'Concepts', link: '/concepts' },
        { text: 'Supported catalog', link: '/supported' },
        { text: 'Architecture', link: '/architecture' },
        { text: 'Facade API', link: '/facade-api' },
      ],
    },
    { text: 'Roadmap', link: '/roadmap' },
    { text: 'Troubleshooting', link: '/troubleshooting' },
  ],
} satisfies DefaultTheme.Config

const rus = {
  nav: [
    { text: 'Введение', link: '/ru/introduction' },
    {
      text: 'Начало работы',
      items: [
        { text: 'Установка', link: '/ru/installation' },
        { text: 'Быстрый старт', link: '/ru/quick-start' },
      ],
    },
    {
      text: 'Использование',
      items: [
        { text: 'Использование CLI', link: '/ru/cli-usage' },
        { text: 'Sniff', link: '/ru/sniff' },
        { text: 'Dig', link: '/ru/dig' },
        { text: 'Pack', link: '/ru/pack' },
        { text: 'Stash', link: '/ru/stash' },
        { text: 'Rinse', link: '/ru/rinse' },
        { text: 'Raid', link: '/ru/raid' },
        { text: 'Init', link: '/ru/init' },
        { text: 'Git, init и DX', link: '/ru/git-and-dx' },
        { text: 'Конфигурация', link: '/ru/configuration' },
        { text: 'Cookbook — сценарии и скрипты', link: '/ru/cookbook' },
        { text: 'TUI', link: '/ru/tui-usage' },
        { text: 'Desktop', link: '/ru/desktop-usage' },
      ],
    },
    {
      text: 'Справочник',
      items: [
        { text: 'Основные понятия', link: '/ru/concepts' },
        { text: 'Что поддерживается', link: '/ru/supported' },
        { text: 'Архитектура', link: '/ru/architecture' },
        { text: 'Facade API', link: '/ru/facade-api' },
      ],
    },
    { text: 'Дорожная карта', link: '/ru/roadmap' },
    { text: 'Устранение неполадок', link: '/ru/troubleshooting' },
  ],
  sidebar: [
    {
      text: 'Введение',
      items: [{ text: 'Введение', link: '/ru/introduction' }],
    },
    {
      text: 'Начало работы',
      items: [
        { text: 'Установка', link: '/ru/installation' },
        { text: 'Быстрый старт', link: '/ru/quick-start' },
      ],
    },
    {
      text: 'Использование',
      items: [
        { text: 'CLI', link: '/ru/cli-usage' },
        { text: 'Sniff', link: '/ru/sniff' },
        { text: 'Dig', link: '/ru/dig' },
        { text: 'Pack', link: '/ru/pack' },
        { text: 'Stash', link: '/ru/stash' },
        { text: 'Rinse', link: '/ru/rinse' },
        { text: 'Raid', link: '/ru/raid' },
        { text: 'Init', link: '/ru/init' },
        { text: 'Git, init и DX', link: '/ru/git-and-dx' },
        { text: 'Конфигурация', link: '/ru/configuration' },
        { text: 'Cookbook — сценарии и скрипты', link: '/ru/cookbook' },
        { text: 'TUI', link: '/ru/tui-usage' },
        { text: 'Desktop', link: '/ru/desktop-usage' },
      ],
    },
    {
      text: 'Справочник',
      items: [
        { text: 'Основные понятия', link: '/ru/concepts' },
        { text: 'Что поддерживается', link: '/ru/supported' },
        { text: 'Архитектура', link: '/ru/architecture' },
        { text: 'Facade API', link: '/ru/facade-api' },
      ],
    },
    { text: 'Дорожная карта', link: '/ru/roadmap' },
    { text: 'Устранение неполадок', link: '/ru/troubleshooting' },
  ],
} satisfies DefaultTheme.Config

export default defineConfig({
  title: 'raccpack',
  description:
    'Tool for scanning projects, finding secrets, cleaning build trash, and packing projects into a den.',
  base: '/raccpack/',
  cleanUrls: false,
  head: [
    ['link', { rel: 'icon', href: '/raccpack/RP.webp', type: 'image/webp' }],
    [
      'script',
      {},
      `(function(){try{var s=JSON.parse(localStorage.getItem('raccpack-wiki-appearance')||'{}');var r=document.documentElement;if(s.text)r.dataset.denText=s.text;if(s.width)r.dataset.denWidth=s.width;var c=s.color||'dark';if(c==='dark'){r.classList.add('dark')}else if(c==='light'){r.classList.remove('dark')}else{var d=window.matchMedia&&window.matchMedia('(prefers-color-scheme: dark)').matches;r.classList.toggle('dark',!!d)}}catch(e){}})();`,
    ],
  ],
  lastUpdated: true,
  markdown: {
    lineNumbers: true,
    container: {
      tipLabel: 'TIP',
      warningLabel: 'WARNING',
      dangerLabel: 'DANGER',
      infoLabel: 'INFO',
      detailsLabel: 'Details'
    }
  },
  locales: {
    root: {
      label: 'English',
      lang: 'en-US',
      link: '/',
      title: 'raccpack',
      description:
        'Tool for scanning projects, finding secrets, cleaning build trash, and packing projects into a den.',
      themeConfig: {
        ...en,
        outline: { label: 'On this page' },
        docFooter: { prev: 'Previous page', next: 'Next page' },
        lastUpdated: { text: 'Last updated' },
      },
    },
    ru: {
      label: 'Русский',
      lang: 'ru-RU',
      link: '/ru/',
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
  },
  appearance: false,
  themeConfig: {
    i18nRouting: false,
    logo: '/RP.webp',
    siteTitle: 'raccpack',
    socialLinks: [
      { icon: 'github', link: 'https://github.com/y-tretyakov/raccpack' },
    ],
    search: { provider: 'local' },
  },
})
