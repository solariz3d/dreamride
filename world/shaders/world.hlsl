// world.hlsl: every shader of the Luminesce world (W2, the original theme: the void).
// Compiled at launch by dxcompiler.dll (SM 6.5: RayQuery inline raytracing in the floor shader).
// The look's numbers come from the host's Theme (main.cpp) through the frame constants.
//
// Coordinates are relative to the present: x is time (the draw-head is at x = 0, the past is
// x < 0), y is up (the floor is y = 0), z points from the wall (z = 0) toward the camera's side.

#define MAX_VOICES 32
static const float PI = 3.14159265;

struct Frame {
  row_major float4x4 viewProj;
  row_major float4x4 invViewProj;
  float4 camPos;     // xyz; w: world size of one pixel at distance 1
  float4 world;      // x beatLen (m), y now (beats), z bar phase in [0,1), w bar length (m)
  float4 fog;        // rgb fog colour, w e-fold distance (m)
  float4 fogP;       // x fog start (m), y recency e-fold (beats), z recency boost, w exposure
  float4 irrRegion;  // x0, z0, size x, size z of the floor light texture
  float4 floorP;     // x GGX alpha, y F0, z albedo, w grid strength
  float4 screen;     // w, h, 1/w, 1/h
  uint4  counts;     // x segments, y heads, z reflection rays, w MSAA samples
  float4 post;       // x bloom strength, y vignette, z floor light scale, w time (s)
  float4 voice[MAX_VOICES];  // rgb colour, w intensity
  float4 debug;              // x: 0 off, 1 floor light only, 2 reflection only (x10)
  float4 expr;               // x onset flare, y flare e-fold (beats), z pitch hue (rad), w scatter
  float4 rippleP;            // x speed (m/s), y width (m), z decay (s), w wavenumber (rad/m)
  float4 dustP;              // x count, y brightness, z distance travelled mod DUST_PERIOD, w size (px)
  float4 ripple[4];          // x, z, age (s), amplitude (0: none)
  float4 lightPos[4];        // xyz, w the light's own radius
  float4 lightCol[4];        // rgb x intensity (0: none)
  float4 bass[6];            // the bass on the water, per note: x, z, k (rad/m), phase (rad)
  float4 bassB[6];           // r0 (inner edge, m), r1 (front, m), height (m; 0: none), e-fold distance (m)
};
#define DUST_PERIOD 72.0
ConstantBuffer<Frame> F : register(b1);

// Root constants: their meaning is per shader.
struct RootC { float4 c0; float4 c1; float4 c2; float4 c3; };
ConstantBuffer<RootC> R : register(b0);

struct TubeVtxS { float3 center; float3 dir; float radius; uint voice; float level; float noteX; float pitch; float pad; };
struct Head { float3 pos; float radius; float3 color; float intensity; };
struct Seg { float3 a; float pad0; float3 b; float pad1; float3 power; float pad2; };

RaytracingAccelerationStructure Scene : register(t0);
StructuredBuffer<TubeVtxS> Verts : register(t1);
StructuredBuffer<uint> Indices : register(t2);
StructuredBuffer<Head> Heads : register(t3);
StructuredBuffer<Seg> Segs : register(t3);   // the light pass binds segments where the heads go

Texture2D<float4> Tex0 : register(t4);
Texture2D<float4> Tex1 : register(t5);
Texture2DMS<float4> Msaa : register(t4);
RWTexture2D<float4> Out0 : register(u0);
SamplerState sLinear : register(s0);

float Luma(float3 c) { return dot(c, float3(0.2126, 0.7152, 0.0722)); }

// The past fades into fog with distance from the eye.
float FogKeep(float d) { return exp(-max(d - F.fogP.x, 0.0) / F.fog.w); }
float3 ApplyFog(float3 c, float d) { return lerp(F.fog.rgb, c, FogKeep(d)); }

// The lights in the air (the present, and drum hits as they land) scattering in the fog along an
// eye ray: the single-scattering integral of an isotropic point light over the ray from o to
// o + d L, in closed form, (atan((L - b) / h) + atan(b / h)) / h, with b the light's distance along
// the ray and h its distance from it (held above the light's own radius).
float3 InScatter(float3 o, float3 d, float L) {
  float3 s = 0;
  [unroll] for (uint k = 0; k < 4; ++k) {
    float3 lc = F.lightCol[k].rgb;
    if (dot(lc, 1) <= 0) continue;
    float3 op = F.lightPos[k].xyz - o;
    float b = dot(op, d);
    float h = sqrt(max(dot(op, op) - b * b, F.lightPos[k].w * F.lightPos[k].w));
    s += lc * (atan((L - b) / h) + atan(b / h)) / h;
  }
  return s * F.expr.w;
}

