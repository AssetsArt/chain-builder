<script setup lang="ts">
// Animated grainy multi-colour gradient ("grainient") rendered with raw WebGL.
// Effect inspired by reactbits.dev/backgrounds/grainient (MIT); shader authored
// here so it runs natively in VitePress (Vue) without React/OGL.
import { onBeforeUnmount, onMounted, ref } from 'vue'

const canvas = ref<HTMLCanvasElement | null>(null)

// brand grainient palette (sRGB 0..1)
const C1 = [0x1b / 255, 0xcf / 255, 0x96 / 255] // #1BCF96 green
const C2 = [0x27 / 255, 0xae / 255, 0xff / 255] // #27aeff blue
const C3 = [0x08 / 255, 0x05 / 255, 0x0b / 255] // #08050b near-black

const VERT = `
attribute vec2 p;
void main() { gl_Position = vec4(p, 0.0, 1.0); }
`

const FRAG = `
precision highp float;
uniform vec2 uRes;
uniform float uTime;
uniform vec3 c1;
uniform vec3 c2;
uniform vec3 c3;

float hash(vec2 p) {
  p = fract(p * vec2(123.34, 456.21));
  p += dot(p, p + 45.32);
  return fract(p.x * p.y);
}

void main() {
  vec2 uv = gl_FragCoord.xy / uRes;
  float t = uTime * 0.05;

  // slowly drifting gradient sources
  vec2 a = vec2(0.16 + 0.10 * sin(t), 0.20 + 0.09 * cos(t * 0.8));
  vec2 b = vec2(0.86 + 0.08 * cos(t * 0.7), 0.06 + 0.08 * sin(t * 1.1));
  vec2 d = vec2(0.64 + 0.12 * sin(t * 0.5), 1.0 + 0.10 * cos(t * 0.6));

  // tighter falloff keeps each colour vivid and the rest deep/dark
  float wa = smoothstep(0.72, 0.04, distance(uv, a));
  float wb = smoothstep(0.66, 0.04, distance(uv, b));
  float wd = smoothstep(0.85, 0.04, distance(uv, d));

  vec3 col = c3;                            // near-black base
  col = mix(col, c1, wa);                   // green
  col = mix(col, c2, wb);                   // blue
  col = mix(col, mix(c2, c3, 0.45), wd * 0.7);

  // vignette toward the dark base at the edges
  float vig = smoothstep(1.25, 0.15, length(uv - 0.5));
  col *= mix(0.62, 1.0, vig);

  // film grain
  float g = hash(uv * uRes * 0.5 + t * 80.0);
  col += (g - 0.5) * 0.10;

  gl_FragColor = vec4(col, 1.0);
}
`

let raf = 0
let gl: WebGLRenderingContext | null = null
let cleanupResize: (() => void) | null = null
let start = 0

function compile(g: WebGLRenderingContext, type: number, src: string) {
  const sh = g.createShader(type)!
  g.shaderSource(sh, src)
  g.compileShader(sh)
  return sh
}

onMounted(() => {
  const cv = canvas.value
  if (!cv) return
  gl = cv.getContext('webgl', { antialias: false, alpha: false })
  if (!gl) return
  const g = gl

  const prog = g.createProgram()!
  g.attachShader(prog, compile(g, g.VERTEX_SHADER, VERT))
  g.attachShader(prog, compile(g, g.FRAGMENT_SHADER, FRAG))
  g.linkProgram(prog)
  g.useProgram(prog)

  const buf = g.createBuffer()
  g.bindBuffer(g.ARRAY_BUFFER, buf)
  g.bufferData(g.ARRAY_BUFFER, new Float32Array([-1, -1, 3, -1, -1, 3]), g.STATIC_DRAW)
  const loc = g.getAttribLocation(prog, 'p')
  g.enableVertexAttribArray(loc)
  g.vertexAttribPointer(loc, 2, g.FLOAT, false, 0, 0)

  const uRes = g.getUniformLocation(prog, 'uRes')
  const uTime = g.getUniformLocation(prog, 'uTime')
  g.uniform3fv(g.getUniformLocation(prog, 'c1'), C1)
  g.uniform3fv(g.getUniformLocation(prog, 'c2'), C2)
  g.uniform3fv(g.getUniformLocation(prog, 'c3'), C3)

  const resize = () => {
    const dpr = Math.min(window.devicePixelRatio || 1, 1.5)
    const w = Math.floor(window.innerWidth * dpr)
    const h = Math.floor(window.innerHeight * dpr)
    cv.width = w
    cv.height = h
    g.viewport(0, 0, w, h)
    g.uniform2f(uRes, w, h)
  }
  resize()
  window.addEventListener('resize', resize)
  cleanupResize = () => window.removeEventListener('resize', resize)

  document.documentElement.classList.add('grainient-on')

  start = performance.now()
  const loop = () => {
    if (!gl) return
    g.uniform1f(uTime, (performance.now() - start) / 1000)
    g.drawArrays(g.TRIANGLES, 0, 3)
    raf = requestAnimationFrame(loop)
  }
  // pause when the tab is hidden
  const onVisibility = () => {
    if (document.hidden) {
      cancelAnimationFrame(raf)
    } else {
      raf = requestAnimationFrame(loop)
    }
  }
  document.addEventListener('visibilitychange', onVisibility)
  const prevCleanup = cleanupResize
  cleanupResize = () => {
    prevCleanup?.()
    document.removeEventListener('visibilitychange', onVisibility)
  }
  loop()
})

onBeforeUnmount(() => {
  cancelAnimationFrame(raf)
  cleanupResize?.()
  document.documentElement.classList.remove('grainient-on')
  const lose = gl?.getExtension('WEBGL_lose_context')
  lose?.loseContext()
  gl = null
})
</script>

<template>
  <canvas ref="canvas" class="grainient-canvas" aria-hidden="true" />
</template>

<style>
.grainient-canvas {
  position: fixed;
  inset: 0;
  width: 100%;
  height: 100%;
  z-index: -1;
  pointer-events: none;
  display: block;
}
</style>
