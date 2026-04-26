import { createRouter, createWebHistory } from 'vue-router';
import Home from '../views/Home.vue';
import Search from '../views/Search.vue';
import GlobalSearch from '../views/GlobalSearch.vue';
import ToolsEdit from '../views/ToolsEdit.vue';
import ToolsManage from '../views/ToolsManage.vue';
import Edit from '../views/Edit.vue';
import Config from '../views/Config.vue';

const routes = [
  {
    path: '/',
    redirect: '/home',
  },
  {
    path: '/home',
    name: 'Home',
    component: Home,
  },
  {
    path: '/search',
    name: 'Search',
    component: Search,
  },
  {
    path: '/global-search',
    name: 'GlobalSearch',
    component: GlobalSearch,
  },
  {
    path: '/toolsedit',
    name: 'ToolsEdit',
    component: ToolsEdit,
  },
  {
    path: '/toolsmanage',
    name: 'ToolsManage',
    component: ToolsManage,
  },
  {
    path: '/edit',
    name: 'edit',
    component: Edit,
  },
  {
    path: '/config',
    name: 'Config',
    component: Config,
  },
];

const router = createRouter({
  history: createWebHistory(),
  routes,
});

export default router;
