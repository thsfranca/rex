precision mediump float;

attribute vec2 position;
attribute vec2 offset;
attribute float size;
attribute float alpha;
uniform vec2 uResolution;
varying float vAlpha;

void main() {
  vec2 center = offset + position * size;
  vec2 clip = (center / uResolution) * 2.0 - 1.0;
  gl_Position = vec4(clip.x, -clip.y, 0.0, 1.0);
  vAlpha = alpha;
}
