<script setup lang="ts">
import { onBeforeUnmount } from 'vue'
import { lightboxState, closeLightbox, resetLightboxLock } from './lightbox-store'

onBeforeUnmount(resetLightboxLock)
</script>

<template>
  <Teleport to="body">
    <Transition name="iol-zoom">
      <div
        v-if="lightboxState.open"
        class="iol-lightbox"
        role="dialog"
        aria-modal="true"
        title="Закрыть"
        @click="closeLightbox"
      >
        <img :src="lightboxState.src" :alt="lightboxState.alt" @click="closeLightbox" />
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
  padding: 32px;
  background: rgba(0, 0, 0, 0.9);
  cursor: zoom-out;
}

.iol-lightbox img {
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
  border-radius: 4px;
  box-shadow: 0 24px 64px rgba(0, 0, 0, 0.5);
  cursor: zoom-out;
}

.iol-zoom-enter-active,
.iol-zoom-leave-active {
  transition: opacity 0.25s ease;
}

.iol-zoom-enter-from,
.iol-zoom-leave-to {
  opacity: 0;
}
</style>