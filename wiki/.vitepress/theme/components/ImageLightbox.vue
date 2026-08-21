<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'
import { useData } from 'vitepress'
import {
  lightboxState,
  requestClose,
  finishClose,
  resetLightboxLock,
} from './lightbox-store'

const { lang } = useData()
const closeLabel = computed(() =>
  lang.value.startsWith('en') ? 'Close' : 'Закрыть',
)

const MARGIN = 48

const visible = ref(false)
const flipped = ref(false)
const imgEl = ref<HTMLImageElement | null>(null)
const imgStyle = ref<Record<string, string>>({})

let fromRect = { top: 0, left: 0, width: 0, height: 0 }
let naturalWidth = 0
let naturalHeight = 0

watch(
  () => lightboxState.open,
  (open) => {
    if (open) {
      const rect = lightboxState.fromRect
      fromRect = rect ?? fromRect
      naturalWidth = lightboxState.naturalWidth
      naturalHeight = lightboxState.naturalHeight
      flipped.value = Boolean(rect && naturalWidth && naturalHeight)
      void enter()
    } else {
      leave()
    }
  },
)

async function enter(): Promise<void> {
  if (!flipped.value) {
    visible.value = true
    return
  }

  const maxW = window.innerWidth - MARGIN * 2
  const maxH = window.innerHeight - MARGIN * 2
  const scale = Math.min(maxW / naturalWidth, maxH / naturalHeight, 1)
  const w = Math.round(naturalWidth * scale)
  const h = Math.round(naturalHeight * scale)
  const left = Math.round((window.innerWidth - w) / 2)
  const top = Math.round((window.innerHeight - h) / 2)

  imgStyle.value = {
    position: 'absolute',
    width: `${w}px`,
    height: `${h}px`,
    left: `${left}px`,
    top: `${top}px`,
  }
  visible.value = true

  await nextTick()
  const img = imgEl.value
  if (!img) return

  const sx = fromRect.width / w
  const sy = fromRect.height / h
  const tx = fromRect.left - left
  const ty = fromRect.top - top

  img.style.transition = 'none'
  img.style.transform = `translate(${tx}px, ${ty}px) scale(${sx}, ${sy})`
  void img.offsetWidth
  img.style.transition = 'transform 0.4s cubic-bezier(0.2, 0.8, 0.2, 1), opacity 0.25s ease'
  img.style.transform = 'translate(0px, 0px) scale(1, 1)'
}

function leave(): void {
  const img = imgEl.value
  if (img && flipped.value) {
    const w = parseFloat(imgStyle.value.width || '0')
    const h = parseFloat(imgStyle.value.height || '0')
    const left = parseFloat(imgStyle.value.left || '0')
    const top = parseFloat(imgStyle.value.top || '0')
    if (w && h) {
      const sx = fromRect.width / w
      const sy = fromRect.height / h
      img.style.transition = 'transform 0.35s cubic-bezier(0.4, 0, 0.2, 1), opacity 0.25s ease'
      img.style.transform = `translate(${fromRect.left - left}px, ${fromRect.top - top}px) scale(${sx}, ${sy})`
    }
  }
  requestAnimationFrame(() => {
    visible.value = false
  })
}

function onAfterLeave(): void {
  finishClose()
}

function onOverlayClick(): void {
  requestClose()
}

onBeforeUnmount(resetLightboxLock)
</script>

<template>
  <Teleport to="body">
    <Transition name="iol-overlay" @after-leave="onAfterLeave">
      <div
        v-if="visible"
        class="iol-lightbox"
        role="dialog"
        aria-modal="true"
        :aria-label="closeLabel"
        @click="onOverlayClick"
      >
        <img
          ref="imgEl"
          class="iol-image"
          :class="{ 'iol-image--flip': flipped }"
          :style="imgStyle"
          :src="lightboxState.src"
          :alt="lightboxState.alt"
          @click.stop="onOverlayClick"
        />
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.iol-lightbox {
  position: fixed;
  inset: 0;
  z-index: 9999;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 48px;
  background: rgba(0, 0, 0, 0.9);
  cursor: zoom-out;
}

.iol-image {
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
  user-select: none;
  -webkit-user-drag: none;
}

.iol-image--flip {
  transform-origin: 0 0;
}

.iol-overlay-enter-active {
  transition: opacity 0.3s ease;
}

.iol-overlay-leave-active {
  transition: opacity 0.35s ease;
}

.iol-overlay-enter-from,
.iol-overlay-leave-to {
  opacity: 0;
}
</style>