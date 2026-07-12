<script setup lang="ts">
import { computed, ref } from 'vue'
import { Copy, Sparkles } from '@lucide/vue'
import { copyToClipboard } from '@/lib/ipc'

const model = ref('glm-5.2')
const story = ref('')
const style = ref('电影感写实')
const pace = ref('中等节奏')
const custom = ref('')
const generated = ref(false)

const output = computed(() => `# 故事板提示词\n\n- 模型：${model.value}\n- Skill：storyboard-prompt-optimizer\n- 风格：${style.value}\n- 节奏：${pace.value}\n\n## 故事输入\n${story.value || '请先输入故事或剧情片段。'}\n\n## 创作约束\n${custom.value || '镜头语言清晰，人物动作与场景连续。'}\n\n## 分镜生成指令\n将故事拆为连续镜头；每个镜头写明景别、机位、动作、环境、光线、情绪和可用于生图/生视频的中文提示词。`)

function generate() { generated.value = true }
async function copyMarkdown() { await copyToClipboard(output.value) }
</script>

<template>
  <main class="storyboard-page">
    <header><div><p>Creative agent</p><h2>故事板</h2></div><span>交互式创作采集</span></header>
    <section class="storyboard-form">
      <label><span>模型</span><select
        v-model="model"
        data-field="story-model"
      ><option value="glm-5.2">glm-5.2</option></select></label>
      <label><span>已装载 Skill</span><input
        value="storyboard-prompt-optimizer"
        readonly
      ></label>
      <label class="wide"><span>故事或剧情片段</span><textarea
        v-model="story"
        data-field="story-input"
        rows="4"
        placeholder="输入一句话故事、剧情片段或动作构想"
      /></label>
      <label><span>视觉方向</span><select v-model="style"><option>电影感写实</option><option>动画叙事</option><option>手绘分镜</option><option>其他</option></select></label>
      <label><span>镜头节奏</span><select v-model="pace"><option>中等节奏</option><option>舒缓文戏</option><option>快速动作</option><option>其他</option></select></label>
      <label class="wide"><span>其他自定义要求</span><textarea
        v-model="custom"
        rows="2"
        placeholder="人物、场景、镜头或参考风格"
      /></label>
      <button
        data-action="generate-storyboard"
        type="button"
        @click="generate"
      >
        <Sparkles :size="16" />生成提示词
      </button>
    </section>
    <section
      v-if="generated"
      class="storyboard-output"
      data-storyboard-output
    >
      <pre>{{ output }}</pre><button
        data-action="copy-storyboard-markdown"
        type="button"
        title="复制 Markdown"
        @click="copyMarkdown"
      >
        <Copy :size="16" />复制 Markdown
      </button>
    </section>
  </main>
</template>

<style scoped>
.storyboard-page { min-height:100%; padding:22px; background:var(--bb-bg); }
header { display:flex; align-items:end; justify-content:space-between; padding-bottom:16px; border-bottom:1px solid var(--bb-border); }
header p,header h2 { margin:0; } header p { color:var(--bb-primary); font:11px var(--bb-mono); letter-spacing:.08em; text-transform:uppercase; } header h2 { margin-top:3px; font-size:20px; } header span { color:var(--bb-text-soft); font-size:11px; }
.storyboard-form { display:grid; grid-template-columns:repeat(2,minmax(0,1fr)); gap:12px; margin-top:16px; }
label { display:grid; gap:5px; color:var(--bb-text-muted); font-size:11px; } label.wide { grid-column:span 2; } input,select,textarea { width:100%; min-height:31px; padding:6px 8px; } textarea { resize:vertical; }
.storyboard-form > button,.storyboard-output button { display:inline-flex; width:max-content; min-height:32px; align-items:center; gap:6px; padding:0 11px; border-color:rgba(102,247,211,.45); background:var(--bb-primary); color:#06231f; font-weight:700; }
.storyboard-output { margin-top:16px; padding:14px; border:1px solid var(--bb-border); border-radius:var(--bb-radius-sm); background:var(--bb-surface-soft); } pre { margin:0 0 12px; overflow:auto; color:var(--bb-text); font:12px/1.55 var(--bb-mono); white-space:pre-wrap; }
@media(max-width:620px){ .storyboard-page { padding:14px 12px; } .storyboard-form { grid-template-columns:1fr; } label.wide { grid-column:auto; } }
</style>
