import { createApp } from 'vue'
import { createPinia } from 'pinia'
import { getCurrentWindow } from '@tauri-apps/api/window'
import App from './App.vue'
import FloatButton from './components/FloatButton.vue'
import './styles/main.css'

// 多窗口：floatbtn 窗口挂载悬浮按钮，其余挂载主应用
const label = getCurrentWindow().label

if (label === 'floatbtn') {
  createApp(FloatButton).mount('#app')
} else {
  createApp(App).use(createPinia()).mount('#app')
}
