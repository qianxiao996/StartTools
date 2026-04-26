import { createApp } from "vue";
import AppTools from "./AppTools.vue";
// import store from './store';
import ElementPlus from 'element-plus'
// 如果您正在使用CDN引入，请删除下面一行。
import * as ElementPlusIconsVue from '@element-plus/icons-vue'
import 'element-plus/dist/index.css'
import 'element-plus/theme-chalk/dark/css-vars.css'
import '../../css/dark.css'
const app = createApp(AppTools)
for (const [key, component] of Object.entries(ElementPlusIconsVue)) {
    app.component(key, component)
}
// app.use(store)
app.use(ElementPlus)
// app.use(router); // 使用路由
app.mount('#apptools')
