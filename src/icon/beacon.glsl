#ifdef GL_ES
precision highp float;
#endif

uniform vec2 u_resolution;
uniform float u_time;

#define PI 3.14159265359

mat2 Rot(float a) {
   float s = sin(a), c = cos(a);
   return mat2(c, -s, s, c);
}

float hash(vec2 p) {
   return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453123);
}

float roundCompat(float x) {
   return sign(x) * floor(abs(x) + 0.5);
}

float smin(float a, float b, float k) {
   float h = clamp(0.5 + 0.5 * (b - a) / k, 0.0, 1.0);
   return mix(b, a, h) - k * h * (1.0 - h);
}

float smax(float a, float b, float k) {
   return -smin(-a, -b, k);
}

float sdCapsule(vec3 p, float r, float h) {
   p.y -= clamp(p.y, 0.0, h);
   return length(p) - r;
}

float sdDiamondTorus(vec3 p, float r1, float r2) {
   vec2 q = vec2(length(p.xz) - r1, p.y);
   return abs(q.x) + abs(q.y) - r2;
}

float starSuccessor(vec3 p) {
   float rayLength = 1.3;
   float rayThickness = 0.011;

   float pointAngle = atan(p.z, p.y);
   float numSpokes = 8.0;
   float spokeSpacing = 2.0 * PI / numSpokes;

   float spokeIndex = roundCompat(pointAngle / spokeSpacing);
   float closestSpokeAngle = spokeIndex * spokeSpacing;

   float size = 0.84;
   float give = 0.38;
   vec3 pos = vec3(0.0, 1.5, 0.0);

   float a = abs(closestSpokeAngle);

   if (abs(a) < 0.01 || abs(a - PI) < 0.01) {
      pos = vec3(0.0, 2.2, 0.0);
      rayThickness = 0.011;
      rayLength = 3.44;
   } else if (abs(a - PI / 2.0) < 0.01) {
      pos = vec3(0.0, 1.5, 0.0);
      size = 0.4;
      give = 0.18;
      rayLength = 2.4;
   } else if (abs(a - PI / 4.0) < 0.01 || abs(a - 3.0 * PI / 4.0) < 0.01) {
      pos = vec3(0.0, 1.7, 0.0);
      rayLength = 3.0;
   }

   vec3 spokePt = p;
   spokePt.yz *= Rot(-closestSpokeAngle);

   float rays = sdCapsule(spokePt, rayThickness, rayLength);

   vec3 torusPos = spokePt - pos;
   torusPos.xy *= Rot(PI / 2.0);
   float torus = sdDiamondTorus(torusPos, size, 0.032);

   return smin(rays, torus, give);
}

float dial(vec3 p) {
   p.xy *= Rot(PI / 2.0);
   float d = sdDiamondTorus(p, 1.0, 0.05);
   d = smin(d, sdDiamondTorus(p, 1.5, 0.02), 0.04);
   d = smin(d, sdDiamondTorus(p, 1.9, 0.012), 0.03);
   return d;
}

float Form(vec3 p) {
   float dialDist = dial(p);
   float starDist = starSuccessor(p);
   float form = smin(dialDist, starDist, 0.22);
   float hub = length(p) - 0.22;
   return smin(form, hub, 0.14);
}

vec3 normalAt(vec3 p) {
   vec2 e = vec2(0.002, 0.0);
   return normalize(vec3(
      Form(p + e.xyy) - Form(p - e.xyy),
      Form(p + e.yxy) - Form(p - e.yxy),
      Form(p + e.yyx) - Form(p - e.yyx)
   ));
}

void main() {
   vec2 uv = (gl_FragCoord.xy * 2.0 - u_resolution.xy) / min(u_resolution.x, u_resolution.y);
   uv *= 0.88;

   float grain = hash(floor(gl_FragCoord.xy * 0.45));
   vec3 paper = vec3(0.72, 0.71, 0.67) + (grain - 0.5) * 0.035;
   vec3 ink = vec3(0.025, 0.022, 0.019);
   vec3 color = paper;

   vec3 ro = vec3(6.0, 0.0, 0.0);
   vec3 rd = normalize(vec3(-1.35, uv.y, uv.x));

   float distanceMarched = 0.0;
   float distToShape = 0.0;
   bool hit = false;

   // FIXED: Doubled the steps and drastically lowered the step speed.
   // This guarantees the ray won't clip through the smoothed geometry.
   for (int i = 0; i < 300; i++) { 
      vec3 p = ro + rd * distanceMarched;
      distToShape = Form(p);
      
      distanceMarched += distToShape * 0.55; 

      if (distanceMarched > 20.0) {
         break;
      }

      if (abs(distToShape) < 0.0005) { 
         hit = true;
         break;
      }
   }

   // FIXED: Solidified the object completely. 
   // It no longer mixes with the 'paper' variable based on lighting.
   if (hit) {
      vec3 p = ro + rd * distanceMarched;
      vec3 n = normalAt(p);
      
      // Calculate a simple diffuse light
      float diffuse = clamp(dot(n, normalize(vec3(0.6, 0.9, 0.4))), 0.0, 1.0);
      
      // Keep the ink completely opaque, just slightly lighten the ink color where light hits
      float lightIntensity = 0.85 + 0.3 * diffuse; 
      color = ink * lightIntensity; 
   }

   float center = length(uv);
   float diode = smoothstep(0.090, 0.030, abs(uv.x) + abs(uv.y));
   float glow = smoothstep(0.24, 0.0, center) * 0.18;
   color = mix(color, vec3(0.94, 0.08, 0.07), glow);
   color = mix(color, vec3(0.98, 0.10, 0.09), diode);

   float vignette = smoothstep(1.55, 0.35, length(uv * 0.48));
   color *= mix(0.82, 1.04, vignette);

   gl_FragColor = vec4(color, 1.0);
}
