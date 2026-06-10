import { h } from 'vue'
import DefaultTheme from 'vitepress/theme'
import GrainientBackground from './components/GrainientBackground.vue'
import './custom.css'

export default {
  extends: DefaultTheme,
  // Render the animated grainient only on the home page (the `home-hero-before`
  // slot exists solely on `layout: home` pages). The canvas is `position:
  // fixed`, so it paints the whole viewport behind the content.
  Layout() {
    return h(DefaultTheme.Layout, null, {
      'home-hero-before': () => h(GrainientBackground),
    })
  },
}
