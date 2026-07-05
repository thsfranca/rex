precision mediump float;

uniform float uTime;
uniform vec2 uResolution;
uniform float uIntensity;
uniform float uFlowAngle;

float hash(vec2 p) {
  return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}

float noise(vec2 p) {
  vec2 i = floor(p);
  vec2 f = fract(p);
  float a = hash(i);
  float b = hash(i + vec2(1.0, 0.0));
  float c = hash(i + vec2(0.0, 1.0));
  float d = hash(i + vec2(1.0, 1.0));
  vec2 u = f * f * (3.0 - 2.0 * f);
  return mix(a, b, u.x) + (c - a) * u.y * (1.0 - u.x) + (d - b) * u.x * u.y;
}

void main() {
  vec2 uv = gl_FragCoord.xy / uResolution;
  vec2 flow = vec2(cos(uFlowAngle), sin(uFlowAngle));
  float n = noise(uv * 3.5 + flow * uTime * 0.15);
  float band = sin(uTime * 1.2 + uv.x * 8.0 + n * 2.0) * 0.5 + 0.5;
  float pulse = 0.06 + 0.08 * band * uIntensity;
  vec3 electric = vec3(0.35, 0.45, 0.95);
  vec3 teal = vec3(0.25, 0.85, 0.72);
  vec3 color = mix(electric, teal, n) * pulse;
  gl_FragColor = vec4(color, pulse * (0.5 + uIntensity * 0.5));
}