// Fog and the air's own light, for a surface at P seen from the eye.
float3 Atmos(float3 c, float3 P) {
  float3 d = P - F.camPos.xyz;
  float L = length(d);
  return ApplyFog(c, L) + InScatter(F.camPos.xyz, d / L, L);
}

// A hue turn about the grey axis (Rodrigues), luminance kept.
float3 HueRot(float3 c, float a) {
  const float3 k = 0.57735027;
  float ca = cos(a), sa = sin(a);
  return max(c * ca + cross(k, c) * sa + k * dot(k, c) * (1.0 - ca), 0.0);
}

// A tube's light: the voice's colour, tinted by its pitch class (a turn of hue by the note's place on
// the circle of fifths, so related chords keep related tints and an octave is the same tint),
// brighter at the core facing the eye, brightest near the present, flaring at its note's onset
// (the whole new segment), scaled by the note's level. noteRel: where the note began, relative to
// the present (m); pitch in semitones (below -500 for drums: no tint).
float3 TubeEmission(uint v, float level, float x, float ndv, float noteRel, float pitch) {
  float4 vc = F.voice[min(v, MAX_VOICES - 1)];
  float age = max(-x, 0.0) / F.world.x;
  float recency = 1.0 + F.fogP.z * exp(-age / F.fogP.y);
  float noteAge = -noteRel / F.world.x;
  float flare = noteAge >= 0.0 ? F.expr.x * exp(-noteAge / F.expr.y) : 0.0;
  float3 base = pitch > -500.0 ? HueRot(vc.rgb, F.expr.z * sin(2.0 * PI * pitch * 7.0 / 12.0)) : vc.rgb;
  float core = ndv * ndv;
  float3 c = base * (0.3 + 0.7 * core) + pow(ndv, 8.0) * 0.45 * (base * 0.5 + 0.5);
  return c * vc.w * level * (recency + flare);
}

// ---------------------------------------------------------------- sky (the void)

struct FsOut { float4 pos : SV_Position; float2 uv : UV; };
FsOut FullscreenVS(uint id : SV_VertexID) {
  FsOut o;
  float2 uv = float2((id << 1) & 2, id & 2);
  o.pos = float4(uv * float2(2, -2) + float2(-1, 1), 0, 1);
  o.uv = uv;
  return o;
}

float3 ViewRay(float2 uv) {
  float2 ndc = uv * float2(2, -2) + float2(-1, 1);
  float4 a = mul(F.invViewProj, float4(ndc, 1, 1));
  float4 b = mul(F.invViewProj, float4(ndc, 0.5, 1));
  return normalize(b.xyz / b.w - a.xyz / a.w);
}

float3 SkyColor(float3 d) {
  // A faint band of haze at the horizon, darkening upward: depth without a single object.
  float h = saturate(d.y);
  return F.fog.rgb * (0.55 + 0.9 * exp(-h * 9.0)) ;
}

float4 SkyPS(FsOut i) : SV_Target {
  float3 d = ViewRay(i.uv);
  return float4(SkyColor(d) + InScatter(F.camPos.xyz, d, 200.0), 1);
}

// ---------------------------------------------------------------- tubes and dashes

struct TubeIn { float3 center : CENTER; float3 dir : DIR; float radius : RADIUS; uint voice : VOICE; float level : LEVEL; float noteX : NOTEX; float pitch : PITCH; };
struct TubeOut {
  float4 pos : SV_Position;
  float3 wpos : WPOS;
  float3 n : NORMAL;
  nointerpolation uint voice : VOICE;
  float level : LEVEL;
  float iscale : ISCALE;
  nointerpolation float noteRel : NOTEX;
  float pitch : PITCH;
};

TubeOut TubeVS(TubeIn v) {
  TubeOut o;
  float3 c = v.center;
  c.x += R.c0.x;  // this copy's offset from the present (the loop it belongs to)
  // Never thinner than about a pixel: a tube far away widens to a pixel and dims by the same
  // ratio, so thin lines neither vanish nor crawl.
  float px = distance(c, F.camPos.xyz) * F.camPos.w;
  float r = max(v.radius, 0.85 * px);
  float3 p = c + v.dir * r;
  o.pos = mul(F.viewProj, float4(p, 1));
  o.wpos = p;
  o.n = v.dir;
  o.voice = v.voice;
  o.level = v.level;
  o.iscale = v.radius / r;
  o.noteRel = v.noteX + R.c0.x;
  o.pitch = v.pitch;
  return o;
}

