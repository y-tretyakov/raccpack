<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref } from 'vue'

/* =====================================================================
 * Единый конфиг plexus — все параметры здесь (DEFAULTS ниже).
 * Правки вносятся в DEFAULTS и применяются при пересборке/перезапуске.
 * Для live-проб без перезапуска можно менять из консоли браузера:
 *
 *   window.__DEN_PLEXUS__.set({ pointCountMin: 200, cursorRadius: 220 })
 *   window.__DEN_PLEXUS__.set({ speedMin: 0.2, speedMax: 0.6 })
 *   window.__DEN_PLEXUS__.reset()          // вернуть DEFAULTS
 *
 * Цвета задаются строкой RGB «r, g, b» (без скобок).
 * ===================================================================== */

interface DenPlexusConfig {
  desktopMin: number
  dprCap: number
  speedMin: number
  speedMax: number
  maxSpeedFactor: number
  relax: number
  pushForce: number
  swirl: number
  pointRMin: number
  pointRMax: number
  pointCountMin: number
  pointCountMax: number
  brandChance: number
  pointAlphaMin: number
  pointAlphaMax: number
  neutralAlpha: number
  linkDistMin: number
  linkDistMax: number
  linkAlphaBase: number
  linkAlphaFade: number
  linkWidthBase: number
  linkWidthFade: number
  linkGlowAlpha: number
  cursorRadius: number
  mouseFadeMs: number
  maxFlashes: number
  flashDurMin: number
  flashDurMax: number
  flashPeakMin: number
  flashPeakMax: number
  flashPointHalo: number
  flashPointBoost: number
  brand: string
  neutral: string
  canvasOpacity: number
  resizeDebounceMs: number
}

const DEFAULTS: DenPlexusConfig = {
  desktopMin: 960,
  dprCap: 1.75,
  speedMin: 0.1,
  speedMax: 0.4,
  maxSpeedFactor: 4,
  relax: 0.004,
  pushForce: 0.12,
  swirl: 0.025,
  pointRMin: 1.2,
  pointRMax: 2.2,
  pointCountMin: 80,
  pointCountMax: 145,
  brandChance: 0.72,
  pointAlphaMin: 0.6,
  pointAlphaMax: 0.95,
  neutralAlpha: 0.4,
  linkDistMin: 110,
  linkDistMax: 150,
  linkAlphaBase: 0.3,
  linkAlphaFade: 0.32,
  linkWidthBase: 0.8,
  linkWidthFade: 0.7,
  linkGlowAlpha: 0.22,
  cursorRadius: 140,
  mouseFadeMs: 400,
  maxFlashes: 14,
  flashDurMin: 220,
  flashDurMax: 450,
  flashPeakMin: 0.55,
  flashPeakMax: 0.75,
  flashPointHalo: 0.8,
  flashPointBoost: 0.75,
  brand: '201, 108, 44',
  neutral: '232, 234, 239',
  canvasOpacity: 0.95,
  resizeDebounceMs: 120,
}

interface Particle {
  x: number
  y: number
  vx: number
  vy: number
  cruise: number
  r: number
  brand: boolean
  alpha: number
}

interface Flash {
  i: number
  j: number
  born: number
  duration: number
  peak: number
  alive: boolean
}

declare global {
  interface Window {
    __DEN_PLEXUS__?: {
      set(patch: Partial<DenPlexusConfig>): void
      reset(): void
      get(): DenPlexusConfig
    }
  }
}

const enabled = ref(false)
const wrapRef = ref<HTMLElement | null>(null)
const canvasRef = ref<HTMLCanvasElement | null>(null)

const particles: Particle[] = []
const flashes: Flash[] = []
const prevLinks = new Set<number>()
const flashEnv = new Map<number, number>()
const flashPeak = new Map<number, number>()
const flashPoints = new Map<number, number>()
const flashPointPeak = new Map<number, number>()

let ctx: CanvasRenderingContext2D | null = null
let raf = 0
let running = false
let cssW = 0
let cssH = 0
let dpr = 1
let linkDist = 120
let lastTs = 0
let mouseX = 0
let mouseY = 0
let mouseInside = false
let mouseInfluence = 0
let resizeTimer = 0
let config: DenPlexusConfig = { ...DEFAULTS }
let settleLinks = true
let reducedMq: MediaQueryList | null = null
let themeObserver: MutationObserver | null = null
let boxObserver: ResizeObserver | null = null
let runtimeListening = false

