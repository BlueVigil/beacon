#ifdef GL_ES
precision highp float;
#endif

uniform vec2 u_resolution;
uniform float u_time;

#define TAU 6.28318530718

float hash(vec2 p) {
   return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453123);
}

float sdCircle(vec2 p, float r) {
   return length(p) - r;
}

float sdBox(vec2 p, vec2 b) {
   vec2 d = abs(p) - b;
   return length(max(d, 0.0)) + min(max(d.x, d.y), 0.0);
}

float sdSegment(vec2 p, vec2 a, vec2 b) {
   vec2 pa = p - a;
   vec2 ba = b - a;
   float h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);
   return length(pa - ba * h);
}

mat2 rot(float a) {
   float s = sin(a);
   float c = cos(a);
   return mat2(c, -s, s, c);
}

float fill(float d, float px) {
   return smoothstep(px, -px, d);
}

float stroke(float d, float w, float px) {
   return smoothstep(w + px, w - px, abs(d));
}

float line(vec2 p, vec2 a, vec2 b, float w, float px) {
   return fill(sdSegment(p, a, b) - w, px);
}

float ring(vec2 p, float r, float w, float px) {
   return stroke(sdCircle(p, r), w, px);
}

float diamond(vec2 p, float s, float px) {
   return fill(abs(p.x) + abs(p.y) - s, px);
}

float capsule(vec2 p, vec2 size, float radius, float px) {
   return fill(sdBox(p, size) - radius, px);
}

float petal(vec2 p, float px) {
   p.y -= 0.22;
   float outer = fill(sdCircle(p / vec2(0.62, 1.0), 0.18), px);
   float cut = fill(sdCircle((p - vec2(0.0, -0.055)) / vec2(0.58, 1.0), 0.145), px);
   return max(outer - cut, 0.0);
}

float beaconGlyph(vec2 p, float px) {
   float mark = 0.0;

   mark += ring(p, 0.48, 0.010, px);
   mark += ring(p, 0.33, 0.005, px) * 0.55;

   for (int i = 0; i < 8; i++) {
      vec2 q = p * rot(float(i) * TAU / 8.0);
      mark += petal(q, px);
      mark += diamond(q - vec2(0.0, 0.51), 0.034, px);
   }

   for (int i = 0; i < 32; i++) {
      float a = float(i) * TAU / 32.0;
      vec2 dir = vec2(cos(a), sin(a));
      float n = hash(vec2(float(i), 4.0));
      float len = mix(0.030, 0.095, n);
      mark += line(p, dir * 0.57, dir * (0.57 + len), 0.004, px) * smoothstep(0.22, 0.95, n);
   }

   mark += line(p, vec2(-0.55, 0.0), vec2(0.55, 0.0), 0.006, px) * 0.62;
   mark += line(p, vec2(0.0, -0.55), vec2(0.0, 0.55), 0.006, px) * 0.62;
   mark += diamond(p, 0.110, px);
   mark -= fill(sdCircle(p, 0.074), px) * 0.95;

   return clamp(mark, 0.0, 1.0);
}

float innerCircuit(vec2 p, float px) {
   float c = 0.0;
   for (int i = 0; i < 10; i++) {
      float fi = float(i);
      vec2 a = vec2(-0.34 + hash(vec2(fi, 1.0)) * 0.68, -0.30 + hash(vec2(fi, 2.0)) * 0.60);
      vec2 b = vec2(-0.34 + hash(vec2(fi, 3.0)) * 0.68, -0.30 + hash(vec2(fi, 4.0)) * 0.60);
      c += line(p, a, b, 0.0026, px) * 0.35;
      c += fill(sdCircle(a - p, 0.010), px) * 0.45;
      c += fill(sdCircle(b - p, 0.010), px) * 0.45;
   }
   return clamp(c, 0.0, 1.0);
}

void main() {
   vec2 uv = (gl_FragCoord.xy * 2.0 - u_resolution.xy) / min(u_resolution.x, u_resolution.y);
   float px = 2.0 / min(u_resolution.x, u_resolution.y);

   float grain = hash(floor(gl_FragCoord.xy * 0.55));
   vec3 bg = vec3(0.036, 0.035, 0.032) + (grain - 0.5) * 0.045;
   vec3 ink = vec3(0.86, 0.83, 0.73);
   vec3 red = vec3(1.0, 0.20, 0.18);
   vec3 color = bg;

   float vignette = smoothstep(1.25, 0.12, length(uv));
   color *= mix(0.54, 1.08, vignette);

   float tile = fill(sdBox(uv, vec2(0.92)) - 0.040, px * 2.0);
   float tileEdge = stroke(sdBox(uv, vec2(0.90)) - 0.030, 0.003, px);
   color = mix(color, vec3(0.060, 0.058, 0.053), tile * 0.50);
   color = mix(color, ink, tileEdge * 0.18);

   vec2 p = uv * rot(-0.15);
   float halo = fill(sdCircle(p, 0.70), px) - fill(sdCircle(p, 0.18), px);
   float main = beaconGlyph(p, px);
   float circuits = innerCircuit(p, px) * fill(sdCircle(p, 0.43), px);
   float outerDial = 0.0;

   for (int i = 0; i < 11; i++) {
      outerDial += ring(p, 0.235 + float(i) * 0.035, 0.0022, px) * 0.34;
   }

   for (int i = 0; i < 12; i++) {
      vec2 q = p * rot(float(i) * TAU / 12.0);
      outerDial += diamond(q - vec2(0.0, 0.625), 0.023, px) * 0.80;
   }

   color = mix(color, ink, halo * 0.035);
   color = mix(color, ink, outerDial * 0.42);
   color = mix(color, ink, circuits * 0.34);
   color = mix(color, ink, main * 0.92);

   float coreShadow = fill(sdCircle(uv, 0.105), px);
   float coreRing = ring(uv, 0.122, 0.006, px);
   float diodeGlow = fill(sdCircle(uv, 0.090), px) * 0.22;
   float diode = capsule(uv, vec2(0.022), 0.008, px);
   color = mix(color, vec3(0.010, 0.010, 0.011), coreShadow * 0.88);
   color = mix(color, ink, coreRing * 0.82);
   color = mix(color, red, diodeGlow);
   color = mix(color, red, diode);

   gl_FragColor = vec4(color, 1.0);
}