float4 TubePS(TubeOut i) : SV_Target {
  if (i.wpos.x > 0.0) discard;  // nothing ahead of the present is drawn
  float3 V = normalize(F.camPos.xyz - i.wpos);
  float ndv = saturate(abs(dot(normalize(i.n), V)));
  float3 e = TubeEmission(i.voice, i.level, i.wpos.x, ndv, i.noteRel, i.pitch) * i.iscale;
  return float4(Atmos(e, i.wpos), 1);
}

// ---------------------------------------------------------------- draw-heads (the present)

struct HeadIn { float3 p : POSITION; };
struct HeadOut { float4 pos : SV_Position; float3 wpos : WPOS; float3 n : NORMAL; nointerpolation uint id : HEAD; };

HeadOut HeadVS(HeadIn v, uint inst : SV_InstanceID) {
  Head h = Heads[inst];
  HeadOut o;
  float3 p = h.pos + v.p * h.radius;
  o.pos = mul(F.viewProj, float4(p, 1));
  o.wpos = p;
  o.n = v.p;
  o.id = inst;
  return o;
}

float3 HeadEmission(Head h, float ndv) {
  return h.color * h.intensity * (0.5 + 0.5 * ndv) + pow(ndv, 4.0) * h.intensity * 0.6;
}

float4 HeadPS(HeadOut i) : SV_Target {
  Head h = Heads[i.id];
  float3 V = normalize(F.camPos.xyz - i.wpos);
  float ndv = saturate(dot(normalize(i.n), V));
  return float4(Atmos(HeadEmission(h, ndv), i.wpos), 1);
}

// ---------------------------------------------------------------- dust (the air)

// Three floats in [0, 1) from an integer (pcg3d, Jarzynski and Olano 2020).
float3 Hash3(uint n) {
  uint3 v = uint3(n, n * 1664525u + 1013904223u, n ^ 0x9E3779B9u);
  v = v * 1664525u + 1013904223u;
  v.x += v.y * v.z; v.y += v.z * v.x; v.z += v.x * v.y;
  v ^= v >> 16;
  v.x += v.y * v.z; v.y += v.z * v.x; v.z += v.x * v.y;
  return float3(v & 0xFFFFFFu) / 16777216.0;
}

struct DustOut { float4 pos : SV_Position; float2 q : CORNER; nointerpolation float3 col : COLOR; };

// Motes still in the world, so they recede with the past, lit by the graph: the light its tubes
// throw on the floor (carried up into the air, fading with height) and the flashes in the air. Only
// behind the present, never close to the eye; a fixed size in pixels, so none of them crawls.
DustOut DustVS(uint vid : SV_VertexID, uint iid : SV_InstanceID) {
  DustOut o;
  float3 h = Hash3(iid), h2 = Hash3(iid + 7919u);
  float ph = 2.0 * PI * F.dustP.z / DUST_PERIOD;  // whole turns per period: continuous at the wrap
  float3 P;
  P.x = frac(h.x - F.dustP.z / DUST_PERIOD) * DUST_PERIOD - (DUST_PERIOD - 6.0);
  P.y = 0.08 + h.y * 6.4 + 0.12 * sin(2.0 * ph + h2.x * 2.0 * PI);
  P.z = -3.0 + h.z * 12.0 + 0.15 * sin(ph + h2.y * 2.0 * PI);
  float2 uv = (P.xz - F.irrRegion.xy) / F.irrRegion.zw;
  float edge = saturate(min(min(uv.x, 1 - uv.x), min(uv.y, 1 - uv.y)) * 12.0);
  float3 L = Tex0.SampleLevel(sLinear, uv, 0).rgb * edge * exp(-P.y * 0.35) * 0.04;
  [unroll] for (uint k = 0; k < 4; ++k) {
    float3 dp = F.lightPos[k].xyz - P;
    L += F.lightCol[k].rgb * 0.03 / (dot(dp, dp) + 0.5);
  }
  float dCam = distance(P, F.camPos.xyz);
  float fade = FogKeep(dCam) * smoothstep(1.5, 3.5, dCam) * smoothstep(0.5, -1.5, P.x);
  o.col = L * F.dustP.y * (0.4 + 1.2 * h2.z * h2.z) * fade;
  const float2 corners[6] = { float2(-1, -1), float2(1, -1), float2(-1, 1), float2(-1, 1), float2(1, -1), float2(1, 1) };
  float2 c = corners[vid];
  float4 cp = mul(F.viewProj, float4(P, 1));
  cp.xy += c * F.dustP.w * 2.0 * F.screen.zw * cp.w;
  o.pos = cp;
  o.q = c;
  return o;
}

