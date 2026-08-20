<script setup lang="ts">
// ============================================================
// FilterGallery - Bottom drawer gallery filtered by photo status
// ============================================================

import { computed, ref, onMounted, onBeforeUnmount } from 'vue';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useSessionStore } from '@/stores/sessionStore';
import { useViewStore } from '@/stores/viewStore';
import { PhotoStatus } from '@/types';
import { convertFileSrc } from '@tauri-apps/api/core';
import { Star, Trash2, SkipForward, Circle, X, CheckSquare, Check } from 'lucide-vue-next';

const session = useSessionStore();
const view = useViewStore();

const categories = [
  { key: PhotoStatus.Starred, label: '已标记', icon: Star, color: 'text-sift-star', bgColor: 'bg-sift-star' },
  { key: PhotoStatus.Deleted, label: '已删除', icon: Trash2, color: 'text-sift-delete', bgColor: 'bg-sift-delete' },
  { key: PhotoStatus.Skipped, label: '已跳过', icon: SkipForward, color: 'text-sift-muted', bgColor: 'bg-sift-skip' },
  { key: PhotoStatus.Unprocessed, label: '未处理', icon: Circle, color: 'text-sift-accent', bgColor: 'bg-sift-accent' },
];

// 缩略图尺寸档位（最小 110px，支持 1.5x / 2x / 3x）
const THUMB_SIZES = [
  { key: '小', width: 110 },
  { key: '中', width: 165 },
  { key: '大', width: 220 },
  { key: '硕', width: 330 },
];
const thumbSizeIndex = ref(0);
const thumbMinWidth = computed(() => THUMB_SIZES[thumbSizeIndex.value].width);

function cycleThumbSize() {
  thumbSizeIndex.value = (thumbSizeIndex.value + 1) % THUMB_SIZES.length;
}

const isOpen = computed(() => view.filterCategory !== null);

const activeCategory = computed(() => view.filterCategory);

function getCategoryCount(key: PhotoStatus): number {
  return session.pairs.filter(p => p.status === key).length;
}

const filteredPairs = computed(() => {
  if (!view.filterCategory) return [];
  return session.pairs
    .map((pair, index) => ({ pair, originalIndex: index }))
    .filter(({ pair }) => pair.status === view.filterCategory);
});

function getThumbnailSrc(pair: { thumbnailPath?: string; jpgPath: string }): string {
  if (pair.thumbnailPath) {
    return convertFileSrc(pair.thumbnailPath);
  }
  return convertFileSrc(pair.jpgPath);
}

function getFileName(pair: { jpgPath: string; rawPath?: string | null; source?: string }): string {
  // For RAW-only photos, show the RAW filename
  if (pair.source === 'rawPreview' && pair.rawPath) {
    return pair.rawPath.split('/').pop()?.split('\\').pop() || '';
  }
  return pair.jpgPath.split('/').pop()?.split('\\').pop() || '';
}

function handleSelect(originalIndex: number) {
  session.goTo(originalIndex);
  view.closeFilterGallery();
}

function handleBackdropClick() {
  view.closeFilterGallery();
}

function switchCategory(key: PhotoStatus) {
  view.openFilterGallery(key);
}

// ---- Multi-select state ----
const isSelectMode = ref(false);
const selectedIndices = ref<Set<number>>(new Set());

function isSelected(index: number): boolean {
  return selectedIndices.value.has(index);
}

function toggleSelectMode() {
  isSelectMode.value = !isSelectMode.value;
  selectedIndices.value = new Set();
}

function handleThumbClick(originalIndex: number) {
  if (isSelectMode.value) {
    toggleSelect(originalIndex);
  } else {
    handleSelect(originalIndex);
  }
}

function toggleSelect(originalIndex: number) {
  const next = new Set(selectedIndices.value);
  if (next.has(originalIndex)) {
    next.delete(originalIndex);
  } else {
    next.add(originalIndex);
  }
  selectedIndices.value = next;
}

function selectAll() {
  selectedIndices.value = new Set(
    filteredPairs.value.map(({ originalIndex }) => originalIndex)
  );
}

function clearSelection() {
  selectedIndices.value = new Set();
}

function batchStar() {
  session.markStarBatch([...selectedIndices.value]);
  exitSelectMode();
}

function batchDelete() {
  session.markDeleteBatch([...selectedIndices.value]);
  exitSelectMode();
}

function exitSelectMode() {
  isSelectMode.value = false;
  selectedIndices.value = new Set();
}

// ---- 全屏模式 ----
const isFullscreen = ref(false);