function clamp01(t: number): number {
  return t < 0 ? 0 : t > 1 ? 1 : t
}

function applyCanvasOpacity(): void {
  const canvas = canvasRef.value
  if (canvas) canvas.style.opacity = String(config.canvasOpacity)
}

function exposeApi(): void {
  window.__DEN_PLEXUS__ = {
    set(patch) {
      Object.assign(config, patch)
      seed(cssW, cssH)
    },
    reset() {
      config = { ...DEFAULTS }
      seed(cssW, cssH)
    },
    get() {
      return { ...config }
    },
  }
}

function shouldRun(): boolean {
  if (typeof window === 'undefined') return false
  if (window.innerWidth < config.desktopMin) return false
  if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
    return false
  }
  return document.documentElement.classList.contains('dark')
}

function particleCount(width: number): number {
  const t = clamp01((width - 1024) / (1600 - 1024))
  return Math.round(
    config.pointCountMin + t * (config.pointCountMax - config.pointCountMin),
  )
}

function linkDistance(width: number): number {
  const t = clamp01((width - 1024) / (1600 - 1024))
  return config.linkDistMin + t * (config.linkDistMax - config.linkDistMin)
}

function pairKey(i: number, j: number): number {
  return i < j ? i * 4096 + j : j * 4096 + i
}

function ensureFlashSlots(): void {
  while (flashes.length < config.maxFlashes) {
    flashes.push({ i: 0, j: 0, born: 0, duration: 0, peak: 0, alive: false })
  }
  flashes.length = config.maxFlashes
}

function seed(width: number, height: number): void {
  applyCanvasOpacity()
  linkDist = linkDistance(width)
  const n = particleCount(width)
  while (particles.length < n) {
    particles.push({
      x: 0,
      y: 0,
      vx: 0,
      vy: 0,
      cruise: 0,
      r: 1.5,
      brand: true,
      alpha: 0.6,
    })
  }
  particles.length = n
  for (let i = 0; i < n; i++) {
    const p = particles[i]
    const speed = config.speedMin + Math.random() * (config.speedMax - config.speedMin)
    const angle = Math.random() * Math.PI * 2
    p.x = Math.random() * width
    p.y = Math.random() * height
    p.vx = Math.cos(angle) * speed
    p.vy = Math.sin(angle) * speed
    p.cruise = speed
    p.r = config.pointRMin + Math.random() * (config.pointRMax - config.pointRMin)
    p.brand = Math.random() < config.brandChance
    p.alpha = config.pointAlphaMin + Math.random() * (config.pointAlphaMax - config.pointAlphaMin)
  }
  ensureFlashSlots()
  for (const flash of flashes) flash.alive = false
  prevLinks.clear()
  settleLinks = true
  flashEnv.clear()
  flashPeak.clear()
  flashPoints.clear()
  flashPointPeak.clear()
}

function sizeCanvas(): boolean {
  const canvas = canvasRef.value
  const wrap = wrapRef.value
  if (!canvas || !wrap) return false
  const rect = wrap.getBoundingClientRect()
  cssW = Math.max(1, Math.round(rect.width))
  cssH = Math.max(1, Math.round(rect.height))
  dpr = Math.min(window.devicePixelRatio || 1, config.dprCap)
  canvas.width = Math.round(cssW * dpr)
  canvas.height = Math.round(cssH * dpr)
  ctx = canvas.getContext('2d', { alpha: true })
  if (!ctx) return false
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
  linkDist = linkDistance(cssW)
  applyCanvasOpacity()
  return true
}

function spawnFlash(i: number, j: number, now: number): void {
  const slot = flashes.find((flash) => !flash.alive)
  if (!slot) return
  slot.i = i
  slot.j = j
  slot.born = now
  slot.duration = config.flashDurMin + Math.random() * (config.flashDurMax - config.flashDurMin)
  slot.peak = config.flashPeakMin + Math.random() * (config.flashPeakMax - config.flashPeakMin)
  slot.alive = true
}