float4 DustPS(DustOut i) : SV_Target {
  return float4(i.col * exp(-3.0 * dot(i.q, i.q)), 0);
}

// ---------------------------------------------------------------- the floor

struct FloorOut { float4 pos : SV_Position; float3 wpos : WPOS; };

FloorOut FloorVS(uint id : SV_VertexID) {
  // Two triangles, a large plane under the wall and the camera.
  const float2 c[6] = { float2(-400, -150), float2(60, -150), float2(60, 200),
                        float2(-400, -150), float2(60, 200), float2(-400, 200) };
  FloorOut o;
  float3 p = float3(c[id].x, 0, c[id].y);
  o.pos = mul(F.viewProj, float4(p, 1));
  o.wpos = p;
  return o;
}

// Interleaved gradient noise (Jimenez 2014): a per-pixel rotation for the reflection samples.
float IGN(float2 p) { return frac(52.9829189 * frac(dot(p, float2(0.06711056, 0.00583715)))); }

float RaySphere(float3 o, float3 d, float3 c, float r) {
  float3 oc = o - c;
  float b = dot(oc, d);
  float h = b * b - (dot(oc, oc) - r * r);
  if (h < 0) return 1e30;
  float t = -b - sqrt(h);
  return t > 1e-3 ? t : 1e30;
}

// What a reflected ray sees: the nearest tube or dash that is not in the future (candidates ahead
// of the present are rejected inside the traversal), or a draw-head, fogged along the ray.
float3 TraceEmission(float3 o, float3 d, out float hitDist) {
  RayQuery<RAY_FLAG_NONE> q;
  RayDesc ray;
  ray.Origin = o;
  ray.Direction = d;
  ray.TMin = 1e-3;
  ray.TMax = 300.0;
  q.TraceRayInline(Scene, RAY_FLAG_NONE, 0xFF, ray);
  while (q.Proceed()) {
    if (q.CandidateType() == CANDIDATE_NON_OPAQUE_TRIANGLE) {
      float hx = o.x + d.x * q.CandidateTriangleRayT();
      if (hx <= 0.0) q.CommitNonOpaqueTriangleHit();
    }
  }
  float tHit = 1e30;
  float3 e = 0;
  if (q.CommittedStatus() == COMMITTED_TRIANGLE_HIT) {
    tHit = q.CommittedRayT();
    uint prim = q.CommittedPrimitiveIndex();
    float2 b = q.CommittedTriangleBarycentrics();
    TubeVtxS v0 = Verts[Indices[prim * 3 + 0]];
    TubeVtxS v1 = Verts[Indices[prim * 3 + 1]];
    TubeVtxS v2 = Verts[Indices[prim * 3 + 2]];
    float w0 = 1.0 - b.x - b.y;
    float level = v0.level * w0 + v1.level * b.x + v2.level * b.y;
    float pitch = v0.pitch * w0 + v1.pitch * b.x + v2.pitch * b.y;
    float3 n = normalize(v0.dir * w0 + v1.dir * b.x + v2.dir * b.y);
    float hx = o.x + d.x * tHit;
    // This copy's offset from the song's own x: the hit against the vertices' (surface) x.
    float sx = (v0.center.x + v0.dir.x * v0.radius) * w0 + (v1.center.x + v1.dir.x * v1.radius) * b.x
             + (v2.center.x + v2.dir.x * v2.radius) * b.y;
    e = TubeEmission(v0.voice, level, hx, saturate(abs(dot(n, d))), v0.noteX + (hx - sx), pitch);
  }
  for (uint k = 0; k < F.counts.y; ++k) {
    Head h = Heads[k];
    float t = RaySphere(o, d, h.pos, h.radius);
    if (t < tHit) {
      tHit = t;
      float3 n = normalize(o + d * t - h.pos);
      e = HeadEmission(h, saturate(-dot(n, d)));
    }
  }
  hitDist = min(tHit, 300.0);
  if (tHit >= 1e29) return SkyColor(d);
  return ApplyFog(e, tHit);
}

float GridLine(float u, float width) {
  // Anti-aliased line at every integer of u, faded where the lines get finer than a pixel.
  float w = fwidth(u);
  float d = abs(frac(u + 0.5) - 0.5);
  float ln = 1.0 - smoothstep(width, width + w * 1.5, d);
  return ln * saturate(1.0 - w * 3.0);
}

