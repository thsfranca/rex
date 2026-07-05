precision mediump float;

uniform vec4 uColor;
varying float vAlpha;

void main() {
  gl_FragColor = vec4(uColor.rgb, uColor.a * vAlpha);
}
