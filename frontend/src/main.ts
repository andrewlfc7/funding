import { createApp } from 'vue';
import App from './App.vue';
import router from './router';
import './styles/index.css'; // Changed from './style.css'

createApp(App)
  .use(router)
  .mount('#app');