#ifdef GL_ES
precision highp float;
#endif

uniform vec2 u_resolution;
uniform float u_time;

#define PI 3.14159265359
#define TAU 6.28318530718

float hash(vec2 p) {
   return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453123);
}

mat2 rot(float a) {
   float s = sin(a);
   float c = cos(a);
   return mat2(c, -s, s, c);
}

float sdCircle(vec2 p, float r) {
   return length(p) - r;
}

float sdEllipse(vec2 p, vec2 r) {
   return (length(p / r) - 1.0) * min(r.x, r.y);
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

float fill(float d, float px) {
   return smoothstep(px, -px, d);
}

float stroke(float d, float w, float px) {
   return smoothstep(w + px, w - px, abs(d));
}

float line(vec2 p, vec2 a, vec2 b, float w, float px) {
   return fill(sdSegment(p, a, b) - w, px);
}

float diamond(vec2 p, float s, float px) {
   return fill(abs(p.x) + abs(p.y) - s, px);
}

float ring(vec2 p, float r, float w, float px) {
   return stroke(sdCircle(p, r), w, px);
}

float spokeSuccessor(vec2 p, float px) {
   float mark = 0.0;

   for (int i = 0; i < 8; i++) {
      float a = float(i) * TAU / 8.0;
      vec2 q = p * rot(-a);

      float rayLength = 0.52;
      float rayThickness = 0.018;
      float lobeSize = 0.126;
      float lobeY = 0.285;

      if (mod(float(i), 2.0) < 0.5) {
         rayLength = 0.62;
         rayThickness = 0.022;
         lobeSize = 0.154;
         lobeY = 0.330;
      }

      mark += line(q, vec2(0.0, 0.030), vec2(0.0, rayLength), rayThickness, px);
      mark += fill(sdEllipse(q - vec2(0.0, lobeY), vec2(lobeSize * 0.62, lobeSize)), px);
      mark += fill(sdEllipse(q - vec2(0.0, lobeY + 0.115), vec2(lobeSize * 0.42, lobeSize * 0.68)), px);

      float hollowA = fill(sdEllipse(q - vec2(0.052, lobeY + 0.012), vec2(0.045, 0.075)), px);
      float hollowB = fill(sdEllipse(q - vec2(-0.052, lobeY + 0.012), vec2(0.045, 0.075)), px);
      mark -= (hollowA + hollowB) * 0.95;

      mark += diamond(q - vec2(0.0, 0.665), 0.030, px);
      mark += fill(sdCircle(q - vec2(0.0, 0.715), 0.012), px);
   }

   return clamp(mark, 0.0, 1.0);
}

float dial(vec2 p, float px) {
   float d = 0.0;
   d += ring(p, 0.610, 0.010, px);
   d += ring(p, 0.455, 0.006, px);
   d += ring(p, 0.315, 0.004, px) * 0.70;

   for (int i = 0; i < 32; i++) {
      float a = float(i) * TAU / 32.0;
      vec2 dir = vec2(cos(a), sin(a));
      float longTick = step(0.75, hash(vec2(float(i), 4.0)));
      d += line(p, dir * 0.615, dir * (0.650 + longTick * 0.045), 0.0038, px) * 0.72;
   }

   for (int i = 0; i < 8; i++) {
      float a = float(i) * TAU / 8.0;
      vec2 dir = vec2(cos(a), sin(a));
      d += diamond(p - dir * 0.610, 0.034, px);
   }

   return clamp(d, 0.0, 1.0);
}

float cutouts(vec2 p, float px) {
   float c = 0.0;

   for (int i = 0; i < 8; i++) {
      vec2 q = p * rot(-float(i) * TAU / 8.0);
      c += fill(sdEllipse(q - vec2(0.0, 0.235), vec2(0.040, 0.090)), px);
      c += fill(sdEllipse(q - vec2(0.066, 0.360), vec2(0.050, 0.080)), px);
      c += fill(sdEllipse(q - vec2(-0.066, 0.360), vec2(0.050, 0.080)), px);
   }

   c += fill(sdCircle(p, 0.075), px);
   return clamp(c, 0.0, 1.0);
}

float innerNodes(vec2 p, float px) {
   float n = 0.0;

   for (int i = 0; i < 8; i++) {
      vec2 q = p * rot(-float(i) * TAU / 8.0);
      n += fill(sdCircle(q - vec2(0.0, 0.205), 0.018), px);
      n += line(q, vec2(0.0, 0.080), vec2(0.0, 0.285), 0.0025, px) * 0.40;
   }

   return clamp(n, 0.0, 1.0);
}

void main() {
   vec2 uv = (gl_FragCoord.xy * 2.0 - u_resolution.xy) / min(u_resolution.x, u_resolution.y);
   float px = 2.0 / min(u_resolution.x, u_resolution.y);

   float paperNoise = hash(floor(gl_FragCoord.xy * 0.42));
   vec3 paper = vec3(0.705, 0.700, 0.670) + (paperNoise - 0.5) * 0.055;
   vec3 black = vec3(0.020, 0.018, 0.016);
   vec3 red = vec3(0.95, 0.110, 0.095);
   vec3 color = paper;

   float vignette = smoothstep(1.28, 0.20, length(uv));
   color *= mix(0.78, 1.07, vignette);

   vec2 p = uv * 1.05;
   float shadow = fill(sdCircle(p, 0.72), px) * 0.075;
   float shape = max(dial(p, px), spokeSuccessor(p, px));
   float holes = cutouts(p, px);
   float nodes = innerNodes(p, px);

   shape = clamp(shape - holes, 0.0, 1.0);

   color = mix(color, black, shadow);
   color = mix(color, black, shape * 0.96);
   color = mix(color, paper, holes * 0.98);
   color = mix(color, black, nodes * 0.64);

   float coreCut = fill(sdCircle(p, 0.088), px);
   float coreRing = ring(p, 0.112, 0.008, px);
   float diode = fill(sdBox(p, vec2(0.030)) - 0.006, px);
   float glow = fill(sdCircle(p, 0.095), px) * 0.22;

   color = mix(color, paper, coreCut * 0.95);
   color = mix(color, black, coreRing * 0.92);
   color = mix(color, red, glow);
   color = mix(color, red, diode);

   gl_FragColor = vec4(color, 1.0);
}