// The floor's normal: flat, but a kick rings it. Each ripple is a travelling wave packet, a ring
// whose front moves out from under the kick at rippleP.x m/s, a Gaussian rippleP.y wide, dying
// with an e-fold of rippleP.z s; the slope it adds is cos(k s) under that envelope.
float3 FloorNormal(float3 P) {
  float2 g = 0;
  [unroll] for (uint k = 0; k < 4; ++k) {
    float4 r = F.ripple[k];
    if (r.w <= 0.0) continue;
    float2 dxz = P.xz - r.xy;
    float dist = length(dxz) + 1e-4;
    float s = dist - r.z * F.rippleP.x;
    float env = r.w * exp(-r.z / F.rippleP.z) * exp(-s * s / (F.rippleP.y * F.rippleP.y));
    g += env * cos(F.rippleP.w * s) * dxz / dist;
  }
  // The bass on the water (BuildBassWaves, water.h): each note's standing waves, summed with the kicks' into one
  // surface. Height a cos(k r - phase) under an envelope: the e-fold with distance (the waves' decay at their speed),
  // the 1 / sqrt(r) of a spreading circle (held finite inside the first wavelength), and the stretch from the inner
  // edge (where a stopped note's drive ended) to the front (how far its energy has gone, at the group velocity),
  // which crests a little as it arrives. The slope is the height's derivative, a k sin(...). Where a wavelength
  // spans under a few pixels it fades out instead of shimmering.
  float dCam = distance(F.camPos.xyz, P);
  float fp = dCam * F.camPos.w / max(F.camPos.y / dCam, 0.05);  // a pixel's length on the floor, along the view
  [unroll] for (uint b = 0; b < 6; ++b) {
    float4 q = F.bass[b], e = F.bassB[b];
    if (e.z <= 0.0) continue;
    float2 dxz = P.xz - q.xy;
    float r = length(dxz) + 1e-4;
    float lam = 6.2831853 / q.z;
    float win = smoothstep(e.x - lam, e.x, r) * (1.0 - smoothstep(e.y - 0.25 * lam, e.y + 0.25 * lam, r));
    if (win <= 0.0) continue;
    float front = 1.0 + 0.8 * exp(-(r - e.y) * (r - e.y) / (lam * lam));
    float env = e.z * exp(-r / e.w) * rsqrt(1.0 + r / lam) * win * front * smoothstep(2.0, 6.0, lam / fp);
    g += -env * q.z * sin(q.z * r - q.w) * dxz / r;
  }
  return normalize(float3(-g.x, 1.0, -g.y));
}

float4 FloorPS(FloorOut i) : SV_Target {
  float3 P = float3(i.wpos.x, 0, i.wpos.z);
  float3 V = normalize(F.camPos.xyz - P);
  float dCam = distance(F.camPos.xyz, P);

  // The graph's light on the floor (an irradiance texture, recomputed every frame).
  float2 uv = (P.xz - F.irrRegion.xy) / F.irrRegion.zw;
  float edge = saturate(min(min(uv.x, 1 - uv.x), min(uv.y, 1 - uv.y)) * 12.0);
  float3 E = Tex0.SampleLevel(sLinear, uv, 0).rgb * edge;
  float3 col = (F.floorP.z / PI) * E;

  // Markers for scale: bar lines across the floor and faint lanes along it, only in the past.
  float bars = GridLine(P.x / F.world.w + F.world.z, 0.006);
  float beats = GridLine(P.x / F.world.x + F.world.z * 4.0, 0.004) * 0.35;
  float lanes = GridLine(P.z / 1.0, 0.004) * 0.25;
  float past = smoothstep(0.0, -0.4, P.x);
  float grid = max(bars, max(beats, lanes)) * past * F.floorP.w;
  col += grid * (0.02 + 0.6 * E * F.floorP.z) ;

  // The near-mirror reflection: raytraced (and blurred only as much as the finish asks) by
  // ReflectCS / ReflectBlurCS, read back here. Fresnel by the rippled normal.
  float3 refl = Tex1.SampleLevel(sLinear, i.pos.xy * F.screen.zw, 0).rgb;
  float ndv = saturate(dot(FloorNormal(P), V));
  float fres = F.floorP.y + (1.0 - F.floorP.y) * pow(1.0 - ndv, 5.0);
  col += refl * fres;
  if (F.debug.x == 1) return float4(E * 0.2, 1);
  if (F.debug.x == 2) return float4(refl * 10.0, 1);
  if (F.debug.x == 3) { float hd = Tex1.SampleLevel(sLinear, i.pos.xy * F.screen.zw, 0).a; return float4(hd >= 299 ? float3(0.3, 0, 0) : frac(hd / 4.0).xxx, 1); }
  return float4(Atmos(col, P), 1);
}

