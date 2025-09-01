import { createRouter, createWebHistory } from 'vue-router';
import FundingMatrix from './components/funding/FundingMatrix.vue';
import Trend from './components/Trend.vue'
import ZScore from './components/ZScore.vue'

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
      path: '/trend', 
      name: 'Trend', 
      component: Trend, 
      meta: { title: 'Trend' } 
    },
    { 
      path: '/zscore', 
      name: 'ZScore', 
      component: ZScore, 
      meta: { title: 'Z-Score Analysis' } 
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