function toggleFullscreen() {
  isFullscreen.value = !isFullscreen.value;
  syncWindowDecorations();
}

function exitFullscreen() {
  isFullscreen.value = false;
  syncWindowDecorations();
}

/** 全屏时隐藏窗口标题栏（LOGO + SIFT 字样 + 边框），退出时恢复 */
async function syncWindowDecorations() {
  try {
    const win = getCurrentWindow();
    await win.setDecorations(!isFullscreen.value);
  } catch (e) {
    console.warn('setDecorations failed:', e);
  }
}

function batchSkip() {
  session.markSkipBatch([...selectedIndices.value]);
  exitSelectMode();
}

function invertSelection() {
  const all = new Set(filteredPairs.value.map(({ originalIndex }) => originalIndex));
  const next = new Set<number>();
  for (const idx of all) {
    if (!selectedIndices.value.has(idx)) {
      next.add(idx);
    }
  }
  selectedIndices.value = next;
}

// ---- 鼠标框选（拖拽圈选）----
const gridRef = ref<HTMLElement | null>(null);
const isDragging = ref(false);
const dragStart = ref({ x: 0, y: 0 });
const dragCurrent = ref({ x: 0, y: 0 });

const selectionBoxStyle = computed(() => {
  const left = Math.min(dragStart.value.x, dragCurrent.value.x);
  const top = Math.min(dragStart.value.y, dragCurrent.value.y);
  const width = Math.abs(dragCurrent.value.x - dragStart.value.x);
  const height = Math.abs(dragCurrent.value.y - dragStart.value.y);
  return { left: `${left}px`, top: `${top}px`, width: `${width}px`, height: `${height}px` };
});

function onGridMouseDown(e: MouseEvent) {
  if (!isSelectMode.value || e.button !== 0) return;
  // 点在缩略图上时不启动框选（保留单击切换选中）
  if ((e.target as HTMLElement).closest('[data-thumb-index]')) return;
  dragStart.value = { x: e.clientX, y: e.clientY };
  dragCurrent.value = { x: e.clientX, y: e.clientY };
  isDragging.value = true;
  e.preventDefault();
}

function onWindowMouseMove(e: MouseEvent) {
  if (!isDragging.value) return;
  dragCurrent.value = { x: e.clientX, y: e.clientY };
}

function onWindowMouseUp() {
  if (!isDragging.value) return;
  isDragging.value = false;

  const box = {
    left: Math.min(dragStart.value.x, dragCurrent.value.x),
    right: Math.max(dragStart.value.x, dragCurrent.value.x),
    top: Math.min(dragStart.value.y, dragCurrent.value.y),
    bottom: Math.max(dragStart.value.y, dragCurrent.value.y),
  };
  // 太小视为点击，忽略
  if (box.right - box.left < 5 && box.bottom - box.top < 5) return;

  const gridEl = gridRef.value;
  if (!gridEl) return;

  const next = new Set(selectedIndices.value);
  gridEl.querySelectorAll('[data-thumb-index]').forEach((el) => {
    const r = el.getBoundingClientRect();
    if (r.left < box.right && r.right > box.left && r.top < box.bottom && r.bottom > box.top) {
      const idx = Number((el as HTMLElement).dataset.thumbIndex);
      if (!Number.isNaN(idx)) next.add(idx);
    }
  });
  selectedIndices.value = next;
}

function onWindowKeydown(e: KeyboardEvent) {
  // ALT+Enter 切换全屏
  if (e.altKey && e.key === 'Enter') {
    e.preventDefault();
    toggleFullscreen();
    return;
  }
  // Esc 退出全屏
  if (e.key === 'Escape' && isFullscreen.value) {
    e.preventDefault();
    exitFullscreen();
  }
}

onMounted(() => {
  window.addEventListener('mousemove', onWindowMouseMove);
  window.addEventListener('mouseup', onWindowMouseUp);
  window.addEventListener('keydown', onWindowKeydown);
});

onBeforeUnmount(() => {
  window.removeEventListener('mousemove', onWindowMouseMove);
  window.removeEventListener('mouseup', onWindowMouseUp);
  window.removeEventListener('keydown', onWindowKeydown);
});
</script>