// ---------------------------------------------------------------- the floor's reflection (compute)

// The floor is the plane y = 0, so every pixel's floor point is found from the eye ray alone. One
// mirror ray per pixel, traced against the song; the gloss is a blur afterwards,
// so the result has no sampling noise and holds still as the camera moves.
bool FloorPoint(float2 uv, out float3 P, out float tCam) {
  float3 d = ViewRay(uv);
  P = 0; tCam = 0;
  if (d.y > -1e-4) return false;
  tCam = -F.camPos.y / d.y;
  P = F.camPos.xyz + d * tCam;
  return true;
}

// Up to four mirror rays per pixel at fixed sub-pixel offsets (a rotated grid): the reflection of a
// line a few millimetres thick is antialiased like the line itself, and since the offsets never
// change, nothing shimmers frame to frame.
static const float2 kRgss[4] = { float2(0.125, 0.375), float2(-0.375, 0.125), float2(0.375, -0.125), float2(-0.125, -0.375) };

[numthreads(8, 8, 1)]
void ReflectCS(uint3 id : SV_DispatchThreadID) {
  uint2 size = uint2(R.c0.xy);
  if (id.x >= size.x || id.y >= size.y) return;
  uint n = clamp(F.counts.z, 1u, 4u);
  float3 sum = 0; float hsum = 0; uint m = 0;
  for (uint k = 0; k < n; ++k) {
    float2 off = n == 1 ? 0.0 : kRgss[k];
    float2 uv = (float2(id.xy) + 0.5 + off) / float2(size);
    float3 P; float tCam;
    if (!FloorPoint(uv, P, tCam)) continue;
    float3 d = normalize(P - F.camPos.xyz);
    float3 N = FloorNormal(P);
    float3 rd = reflect(d, N);
    rd.y = max(rd.y, 1e-3);  // a ripple must not tip a grazing ray under the floor
    float hit;
    sum += TraceEmission(P + float3(0, 1e-3, 0), normalize(rd), hit);
    // The blur reads this distance, capped: a missed ray (300 m) must not make the gloss jump from
    // tight to huge at every edge between a hit and a miss, which draws contour rings.
    hsum += min(hit, 8.0);
    ++m;
  }
  Out0[id.xy] = m ? float4(sum / m, hsum / m) : 0;
}

// A glossy floor smears a reflection along the view direction, most where the reflected thing is
// far above the floor: the wet-street streak. A Gaussian gather along R.c1.xy (pixels), its width
// the reflection's spread (hit distance x roughness) as seen from the eye, times R.c1.z.
[numthreads(8, 8, 1)]
void ReflectBlurCS(uint3 id : SV_DispatchThreadID) {
  uint2 size = uint2(R.c0.xy);
  if (id.x >= size.x || id.y >= size.y) return;
  float2 uv = (float2(id.xy) + 0.5) / float2(size);
  float4 c = Tex0.Load(int3(id.xy, 0));
  float3 P; float tCam;
  if (!FloorPoint(uv, P, tCam)) { Out0[id.xy] = c; return; }
  float pixelAngle = F.camPos.w * (F.screen.y / size.y);
  float radius = min(c.a * F.floorP.x / max(tCam * pixelAngle, 1e-4) * R.c1.z, 128.0);
  // A near-mirror: where the gloss is under a pixel the supersampled mirror image stands as it is.
  // Between half a pixel and a pixel and a half the blur fades in, so there is no seam.
  if (radius < 0.5) { Out0[id.xy] = c; return; }
  float blend = saturate(radius - 0.5);
  radius = max(radius, 1.0);
  // Taps no more than a pixel and a half apart, so a wide streak stays smooth.
  int N = (int)clamp(ceil(radius / 1.5), 2.0, 64.0);
  float sigma = radius * 0.5;
  float3 sum = 0; float wsum = 0;
  for (int k = -N; k <= N; ++k) {
    float off = radius * k / N;
    float w = exp(-0.5 * off * off / (sigma * sigma));
    float2 q = uv + R.c1.xy * off / float2(size);
    sum += Tex0.SampleLevel(sLinear, q, 0).rgb * w;
    wsum += w;
  }
  Out0[id.xy] = float4(lerp(c.rgb, sum / wsum, blend), c.a);
}

