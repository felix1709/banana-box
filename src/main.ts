import { createApp } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import FloatButton from './components/FloatButton.vue'
import MainRoot from './components/MainRoot.vue'
import './styles/main.css'

// 多窗口：floatbtn 窗口挂载悬浮按钮，其余挂载主应用
const label = getCurrentWindow().label

if (label === 'floatbtn') {
  createApp(FloatButton).mount('#app')
} else {
  createApp(MainRoot).mount('#app')
}
