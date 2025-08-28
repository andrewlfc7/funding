import { createRouter, createWebHistory } from 'vue-router';
import FundingMatrix from './components/FundingMatrix.vue';

// Lazy load the Trends component
const Trends = () => import('./components/Trend.vue');

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { 
      path: '/', 
      redirect: '/funding' 
    },
    { 
      path: '/funding', 
      name: 'Funding', 
      component: FundingMatrix, 
      meta: { title: 'Funding' } 
    },
    { 
      path: '/trends', 
      name: 'Trends', 
      component: Trends, 
      meta: { title: 'Trends' } 
    },
  ],
  scrollBehavior() {
    return { top: 0 };
  },
});

// Update page title based on route
router.afterEach((to) => {
  if (to.meta?.title) {
    document.title = `Dashboard – ${to.meta.title as string}`;
  }
});

export default router;