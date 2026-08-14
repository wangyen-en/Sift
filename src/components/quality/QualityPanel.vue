<script setup lang="ts">
// ============================================================
// QualityPanel - Left slide-in drawer with technical quality metrics
// Analyzes sharpness / exposure / noise on demand, cached per photo
// ============================================================

import { ref, watch, computed } from 'vue'
import { X, Gauge } from 'lucide-vue-next'
import { useSessionStore } from '@/stores/sessionStore'
import { useViewStore } from '@/stores/viewStore'
import { analyzeQuality } from '@/services/tauriCommands'
import type { QualityData } from '@/types'

const session = useSessionStore()
const view = useViewStore()

const quality = ref<QualityData | null>(null)
const isLoading = ref(false)
const error = ref('')

// Cache keyed by photo id (module scope survives panel open/close)
const cache = new Map<string, QualityData>()

watch(
  [() => session.currentPair, () => view.showQualityPanel],
  async ([pair, show]) => {
    if (!show || !pair) {
      return
    }
    const cached = cache.get(pair.id)
    if (cached) {
      quality.value = cached
      error.value = ''
      return
    }
    isLoading.value = true
    error.value = ''
    quality.value = null
    try {
      const data = await analyzeQuality(pair.jpgPath)
      cache.set(pair.id, data)
      quality.value = data
    } catch {
      error.value = '分析失败'
    } finally {
      isLoading.value = false
    }
  },
  { immediate: true }
)

const metrics = computed(() => {
  const q = quality.value
  if (!q) return []
  return [
    { label: '锐度', value: q.sharpness, hint: '清晰度 · 低分可能模糊' },
    { label: '曝光', value: q.exposure, hint: '明暗平衡' },
    { label: '噪点', value: q.noise, hint: '洁净度 · 低分噪点多' },
  ]
})

const warnings = computed(() => {
  const q = quality.value
  if (!q) return []
  const list: string[] = []
  if (q.sharpness < 40) list.push('可能模糊')
  if (q.overexposedPct > 3) list.push('存在过曝')
  if (q.underexposedPct > 3) list.push('存在欠曝')
  if (q.noise < 40) list.push('噪点偏高')
  return list
})

function scoreColor(v: number): string {
  if (v >= 70) return 'text-sift-success'
  if (v >= 40) return 'text-sift-star'
  return 'text-sift-delete'
}

function barColor(v: number): string {
  if (v >= 70) return 'bg-sift-success'
  if (v >= 40) return 'bg-sift-star'
  return 'bg-sift-delete'
}
</script>

<template>
  <Transition name="quality">
    <div
      v-if="view.showQualityPanel"
      class="fixed top-12 left-0 bottom-10 w-72 z-40
             bg-sift-surface/95 backdrop-blur-xl
             border-r border-sift-border
             overflow-y-auto"
    >
      <!-- Header -->
      <div class="flex items-center justify-between px-4 py-3 border-b border-sift-border">
        <div class="flex items-center gap-2">
          <Gauge :size="14" class="text-sift-accent" />
          <span class="text-sm font-semibold text-white">质量评估</span>
        </div>
        <button
          class="p-1 rounded hover:bg-white/10 transition-colors btn-spring"
          @click="view.toggleQualityPanel()"
        >
          <X :size="14" class="text-sift-muted" />
        </button>
      </div>

      <!-- Loading -->
      <div v-if="isLoading" class="p-4 space-y-3">
        <div class="skeleton-pulse h-16 rounded-lg" />
        <div class="skeleton-pulse h-4 rounded w-3/4" />
        <div class="skeleton-pulse h-4 rounded w-2/3" />
        <div class="skeleton-pulse h-4 rounded w-1/2" />
      </div>

      <!-- Error -->
      <div v-else-if="error" class="p-4 text-center">
        <p class="text-sift-muted text-sm">{{ error }}</p>
      </div>

      <!-- Content -->
      <div v-else-if="quality" class="p-4 space-y-5">
        <!-- Overall score -->
        <div class="flex items-center justify-between rounded-lg bg-sift-card/60 px-4 py-3">
          <div>
            <p class="text-xs text-sift-muted">综合质量分</p>
            <p class="text-[11px] text-sift-muted mt-0.5">锐度 40% · 曝光 30% · 噪点 30%</p>
          </div>
          <span class="text-3xl font-bold tabular-nums" :class="scoreColor(quality.overall)">
            {{ quality.overall.toFixed(0) }}
          </span>
        </div>

        <!-- Metric bars -->
        <div class="space-y-4">
          <div v-for="m in metrics" :key="m.label">
            <div class="flex items-center justify-between mb-1.5">
              <span class="text-xs text-sift-muted">{{ m.label }}</span>
              <span class="text-sm font-semibold tabular-nums" :class="scoreColor(m.value)">
                {{ m.value.toFixed(0) }}
              </span>
            </div>
            <div class="h-1.5 rounded-full bg-white/10 overflow-hidden">
              <div
                class="h-full rounded-full transition-all duration-500"
                :class="barColor(m.value)"
                :style="{ width: `${m.value}%` }"
              />
            </div>
            <p class="text-[11px] text-sift-muted mt-1">{{ m.hint }}</p>
          </div>
        </div>

        <!-- Warnings -->
        <div v-if="warnings.length" class="space-y-2">
          <div
            v-for="w in warnings"
            :key="w"
            class="px-3 py-1.5 rounded-md bg-sift-star/10 text-sift-star text-xs flex items-center gap-2"
          >
            <span class="w-1.5 h-1.5 rounded-full bg-sift-star shrink-0" />
            {{ w }}
          </div>
        </div>

        <div class="h-px bg-sift-border" />

        <!-- Raw stats -->
        <div class="space-y-2">
          <p class="text-xs text-sift-muted uppercase tracking-wider">原始指标</p>
          <div class="grid grid-cols-3 gap-3">
            <div>
              <p class="text-[11px] text-sift-muted">亮度</p>
              <p class="text-sm text-white font-semibold">{{ quality.brightness.toFixed(0) }}</p>
            </div>
            <div>
              <p class="text-[11px] text-sift-muted">过曝</p>
              <p class="text-sm text-white font-semibold">{{ quality.overexposedPct.toFixed(1) }}%</p>
            </div>
            <div>
              <p class="text-[11px] text-sift-muted">欠曝</p>
              <p class="text-sm text-white font-semibold">{{ quality.underexposedPct.toFixed(1) }}%</p>
            </div>
          </div>
          <p class="text-[11px] text-sift-muted leading-relaxed pt-1">
            基于图像统计的启发式评估，仅供参考，不作为唯一筛选依据。
          </p>
        </div>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
.quality-enter-active {
  transition: transform 400ms cubic-bezier(0.34, 1.56, 0.64, 1), opacity 300ms ease;
}
.quality-leave-active {
  transition: transform 250ms ease-in, opacity 200ms ease-in;
}
.quality-enter-from {
  transform: translateX(-100%);
  opacity: 0;
}
.quality-leave-to {
  transform: translateX(-100%);
  opacity: 0;
}
</style>