function tick(now: number, frame: number, deltaMs: number): void {
  if (!ctx) return

  const target = mouseInside ? 1 : 0
  mouseInfluence +=
    (target - mouseInfluence) * (1 - Math.exp(-deltaMs / config.mouseFadeMs))

  const w = cssW
  const h = cssH
  const radius = config.cursorRadius
  const radius2 = radius * radius
  const maxSpeed = config.speedMax * config.maxSpeedFactor
  const influence = mouseInfluence

  for (const p of particles) {
    let vx = p.vx
    let vy = p.vy
    if (influence > 0.01) {
      const dx = p.x - mouseX
      const dy = p.y - mouseY
      const d2 = dx * dx + dy * dy
      if (d2 < radius2 && d2 > 0.0001) {
        const dist = Math.sqrt(d2)
        const nx = dx / dist
        const ny = dy / dist
        const falloff = (1 - dist / radius) * influence
        const force = config.pushForce * falloff * frame
        vx += nx * force
        vy += ny * force
        const spin = config.swirl * falloff * frame
        vx += -ny * spin
        vy += nx * spin
      }
    }
    const speed = Math.hypot(vx, vy)
    if (speed > 0.0001) {
      if (speed > maxSpeed) {
        const k = maxSpeed / speed
        vx *= k
        vy *= k
      } else {
        const blend = Math.min(1, config.relax * (1 - influence) * frame)
        const next = speed + (p.cruise - speed) * blend
        const k = next / speed
        vx *= k
        vy *= k
      }
    }
    p.vx = vx
    p.vy = vy
    p.x += vx * frame
    p.y += vy * frame
    if (p.x < 0) {
      p.x = 0
      p.vx = Math.abs(p.vx)
    } else if (p.x > w) {
      p.x = w
      p.vx = -Math.abs(p.vx)
    }
    if (p.y < 0) {
      p.y = 0
      p.vy = Math.abs(p.vy)
    } else if (p.y > h) {
      p.y = h
      p.vy = -Math.abs(p.vy)
    }
  }

  flashEnv.clear()
  flashPeak.clear()
  flashPoints.clear()
  flashPointPeak.clear()
  for (const flash of flashes) {
    if (!flash.alive) continue
    const life = (now - flash.born) / flash.duration
    if (life >= 1) {
      flash.alive = false
      continue
    }
    const env = Math.sin(life * Math.PI)
    const key = pairKey(flash.i, flash.j)
    flashEnv.set(key, env)
    flashPeak.set(key, flash.peak)
    const ii = flash.i
    const jj = flash.j
    const iEnv = flashPoints.get(ii)
    if (iEnv === undefined || env > iEnv) {
      flashPoints.set(ii, env)
      flashPointPeak.set(ii, flash.peak)
    }
    const jEnv = flashPoints.get(jj)
    if (jEnv === undefined || env > jEnv) {
      flashPoints.set(jj, env)
      flashPointPeak.set(jj, flash.peak)
    }
  }

  ctx.clearRect(0, 0, w, h)

  const link2 = linkDist * linkDist
  ctx.lineCap = 'round'
  for (let i = 0; i < particles.length; i++) {
    const a = particles[i]
    for (let j = i + 1; j < particles.length; j++) {
      const b = particles[j]
      const dx = a.x - b.x
      const dy = a.y - b.y
      const d2 = dx * dx + dy * dy
      const key = pairKey(i, j)
      if (d2 >= link2 || d2 === 0) {
        prevLinks.delete(key)
        continue
      }
      if (!prevLinks.has(key)) {
        prevLinks.add(key)
        if (!settleLinks) spawnFlash(i, j, now)
      }
      const dist = Math.sqrt(d2)
      const t = 1 - dist / linkDist
      const env = flashEnv.get(key)
      let alpha = config.linkAlphaBase + t * config.linkAlphaFade
      let width = config.linkWidthBase + t * config.linkWidthFade
      ctx.beginPath()
      ctx.moveTo(a.x, a.y)
      ctx.lineTo(b.x, b.y)
      if (env) {
        const peak = flashPeak.get(key) ?? 0.62
        ctx.lineWidth = width * (2.5 + env * 2)
        ctx.strokeStyle = `rgba(${config.brand}, ${env * peak * config.linkGlowAlpha})`
        ctx.stroke()
        alpha = alpha + (peak - alpha) * env
        width = width * (1 + env * 1.6)
      }
      ctx.lineWidth = width
      ctx.strokeStyle = `rgba(${config.brand}, ${alpha})`
      ctx.stroke()
    }
  }
  settleLinks = false

  for (let i = 0; i < particles.length; i++) {
    const p = particles[i]
    const env = flashPoints.get(i)
    if (env) {
      const peak = flashPointPeak.get(i) ?? 0.62
      const glowR = p.r * (3.2 + env * 2.5)
      ctx.beginPath()
      ctx.arc(p.x, p.y, glowR, 0, Math.PI * 2)
      ctx.fillStyle = `rgba(${config.brand}, ${env * peak * config.flashPointHalo})`
      ctx.fill()
    }
    ctx.beginPath()
    ctx.arc(p.x, p.y, p.r, 0, Math.PI * 2)
    const color = p.brand ? config.brand : config.neutral
    const base = p.brand ? p.alpha : config.neutralAlpha
    const alpha = env ? Math.min(1, base + env * config.flashPointBoost) : base
    ctx.fillStyle = `rgba(${color}, ${alpha})`
    ctx.fill()
  }
}