<template>
  <Teleport to="body">
    <!-- Backdrop (semi-transparent, no blur) -->
    <Transition name="backdrop">
      <div
        v-if="isOpen"
        class="fixed inset-0 z-[100] bg-black/50"
        @click="handleBackdropClick"
      />
    </Transition>

    <!-- Bottom Drawer -->
    <Transition name="drawer">
      <div
        v-if="isOpen"
        class="fixed top-0 left-0 right-0 z-[101]
               bg-[#161616]/95 backdrop-blur-2xl
               border-t border-white/[0.06]
               overflow-hidden"
        :class="isFullscreen ? 'bottom-0 shadow-none' : 'bottom-10 shadow-[0_-8px_40px_rgba(0,0,0,0.5)]'"
        @click.stop
      >
        <!-- Top bar (absolute overlay; hover to reveal in fullscreen) -->
        <div class="group/top absolute top-0 left-0 right-0 z-10">
          <div v-if="isFullscreen" class="absolute top-0 left-0 right-0 h-10" />
          <div
            class="relative bg-[#161616]"
            :class="isFullscreen ? 'hidden group-hover/top:block' : ''"
          >
            <!-- Drag Handle -->
            <div class="flex justify-center pt-1 pb-0.5">
              <div class="w-8 h-1 rounded-full bg-white/15" />
            </div>

        <!-- Tab Bar -->
        <div class="flex items-center px-4 pb-1.5 gap-1">
          <button
            v-for="cat in categories"
            :key="cat.key"
            class="flex items-center gap-1.5 px-2.5 py-1 rounded-lg text-xs font-medium
                   transition-all duration-200"
            :class="[
              activeCategory === cat.key
                ? `bg-white/10 ${cat.color}`
                : 'text-sift-muted hover:text-sift-text hover:bg-white/5',
            ]"
            @click="switchCategory(cat.key)"
          >
            <component :is="cat.icon" :size="13" />
            <span>{{ cat.label }}</span>
            <span
              class="ml-0.5 text-[10px] px-1.5 py-0.5 rounded-full min-w-[20px] text-center
                     transition-colors duration-200"
              :class="[
                activeCategory === cat.key
                  ? 'bg-white/10 text-white/80'
                  : 'bg-white/5 text-sift-muted/60',
              ]"
            >
              {{ getCategoryCount(cat.key) }}
            </span>
          </button>

          <!-- Thumbnail size toggle -->
          <button
            class="ml-auto px-2 py-1 rounded-lg text-xs text-sift-muted hover:text-sift-text hover:bg-white/5 transition-colors"
            :title="`缩略图尺寸：${THUMB_SIZES[thumbSizeIndex].key}`"
            @click="cycleThumbSize"
          >
            {{ THUMB_SIZES[thumbSizeIndex].key }}
          </button>

          <!-- Fullscreen toggle -->
          <button
            class="px-2 py-1 rounded-lg text-xs transition-colors"
            :class="isFullscreen ? 'bg-sift-accent/20 text-sift-accent' : 'text-sift-muted hover:text-sift-text hover:bg-white/5'"
            :title="isFullscreen ? '退出全屏 (Esc)' : '全屏 (Alt+Enter)'"
            @click="toggleFullscreen"
          >
            全
          </button>

          <!-- Multi-select toggle -->
          <button
            class="p-1.5 rounded-lg transition-colors"
            :class="isSelectMode ? 'bg-sift-accent/20 text-sift-accent' : 'text-sift-muted hover:text-sift-text hover:bg-white/5'"
            :title="isSelectMode ? '退出多选' : '多选'"
            @click="toggleSelectMode"
          >
            <CheckSquare :size="14" />
          </button>

          <!-- Close button (pushed right) -->
          <button
            class="p-1.5 rounded-lg text-sift-muted hover:text-sift-text
                   hover:bg-white/5 transition-colors"
            @click="view.closeFilterGallery()"
          >
            <X :size="14" />
          </button>
        </div>

            <!-- Divider -->
            <div class="h-px bg-white/[0.06] mx-4" />
          </div>
        </div>

        <!-- Grid Content -->
        <div
          ref="gridRef"
          class="absolute inset-0 overflow-y-auto p-4"
          :class="isFullscreen ? 'pt-0' : 'pt-16'"
          @mousedown="onGridMouseDown"
        >
          <div
            v-if="filteredPairs.length === 0"
            class="flex flex-col items-center justify-center h-32 gap-2"
          >
            <component
              :is="categories.find(c => c.key === activeCategory)?.icon"
              :size="24"
              class="text-sift-muted/30"
            />
            <span class="text-sift-muted/50 text-xs">暂无图片</span>
          </div>
          <div
            v-else
            class="grid gap-2"
            :style="{ gridTemplateColumns: `repeat(auto-fill, minmax(${thumbMinWidth}px, 1fr))` }"
          >
            <div
              v-for="{ pair, originalIndex } in filteredPairs"
              :key="pair.id"
              :data-thumb-index="originalIndex"
              class="group relative aspect-[4/3] rounded-lg overflow-hidden cursor-pointer
                     bg-sift-card/50 transition-all duration-200
                     hover:ring-1.5 hover:ring-white/20 hover:scale-[1.02]"
              :class="[
                isSelected(originalIndex)
                  ? 'ring-2 ring-sift-accent'
                  : '',
                !isSelectMode && originalIndex === session.currentIndex
                  ? 'ring-1.5 ring-sift-accent'
                  : '',
              ]"
              @click="handleThumbClick(originalIndex)"
            >
              <img
                :src="getThumbnailSrc(pair)"
                :alt="getFileName(pair)"
                class="w-full h-full object-cover"
                loading="lazy"
              />
              <!-- Selected checkmark -->
              <div
                v-if="isSelectMode && isSelected(originalIndex)"
                class="absolute top-1.5 right-1.5 w-5 h-5 rounded-full bg-sift-accent
                       flex items-center justify-center shadow"
              >
                <Check :size="12" class="text-white" />
              </div>
              <!-- Filename overlay -->
              <div
                class="absolute bottom-0 inset-x-0 bg-gradient-to-t from-black/70 to-transparent
                       px-1.5 py-1 opacity-0 group-hover:opacity-100 transition-opacity"
              >
                <span class="text-[10px] text-white/80 truncate block">
                  {{ getFileName(pair) }}
                </span>
              </div>
            </div>
          </div>
        </div>

        <!-- Batch action bar (multi-select mode) -->
        <div v-if="isSelectMode" class="group/bottom absolute bottom-0 left-0 right-0 z-10">
          <div v-if="isFullscreen" class="absolute bottom-0 left-0 right-0 h-10" />
          <div
            :class="isFullscreen ? 'hidden group-hover/bottom:flex' : ''"
            class="relative flex items-center gap-2 px-4 py-1.5 border-t border-white/[0.06] bg-[#1a1a1a]"
          >
          <span class="text-xs text-sift-muted whitespace-nowrap">
            已选 <span class="text-white font-semibold">{{ selectedIndices.size }}</span> 张
          </span>
          <button
            class="px-2 py-1 text-xs text-sift-muted hover:text-sift-text hover:bg-white/5 rounded transition-colors"
            @click="selectAll"
          >
            全选
          </button>
          <button
            class="px-2 py-1 text-xs text-sift-muted hover:text-sift-text hover:bg-white/5 rounded transition-colors"
            @click="clearSelection"
          >
            清空
          </button>
          <button
            class="px-2 py-1 text-xs text-sift-muted hover:text-sift-text hover:bg-white/5 rounded transition-colors"
            @click="invertSelection"
          >
            反选
          </button>
          <button
            class="ml-auto flex items-center gap-1.5 px-2.5 py-1 rounded-lg text-xs font-medium whitespace-nowrap
                   bg-sift-star/15 text-sift-star hover:bg-sift-star/25 transition-colors
                   disabled:opacity-40 disabled:cursor-not-allowed"
            :disabled="selectedIndices.size === 0"
            @click="batchStar"
          >
            <Star :size="13" />
            标记
          </button>
          <button
            class="flex items-center gap-1.5 px-2.5 py-1 rounded-lg text-xs font-medium whitespace-nowrap
                   bg-sift-delete/15 text-sift-delete hover:bg-sift-delete/25 transition-colors
                   disabled:opacity-40 disabled:cursor-not-allowed"
            :disabled="selectedIndices.size === 0"
            @click="batchDelete"
          >
            <Trash2 :size="13" />
            删除
          </button>
          <button
            class="flex items-center gap-1.5 px-2.5 py-1 rounded-lg text-xs font-medium whitespace-nowrap
                   bg-sift-skip/15 text-sift-muted hover:bg-sift-skip/25 transition-colors
                   disabled:opacity-40 disabled:cursor-not-allowed"
            :disabled="selectedIndices.size === 0"
            @click="batchSkip"
          >
            <SkipForward :size="13" />
            跳过
          </button>
          </div>
        </div>
      </div>
    </Transition>

    <!-- Selection box (drag-select rectangle) -->
    <div
      v-if="isDragging"
      class="fixed z-[102] pointer-events-none border border-sift-accent bg-sift-accent/10"
      :style="selectionBoxStyle"
    />

    <!-- Context Menu -->
    <ContextMenu />
  </Teleport>
</template>