// ---------------------------------------------------------------- the floor's light (compute)

// Irradiance at P on an up-facing floor from a thin uniform line emitter A..B of unit power per
// unit length, radiating evenly in all directions: the integral of (n.d) / (4 pi |d|^3) along the
// line, in closed form. With a = A - P, u the unit direction, b = a.u, c = |a|^2, q = c - b^2 and
// R(s) = sqrt(s^2 + 2bs + c), the antiderivative is [a.y (s + b) - u.y (b s + c)] / (q R(s)).
float LineIrradiance(float3 P, float3 A, float3 B) {
  float3 u = B - A;
  float len = length(u);
  float3 a = A - P;
  if (len < 1e-5) {
    float r2 = max(dot(a, a), 1e-4);
    return max(a.y, 0.0) * rsqrt(r2) / r2 * len / (4.0 * PI);
  }
  u /= len;
  float b = dot(a, u);
  float c = dot(a, a);
  float q = max(c - b * b, 1e-5);
  float r0 = sqrt(c);
  float r1 = sqrt(max(len * len + 2.0 * b * len + c, 1e-8));
  float f1 = (a.y * (len + b) - u.y * (b * len + c)) / (q * r1);
  float f0 = (a.y * b - u.y * c) / (q * r0);
  return max(f1 - f0, 0.0) / (4.0 * PI);
}

#define SEG_TILE 128
groupshared Seg sSegs[SEG_TILE];

[numthreads(8, 8, 1)]
void FloorLightCS(uint3 id : SV_DispatchThreadID, uint gi : SV_GroupIndex) {
  uint2 size = uint2(R.c0.xy);
  float2 uv = (float2(id.xy) + 0.5) / float2(size);
  float3 P = float3(F.irrRegion.x + uv.x * F.irrRegion.z, 0, F.irrRegion.y + uv.y * F.irrRegion.w);
  float3 E = 0;
  uint count = F.counts.x;
  for (uint base = 0; base < count; base += SEG_TILE) {
    GroupMemoryBarrierWithGroupSync();
    for (uint k = gi; k < SEG_TILE; k += 64) {
      if (base + k < count) sSegs[k] = Segs[base + k];
    }
    GroupMemoryBarrierWithGroupSync();
    uint m = min(SEG_TILE, count - base);
    for (uint k = 0; k < m; ++k) {
      E += sSegs[k].power * LineIrradiance(P, sSegs[k].a, sSegs[k].b);
    }
  }
  if (id.x < size.x && id.y < size.y) Out0[id.xy] = float4(E * F.post.z, 1);
}

// ---------------------------------------------------------------- resolve, bloom, tonemap

// MSAA resolve weighted by 1 / (1 + luminance): a bright line's samples do not drown the dark
// samples beside it, so the edge keeps its coverage instead of stepping (Karis 2014).
[numthreads(8, 8, 1)]
void ResolveCS(uint3 id : SV_DispatchThreadID) {
  if (id.x >= (uint)F.screen.x || id.y >= (uint)F.screen.y) return;
  float3 sum = 0;
  float wsum = 0;
  uint s = F.counts.w;
  for (uint k = 0; k < s; ++k) {
    float3 c = Msaa.Load(int2(id.xy), k).rgb;
    float w = 1.0 / (1.0 + Luma(c) * F.fogP.w);
    sum += c * w;
    wsum += w;
  }
  Out0[id.xy] = float4(sum / wsum, 1);
}

// Downsample with the 13-tap filter (Jimenez, "Next Generation Post Processing in Call of Duty:
// Advanced Warfare", 2014); the first pass weights each group by 1 / (1 + luminance) so a single
// bright pixel cannot flicker the bloom.
float3 Grp(float3 a, float3 b, float3 c, float3 d, bool karis) {
  if (!karis) return (a + b + c + d) * 0.25;
  float wa = 1 / (1 + Luma(a)), wb = 1 / (1 + Luma(b)), wc = 1 / (1 + Luma(c)), wd = 1 / (1 + Luma(d));
  return (a * wa + b * wb + c * wc + d * wd) / (wa + wb + wc + wd);
}

