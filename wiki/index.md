---
layout: home
title: raccpack
hero:
  name: raccpack
  text: Современный инструмент безопасного бэкапа проектов
  tagline: Секреты — в зашифрованные age-архивы, мусор сборки — вон, каждый проект — в чистый tar.zst в den.
  image:
    src: /RaccPack.webp
    alt: raccpack
  actions:
    - theme: brand
      text: Пайплайн команд
      link: /concepts
    - theme: alt
      text: Быстрый старт
      link: /quick-start
    - theme: alt
      text: Wiki
      link: /introduction
features:
  - title: Rust
    details: Быстрое, надёжное, безопасное ядро raccpack-core — один код для всех интерфейсов.
  - title: age
    details: Секреты шифруются стандартом age — паролем или recipient-ключами; raw-значения живут в памяти только на время шифрования.
  - title: tar.zst
    details: Каждый проект пакуется в чистый tar.zst без секретов и мусора сборки.
  - title: CLI · TUI · Desktop
    details: Одна бизнес-логика, три интерфейса. Сейчас в CLI доступны sniff, dig и pack, остальные команды — по roadmap.
  - title: Безопасность по умолчанию
    details: Секреты маскируются в отчётах, dry-run перед разрушающими операциями.
  - title: Den — хранилище
    details: Архивы проектов, а позже — зашифрованные секреты и JSON-манифесты операций.
---

## Зачем это нужно

Разработчики держат в рабочей папке десятки проектов — с `.env`-файлами, SSH-ключами и каталогами сборки на гигабайты. Копировать такую папку в бэкап как есть — значит утечь секреты и переслать тонны мусора. raccpack автоматизирует наведение порядка перед упаковкой.

## Пайплайн

```
sniff  →  dig  →  stash  →  rinse  →  pack  →  raid
```

## Что дальше

- [Установка](/installation) — собрать и проверить `racc`.
- [Быстрый старт](/quick-start) — первый прогон за пять минут.
- [Основные понятия](/concepts) — den, секреты, риски, фазы.