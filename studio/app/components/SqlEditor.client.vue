<script setup lang="ts">
import { EditorView, basicSetup } from 'codemirror'
import { sql, StandardSQL } from '@codemirror/lang-sql'
import { oneDark } from '@codemirror/theme-one-dark'
import { EditorState } from '@codemirror/state'
import { keymap } from '@codemirror/view'

const props = defineProps<{ modelValue: string }>()
const emit = defineEmits<{
  'update:modelValue': [value: string]
  'run': []
}>()

const editorEl = ref<HTMLElement>()
let view: EditorView | null = null

const customTheme = EditorView.theme({
  '&': {
    height: '100%',
    background: 'transparent',
  },
  '.cm-scroller': {
    fontFamily: "'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace",
    fontSize: '13px',
    lineHeight: '1.6',
  },
  '.cm-gutters': {
    background: '#0f172a',
    border: 'none',
    borderRight: '1px solid #1e293b',
    color: '#475569',
  },
  '.cm-lineNumbers .cm-gutterElement': {
    padding: '0 10px 0 6px',
    minWidth: '32px',
  },
  '.cm-activeLine': {
    background: 'rgba(139, 92, 246, 0.05)',
  },
  '.cm-activeLineGutter': {
    background: 'rgba(139, 92, 246, 0.1)',
    color: '#a78bfa',
  },
  '.cm-selectionBackground, ::selection': {
    background: 'rgba(139, 92, 246, 0.25) !important',
  },
  '&.cm-focused .cm-cursor': {
    borderLeftColor: '#a78bfa',
  },
})

onMounted(() => {
  if (!editorEl.value) return
  view = new EditorView({
    state: EditorState.create({
      doc: props.modelValue,
      extensions: [
        basicSetup,
        sql({ dialect: StandardSQL }),
        oneDark,
        customTheme,
        EditorView.updateListener.of(update => {
          if (update.docChanged) {
            emit('update:modelValue', update.state.doc.toString())
          }
        }),
        keymap.of([
          {
            key: 'Ctrl-Enter',
            run: () => { emit('run'); return true },
          },
          {
            key: 'Mod-Enter',
            run: () => { emit('run'); return true },
          },
        ]),
        EditorView.lineWrapping,
      ],
    }),
    parent: editorEl.value,
  })
})

onBeforeUnmount(() => {
  view?.destroy()
  view = null
})

watch(() => props.modelValue, val => {
  if (view && val !== view.state.doc.toString()) {
    view.dispatch({
      changes: { from: 0, to: view.state.doc.length, insert: val },
    })
  }
})
</script>

<template>
  <div ref="editorEl" class="h-full overflow-hidden" />
</template>