[numthreads(8, 8, 1)]
void BloomDownCS(uint3 id : SV_DispatchThreadID) {
  float2 dst = R.c0.xy;
  float2 texel = R.c0.zw;  // source texel size
  bool karis = R.c1.x > 0.5;
  if (id.x >= (uint)dst.x || id.y >= (uint)dst.y) return;
  float2 uv = (float2(id.xy) + 0.5) / dst;
  #define S(x, y) Tex0.SampleLevel(sLinear, uv + texel * float2(x, y), 0).rgb
  float3 a = S(-2, -2), b = S(0, -2), c = S(2, -2);
  float3 d = S(-2, 0), e = S(0, 0), f = S(2, 0);
  float3 g = S(-2, 2), h = S(0, 2), i = S(2, 2);
  float3 j = S(-1, -1), k = S(1, -1), l = S(-1, 1), m = S(1, 1);
  #undef S
  float3 r = Grp(j, k, l, m, karis) * 0.5
           + Grp(a, b, d, e, karis) * 0.125 + Grp(b, c, e, f, karis) * 0.125
           + Grp(d, e, g, h, karis) * 0.125 + Grp(e, f, h, i, karis) * 0.125;
  Out0[id.xy] = float4(r, 1);
}

// Upsample the coarser level with a 3x3 tent and add this level's downsample.
[numthreads(8, 8, 1)]
void BloomUpCS(uint3 id : SV_DispatchThreadID) {
  float2 dst = R.c0.xy;
  float2 texel = R.c0.zw;  // coarser level's texel size
  float radius = R.c1.x;
  if (id.x >= (uint)dst.x || id.y >= (uint)dst.y) return;
  float2 uv = (float2(id.xy) + 0.5) / dst;
  float2 o = texel * radius;
  float3 here = Tex1.SampleLevel(sLinear, uv, 0).rgb;
  if (R.c1.y < 0.5) { Out0[id.xy] = float4(here, 1); return; }  // the deepest level: nothing coarser
  float3 s = Tex0.SampleLevel(sLinear, uv, 0).rgb * 4.0;
  s += (Tex0.SampleLevel(sLinear, uv + float2(-o.x, 0), 0).rgb + Tex0.SampleLevel(sLinear, uv + float2(o.x, 0), 0).rgb
      + Tex0.SampleLevel(sLinear, uv + float2(0, -o.y), 0).rgb + Tex0.SampleLevel(sLinear, uv + float2(0, o.y), 0).rgb) * 2.0;
  s += Tex0.SampleLevel(sLinear, uv + float2(-o.x, -o.y), 0).rgb + Tex0.SampleLevel(sLinear, uv + float2(o.x, -o.y), 0).rgb
     + Tex0.SampleLevel(sLinear, uv + float2(-o.x, o.y), 0).rgb + Tex0.SampleLevel(sLinear, uv + float2(o.x, o.y), 0).rgb;
  Out0[id.xy] = float4(here + s / 16.0, 1);
}

// ACES filmic, Stephen Hill's fit of the RRT + ODT (BakingLab, MIT).
static const float3x3 ACESIn = { 0.59719, 0.35458, 0.04823, 0.07600, 0.90834, 0.01566, 0.02840, 0.13383, 0.83777 };
static const float3x3 ACESOut = { 1.60475, -0.53108, -0.07367, -0.10208, 1.10813, -0.00605, -0.00327, -0.07276, 1.07602 };
float3 ACES(float3 c) {
  c = mul(ACESIn, c);
  float3 a = c * (c + 0.0245786) - 0.000090537;
  float3 b = c * (0.983729 * c + 0.4329510) + 0.238081;
  return saturate(mul(ACESOut, a / b));
}

float3 ToSRGB(float3 c) {
  return lerp(c * 12.92, 1.055 * pow(c, 1.0 / 2.4) - 0.055, step(0.0031308, c));
}

float Hash(float2 p) { return frac(sin(dot(p, float2(12.9898, 78.233))) * 43758.5453); }

float4 TonemapPS(FsOut i) : SV_Target {
  float3 c = Tex0.Load(int3(i.pos.xy, 0)).rgb;
  float3 bloom = Tex1.SampleLevel(sLinear, i.uv, 0).rgb;
  c = lerp(c, bloom / 6.0, F.post.x);
  c *= F.fogP.w;
  c = ACES(c);
  float2 q = i.uv - 0.5;
  c *= 1.0 - F.post.y * dot(q, q) * 2.0;
  c = ToSRGB(saturate(c));
  // Triangular dither, one 8-bit step: the dark fog gradient must not band.
  float n = Hash(i.pos.xy) + Hash(i.pos.xy + 17.17) - 1.0;
  c += n / 255.0;
  return float4(c, 1);
}
