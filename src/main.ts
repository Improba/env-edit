import { createApp } from 'vue'
import { Dialog, Notify, Quasar } from 'quasar'
import '@quasar/extras/material-icons/material-icons.css'
import 'quasar/src/css/index.sass'
import './styles.css'
import App from './App.vue'

createApp(App).use(Quasar, { plugins: { Dialog, Notify } }).mount('#app')
