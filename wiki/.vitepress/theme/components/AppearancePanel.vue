<script setup lang="ts">
import { computed } from 'vue'
import { useData } from 'vitepress'
import { appearance } from './appearance-store'

const { lang } = useData()

const isEn = computed(() => lang.value.startsWith('en'))

const ui = computed(() =>
  isEn.value
    ? {
        title: 'Appearance',
        text: 'Text',
        textOptions: [
          { value: 'small', label: 'Small' },
          { value: 'standard', label: 'Standard' },
          { value: 'large', label: 'Large' },
        ],
        width: 'Width',
        widthOptions: [
          { value: 'small', label: 'Small' },
          { value: 'standard', label: 'Standard' },
          { value: 'wide', label: 'Wide' },
        ],
        color: 'Color',
        colorOptions: [
          { value: 'automatic', label: 'Automatic' },
          { value: 'light', label: 'Light' },
          { value: 'dark', label: 'Dark' },
        ],
      }
    : {
        title: 'Внешний вид',
        text: 'Текст',
        textOptions: [
          { value: 'small', label: 'Мелкий' },
          { value: 'standard', label: 'Стандартный' },
          { value: 'large', label: 'Крупный' },
        ],
        width: 'Ширина',
        widthOptions: [
          { value: 'small', label: 'Узкий' },
          { value: 'standard', label: 'Стандартный' },
          { value: 'wide', label: 'Широкий' },
        ],
        color: 'Цвет',
        colorOptions: [
          { value: 'automatic', label: 'Автоматически' },
          { value: 'light', label: 'Светлая' },
          { value: 'dark', label: 'Тёмная' },
        ],
      },
)
</script>

<template>
  <div class="appearance-panel" role="region" :aria-label="ui.title">
    <div class="appearance-head">
      <span class="appearance-title">{{ ui.title }}</span>
    </div>

    <section class="appearance-section">
      <h4>{{ ui.text }}</h4>
      <div class="appearance-options" role="radiogroup" :aria-label="ui.text">
        <label
          v-for="option in ui.textOptions"
          :key="option.value"
          class="den-radio"
          :class="{ checked: appearance.text === option.value }"
        >
          <input
            v-model="appearance.text"
            class="den-radio-input"
            type="radio"
            name="den-text"
            :value="option.value"
          />
          <span class="den-radio-mark" aria-hidden="true"></span>
          <span class="den-radio-label">{{ option.label }}</span>
        </label>
      </div>
    </section>

    <section class="appearance-section">
      <h4>{{ ui.width }}</h4>
      <div class="appearance-options" role="radiogroup" :aria-label="ui.width">
        <label
          v-for="option in ui.widthOptions"
          :key="option.value"
          class="den-radio"
          :class="{ checked: appearance.width === option.value }"
        >
          <input
            v-model="appearance.width"
            class="den-radio-input"
            type="radio"
            name="den-width"
            :value="option.value"
          />
          <span class="den-radio-mark" aria-hidden="true"></span>
          <span class="den-radio-label">{{ option.label }}</span>
        </label>
      </div>
    </section>

    <section class="appearance-section">
      <h4>{{ ui.color }}</h4>
      <div class="appearance-options" role="radiogroup" :aria-label="ui.color">
        <label
          v-for="option in ui.colorOptions"
          :key="option.value"
          class="den-radio"
          :class="{ checked: appearance.color === option.value }"
        >
          <input
            v-model="appearance.color"
            class="den-radio-input"
            type="radio"
            name="den-color"
            :value="option.value"
          />
          <span class="den-radio-mark" aria-hidden="true"></span>
          <span class="den-radio-label">{{ option.label }}</span>
        </label>
      </div>
    </section>
  </div>
</template>