function loop(now: number): void {
  if (!running) return
  raf = requestAnimationFrame(loop)
  if (document.hidden) return
  const delta = lastTs ? Math.min(48, now - lastTs) : 16.67
  lastTs = now
  tick(now, delta / 16.67, delta)
}

function startLoop(): void {
  if (running) return
  if (!sizeCanvas()) return
  seed(cssW, cssH)
  running = true
  lastTs = 0
  mouseInfluence = 0
  mouseInside = false
  raf = requestAnimationFrame(loop)
}

function stopLoop(): void {
  running = false
  if (raf) {
    cancelAnimationFrame(raf)
    raf = 0
  }
  lastTs = 0
  ctx = null
}

function onPointerMove(event: PointerEvent): void {
  const wrap = wrapRef.value
  if (!wrap) return
  const rect = wrap.getBoundingClientRect()
  mouseX = event.clientX - rect.left
  mouseY = event.clientY - rect.top
  mouseInside =
    mouseX >= 0 && mouseY >= 0 && mouseX <= rect.width && mouseY <= rect.height
}

function onPointerOut(event: PointerEvent): void {
  if (event.relatedTarget === null) mouseInside = false
}

function onVisibility(): void {
  if (document.hidden) {
    if (raf) {
      cancelAnimationFrame(raf)
      raf = 0
    }
    lastTs = 0
    return
  }
  if (running && !raf) {
    lastTs = 0
    raf = requestAnimationFrame(loop)
  }
}

function attachRuntime(): void {
  if (runtimeListening) return
  window.addEventListener('pointermove', onPointerMove, { passive: true })
  window.addEventListener('pointerout', onPointerOut)
  document.addEventListener('visibilitychange', onVisibility)
  runtimeListening = true
}

function detachRuntime(): void {
  if (!runtimeListening) return
  window.removeEventListener('pointermove', onPointerMove)
  window.removeEventListener('pointerout', onPointerOut)
  document.removeEventListener('visibilitychange', onVisibility)
  runtimeListening = false
  mouseInside = false
  mouseInfluence = 0
}

function observeBox(): void {
  boxObserver?.disconnect()
  boxObserver = null
  if (!wrapRef.value) return
  boxObserver = new ResizeObserver(() => scheduleEvaluate())
  boxObserver.observe(wrapRef.value)
}

async function enable(): Promise<void> {
  if (!enabled.value) enabled.value = true
  attachRuntime()
  await nextTick()
  observeBox()
  if (!running) startLoop()
}

function disable(): void {
  stopLoop()
  detachRuntime()
  boxObserver?.disconnect()
  boxObserver = null
  enabled.value = false
}

function evaluate(): void {
  if (shouldRun()) void enable()
  else disable()
}

function scheduleEvaluate(): void {
  window.clearTimeout(resizeTimer)
  resizeTimer = window.setTimeout(() => {
    if (!shouldRun()) {
      disable()
      return
    }
    if (!enabled.value || !running) {
      void enable()
      return
    }
    if (!sizeCanvas()) return
    seed(cssW, cssH)
  }, config.resizeDebounceMs)
}

onMounted(() => {
  exposeApi()
  reducedMq = window.matchMedia('(prefers-reduced-motion: reduce)')
  reducedMq.addEventListener('change', evaluate)
  window.addEventListener('resize', scheduleEvaluate)
  themeObserver = new MutationObserver(evaluate)
  themeObserver.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ['class'],
  })
  evaluate()
})

onBeforeUnmount(() => {
  window.clearTimeout(resizeTimer)
  reducedMq?.removeEventListener('change', evaluate)
  window.removeEventListener('resize', scheduleEvaluate)
  themeObserver?.disconnect()
  themeObserver = null
  disable()
})
</script>

<template>
  <div
    v-if="enabled"
    ref="wrapRef"
    class="den-plexus"
    aria-hidden="true"
  >
    <canvas ref="canvasRef" />
  </div>
</template>
