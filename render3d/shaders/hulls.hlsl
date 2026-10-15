// entries: vs_hull:vs_3_0:VS_HULL ps_hull:ps_3_0:PS_HULL ps_bump:ps_3_0:PS_BUMP ps_cloak:ps_3_0:PS_CLOAK ps_ghost:ps_3_0:PS_GHOST ps_holo:ps_3_0:PS_HOLO vs_depth:vs_3_0:VS_DEPTH ps_depth:ps_3_0:PS_DEPTH vs_sprite:vs_3_0:VS_SPRITE ps_sprite:ps_3_0:PS_SPRITE
// Hulls: ships, stations and the like, drawn from their SODs by render3d's hulls module
// (src/mods/hulls/). Lighting.asi's hull look ported (lighting/README.md, "Shaders",
// "Specular and rim", "Dynamic range", "Bump-mapped hulls", "The Borg profile",
// "Shadows", "Cloaked ships"), drawn into render3d's half-float target. The lit hull
// keeps Lighting.asi's highlight shoulder (`shoulder`): it stays within 1, as it did
// when Lighting.asi drew it into the same target, so bloom and the frame's tone map
// treat it alike.
//
// Normals are outward here (the scene layer builds them so), where the engine's point
// inward; every light direction below is "towards the light".

// ---- vertex ------------------------------------------------------------------------

float4x4 W  : register(c0);    // world (transposed)
float4x4 VP : register(c4);    // view x projection (transposed)
float4 warp_n  : register(c8);  // the warp stretch: x,y,z the motion's direction (world), w: the plane's distance along it
float4 warp_k  : register(c9);  // x: S - 1, y: the shift o (world units), z: 1 while warping; w: the normals' sign (scene::NORMAL_SIGN)
float4 smat[3] : register(c10); // world -> shadow map u, v, depth (rows)
float4 soff    : register(c13); // x: the normal offset for the shadow lookup (world units, a texel)
float4 skey    : register(c14); // towards the Key, world, unit
float4 wave_d  : register(c15); // a wave light: x,y,z its direction (mesh space), w: 2 pi / wavelength
float4 wave_t  : register(c16); // x: its phase now (time x speed x 2 pi), y: 1 when there is one

struct VI {
    float3 pos : POSITION;
    float3 nrm : NORMAL;
    float2 uv  : TEXCOORD0;
};

struct VO {
    float4 pos : POSITION;
    float2 uv  : TEXCOORD0;
    float3 n   : TEXCOORD1;   // world
    float3 wp  : TEXCOORD2;   // world position
    float3 sc  : TEXCOORD3;   // in the shadow map: u, v, depth
    float  wv  : TEXCOORD4;   // the wave light's share at this vertex (-0.5 .. 0.5)
};

// The warp stretch (spec-chunks-anim-lod.md, section 5): the front of the hull beyond a
// plane across the motion is stretched by S along it, the whole shifted back by o.
float3 warp(float3 p)
{
    float h = dot(p, warp_n.xyz) - warp_n.w;
    float m = h < 0.0 ? -warp_k.y : h * warp_k.x - warp_k.y;
    return p + warp_n.xyz * m * warp_k.z;
}

float3 shadow_coord(float3 wp, float3 n)
{
    float ndl = dot(n, skey.xyz);
    float off = soff.x * (0.5 + 1.5 * sqrt(saturate(1.0 - ndl * ndl)));
    float4 q = float4(wp + n * off, 1.0);
    return float3(dot(q, smat[0]), dot(q, smat[1]), dot(q, smat[2]));
}

VO vs_hull(VI i)
{
    VO o;
    float3 w = warp(mul(float4(i.pos, 1.0), W).xyz);
    o.pos = mul(float4(w, 1.0), VP);
    o.wp = w;
    float3 n = mul(float4(i.nrm, 0.0), W).xyz * warp_k.w;
    float l = length(n);
    o.n = l > 1e-8 ? n / l : float3(0.0, 1.0, 0.0);
    o.uv = i.uv;
    o.sc = shadow_coord(w, o.n);
    o.wv = wave_t.y * 0.5 * sin(dot(i.pos, wave_d.xyz) * wave_d.w + wave_t.x);
    return o;
}

// ---- pixel -------------------------------------------------------------------------

#define DIRS   4
#define POINTS 16
#define PLANETS 8
float4 k_amb   : register(c0);   // ambient rgb; a: self-illumination (0: none; above 1 brighter than the texture)
float4 k_tint  : register(c1);   // the diffuse colour (team colour on team groups, else white); a: unlit (1) or lit (0)
float4 k_misc  : register(c2);   // x: fade (alpha out); y: specular strength; z: its exponent; w: alpha out = texture alpha (1) or the fade (0)
float4 k_rim   : register(c3);   // rim light colour; a: its exponent
float4 k_eye   : register(c4);   // the camera, world
float4 k_shad  : register(c5);   // x: 1 with a shadow map; y: one texel (uv); z: depth bias; w: how much of the Key a shadow takes
float4 k_uv    : register(c6);   // xy: the flipbook's UV offset; z: the wave light's strength; w: 1 to cut out at alpha 0.5
float4 k_wave  : register(c7);   // the wave light's colour
float4 k_dvec[DIRS] : register(c8);   // towards each directional light
float4 k_dcol[DIRS] : register(c12);  // its colour; a: 1 for the Key (the light shadows take away)
float4 k_ppos[POINTS] : register(c16); // point light position; w: full to this distance
float4 k_pcol[POINTS] : register(c32); // colour; w: 1 / the distance it fades over
float4 k_pfall[POINTS] : register(c48); // x: wrap, y: power
float4 k_planet[PLANETS] : register(c64); // a planet: centre, radius (0: none)
float4 k_sun   : register(c72);  // towards the Key; w: how much planets shadow
float4 k_bump  : register(c73);  // x: 1 with a normal map; y: its strength; z: the highlight knee

sampler2D tex  : register(s0);
sampler2D nmap : register(s1);   // a bump-mapped mesh's normal map (from its height map)
sampler2D smap : register(s3);   // the shadow map: depth 0..1 from the Key, 1 where nothing is

float glint(float3 N, float3 L, float3 V)
{
    float3 H = normalize(L + V);
    return dot(N, L) > 0.0 ? pow(max(0.0, dot(N, H)), k_misc.z) : 0.0;
}

// How much of the Key reaches a point: 3x3 texels weighted by where it falls (16 taps).
float map_shadow(float3 sc)
{
    if (k_shad.x <= 0.0 || sc.x <= 0.0 || sc.x >= 1.0 || sc.y <= 0.0 || sc.y >= 1.0) return 1.0;
    float2 t = sc.xy / k_shad.y - 0.5;
    float2 f = frac(t);
    float2 b = (t - f + 0.5) * k_shad.y;
    float d = sc.z - k_shad.z;
    float lit = 0.0;
    for (int y = -1; y <= 2; y++) {
        float wy = y == -1 ? 1.0 - f.y : (y == 2 ? f.y : 1.0);
        for (int x = -1; x <= 2; x++) {
            float wx = x == -1 ? 1.0 - f.x : (x == 2 ? f.x : 1.0);
            float z = tex2Dlod(smap, float4(b + float2(x, y) * k_shad.y, 0.0, 0.0)).r;
            lit += (d <= z ? 1.0 : 0.0) * wx * wy;
        }
    }
    return lit / 9.0;
}

float planet_shadow(float3 wp)
{
    float s = 1.0;
    for (int j = 0; j < PLANETS; j++) {
        float r = k_planet[j].w;
        float3 c = k_planet[j].xyz - wp;
        float t = dot(c, k_sun.xyz);
        float d = length(c - k_sun.xyz * t);
        float w = r * 0.02 + max(t, 0.0) * 0.01;
        s *= r > 0.0 && t > 0.0 ? smoothstep(r - w, r + w, d) : 1.0;
    }
    return lerp(1.0, s, k_sun.w);
}

// Lighting.asi's highlight shoulder: the colour as it is up to the knee k, then rolling
// off towards 1 with the same slope at the knee, per channel (k >= 1: a plain clip; k
// above 1.5: none). The lit hull stays within 1 as it did under Lighting.asi, so a
// point light far past 1 whitens the plating it falls on without blooming over the
// hull; the frame's tone map is applied on top, as it was to Lighting.asi's output.
float3 shoulder(float3 x, float k)
{
    float  r = max(1.0 - k, 1e-4);
    float3 y = k + r * (1.0 - exp(-(x - k) / r));
    return k > 1.5 ? max(x, 0.0) : (k >= 1.0 ? saturate(x) : lerp(max(x, 0.0), y, step(k, x)));
}

// Everything after the normal: the lights, the texture t, the night lights.
float4 shade(float3 N, float3 wp, float4 t, float3 sc, float wv)
{
    float3 V = normalize(k_eye.xyz - wp);
    float3 sum = k_amb.rgb, spec = 0.0;
    float key = lerp(1.0, map_shadow(sc), k_shad.w) * planet_shadow(wp);
    for (int k = 0; k < DIRS; k++) {
        float3 c = k_dcol[k].rgb * lerp(1.0, key, k_dcol[k].a);
        sum += c * max(0.0, dot(N, k_dvec[k].xyz));
        spec += c * glint(N, k_dvec[k].xyz, V);
    }
    for (int j = 0; j < POINTS; j++) {
        float3 L = k_ppos[j].xyz - wp;
        float d = length(L);
        float f = pow(saturate(1.0 - max(0.0, d - k_ppos[j].w) * k_pcol[j].w), k_pfall[j].y);
        L /= max(d, 1e-6);
        sum += k_pcol[j].rgb * saturate((dot(N, L) + k_pfall[j].x) / (1.0 + k_pfall[j].x)) * f;
        spec += k_pcol[j].rgb * glint(N, L, V) * f;
    }
    sum += k_wave.rgb * wv * k_uv.z;
    sum += k_rim.rgb * pow(1.0 - saturate(dot(N, V)), k_rim.a);
    float gloss = dot(t.rgb, float3(0.299, 0.587, 0.114));
    float3 lit = t.rgb * k_tint.rgb * max(sum, 0.0) + spec * k_misc.y * gloss;
    float3 c = shoulder(lerp(lit, t.rgb * max(k_amb.a, 1.0), saturate(t.a * min(k_amb.a, 1.0))), k_bump.z);
    c = lerp(c, t.rgb * k_tint.rgb, k_tint.a);
    return float4(c, lerp(k_misc.x, t.a * k_misc.x, k_misc.w));
}

float4 sample_tex(float2 uv)
{
    float4 t = tex2D(tex, uv + k_uv.xy);
    clip(k_uv.w > 0.5 ? t.a - 0.5 : 1.0);
    return t;
}

float4 ps_hull(VO i) : COLOR
{
    return shade(normalize(i.n), i.wp, sample_tex(i.uv), i.sc, i.wv);
}

// The normal from a normal map, in a tangent frame worked out per pixel from how the
// position and the texture coordinate change across the screen (no tangents stored).
float3 bumped(VO i, float3 N)
{
    float3 dp1 = ddx(i.wp), dp2 = ddy(i.wp);
    float2 du1 = ddx(i.uv), du2 = ddy(i.uv);
    float3 a = cross(dp2, N), b = cross(N, dp1);
    float3 T = a * du1.x + b * du2.x;
    float3 B = a * du1.y + b * du2.y;
    float s = rsqrt(max(max(dot(T, T), dot(B, B)), 1e-20));
    float3 m = tex2D(nmap, i.uv + k_uv.xy).rgb * 2.0 - 1.0;
    m.xy *= k_bump.y;
    // the map's y runs down the texture, as the engine's normal map is built
    float3 n = T * s * m.x - B * s * m.y + N * m.z;
    float l = length(n);
    return l > 1e-5 ? n / l : N;
}

float4 ps_bump(VO i) : COLOR
{
    float3 N = normalize(i.n);
    return shade(bumped(i, N), i.wp, sample_tex(i.uv), i.sc, i.wv);
}

// ---- cloaked hulls: thick glass over a copy of the frame (Lighting.asi's Cloak=1) ----
float4 ck_view[3] : register(c80);  // rows of VIEW's 3x3 (world -> view x, y, z)
float4 ck_proj    : register(c83);  // x, y: the projection's scale over the copy's size; z, w: 1 / the copy's size
float4 ck_ctr     : register(c84);  // the hull's bounding sphere: centre; w: radius
float4 ck_a       : register(c85);  // x: how much of the hull is left; y: time; z: the bend; w: the colour split
float4 ck_b       : register(c86);  // x: ripple; y: rim; z: glint; w: the hull's floor
float4 ck_tint    : register(c87);  // rgb: what the glass passes; a: the front's glow
float4 ck_rim     : register(c88);
float4 ck_edge    : register(c89);
float4 ck_org     : register(c90);  // xy: the copy's top left in the target, pixels
sampler2D ck_scene : register(s6);

float3 ck_scene_at(float2 uv, float2 off, float split)
{
    float3 c = 0.0;
    for (int k = -1; k <= 1; k++) {
        float s = 1.0 + split * 0.35 * k;
        c.r += tex2Dlod(ck_scene, float4(uv + off * (s + split), 0.0, 0.0)).r;
        c.g += tex2Dlod(ck_scene, float4(uv + off * s, 0.0, 0.0)).g;
        c.b += tex2Dlod(ck_scene, float4(uv + off * (s - split), 0.0, 0.0)).b;
    }
    return c / 3.0;
}

float4 ps_cloak(VO i, float2 vpos : VPOS) : COLOR
{
    float3 N = normalize(i.n);
    float4 hull = shade(N, i.wp, sample_tex(i.uv), i.sc, i.wv);
    float3 V = normalize(k_eye.xyz - i.wp);
    float3 q = (i.wp - ck_ctr.xyz) / max(ck_ctr.w, 1e-3);
    float vis = ck_a.x, t = ck_a.y;
    float amp = ck_b.x * (0.4 + 2.4 * vis * (1.0 - vis));
    float3 wob = float3(sin(q.y * 5.3 + t * 1.10), sin(q.z * 4.7 + t * 0.90 + 1.7), sin(q.x * 6.1 - t * 1.30 + 3.1));
    float3 Ng = normalize(N + wob * amp);
    float3 nv = float3(dot(Ng, ck_view[0].xyz), dot(Ng, ck_view[1].xyz), dot(Ng, ck_view[2].xyz));
    float z = max(dot(i.wp - k_eye.xyz, ck_view[2].xyz), 1.0);
    float ndv = saturate(dot(Ng, V));
    float w = ck_a.z * ck_ctr.w * (0.5 + 1.0 * sqrt(saturate(1.0 - ndv * ndv)));
    float2 off = float2(nv.x, -nv.y) * w * ck_proj.xy / z;
    float2 uv = (vpos - ck_org.xy + 0.5) * ck_proj.zw;
    float3 c = ck_scene_at(uv, off, ck_a.w) * ck_tint.rgb;
    float f = pow(1.0 - ndv, 4.0);
    c += ck_rim.rgb * f * ck_b.y;
    c += pow(saturate(dot(Ng, normalize(k_sun.xyz + V))), 90.0) * ck_b.z;
    float m = 0.5 + 0.30 * sin(q.x * 3.1 + q.y * 2.3 + 0.7) * sin(q.z * 2.7 - q.x * 1.3) + 0.15 * sin(q.y * 7.1 - q.z * 5.3 + 2.0);
    float e = vis * 1.4 - 0.2;
    float sh = smoothstep(m - 0.12, m + 0.12, e);
    float edge = (1.0 - saturate(abs(e - m) / 0.12)) * step(0.001, vis) * step(vis, 0.999);
    float keep = lerp(ck_b.w, 1.0, sh);
    c = lerp(c, hull.rgb, keep) + ck_edge.rgb * edge * ck_tint.a;
    return float4(c, 1.0);
}

// ---- the build ghost: the station being placed, faded --------------------------------
// Stock draws it as a faded hull through its sorted translucent path (half the fade),
// which comes out pale and washed towards white with a hot sheen where it faces the
// light. Here: the lit hull lifted towards its texture at full light, a broad highlight
// towards each directional light past white (so it blooms), and a blocked placement's
// red added as stock adds its red ambient. Premultiplied (ONE, INVSRCALPHA).
float4 gh_a   : register(c80); // x: alpha; y: lift; z: sheen; w: the sheen's exponent
float4 gh_add : register(c81); // rgb: light added on the texture (a blocked placement's red)

float lum(float3 c)
{
    return dot(c, float3(0.299, 0.587, 0.114));
}

float4 ps_ghost(VO i) : COLOR
{
    float3 N = normalize(i.n);
    float4 t = sample_tex(i.uv);
    float3 c = shade(N, i.wp, t, i.sc, i.wv).rgb;
    float3 V = normalize(k_eye.xyz - i.wp);
    float sheen = 0.0;
    for (int k = 0; k < DIRS; k++) {
        float3 H = normalize(k_dvec[k].xyz + V);
        sheen += lum(k_dcol[k].rgb) * pow(saturate(dot(N, H)), gh_a.w);
    }
    c = lerp(c, max(c, t.rgb * k_tint.rgb), gh_a.y) + t.rgb * gh_add.rgb;
    c += sheen * gh_a.z * (0.5 + 0.5 * lum(t.rgb));
    return float4(c * gh_a.x, gh_a.x);
}

// ---- holograms: a craft shown as another class (TimedHoloEffect) ---------------------
// The lit hull of the class it shows, drawn as a hologram in the object's team colour:
// scanlines rolling down the screen, a bright band sweeping along the hull, a stepped
// flicker with rare dropouts, and a glow at the silhouette past white. Premultiplied.
float4 ho_a    : register(c82); // x: alpha; y: scanline depth; z: scanline pitch (pixels); w: time (s)
float4 ho_b    : register(c83); // x: flicker; y: edge glow; z: its exponent; w: this hologram's seed
float4 ho_tint : register(c84); // rgb: the hologram's colour; w: the sweep's strength
float4 ho_ax   : register(c85); // xyz: the hull's up axis (world, unit); w: 1 / its radius
float4 ho_ctr  : register(c86); // xyz: its centre; w: how far the hull goes towards the colour
float4 ho_c    : register(c87); // x: the scanlines' roll (pitches per second); y: the sweeps per second; z: dropout chance

float hash1(float x)
{
    return frac(sin(x * 12.9898 + 78.233) * 43758.5453);
}

float4 ps_holo(VO i, float2 vpos : VPOS) : COLOR
{
    float3 N = normalize(i.n);
    float4 t = sample_tex(i.uv);
    float3 c = shade(N, i.wp, t, i.sc, i.wv).rgb;
    float3 V = normalize(k_eye.xyz - i.wp);
    float time = ho_a.w, seed = ho_b.w;
    float s = 0.5 + 0.5 * cos(6.2831853 * (vpos.y / ho_a.z - time * ho_c.x));
    float scan = 1.0 - ho_a.y * s;
    float h = dot(i.wp - ho_ctr.xyz, ho_ax.xyz) * ho_ax.w;
    float at = frac(time * ho_c.y + seed) * 3.0 - 1.5;
    float band = exp(-(h - at) * (h - at) * 30.0) * ho_tint.w;
    float flick = 1.0 - ho_b.x * hash1(floor(time * 15.0) + seed * 97.0);
    flick *= 1.0 - 0.6 * step(1.0 - ho_c.z, hash1(floor(time * 6.0) + seed * 13.0));
    float edge = pow(1.0 - saturate(dot(N, V)), ho_b.z);
    float3 tint = ho_tint.rgb;
    float3 base = lerp(c, lum(c) * tint * 1.6, ho_ctr.w);
    float a = saturate(ho_a.x * scan * flick);
    float3 glow = tint * (edge * ho_b.y + band) * flick;
    return float4(base * a + glow, a);
}

// ---- the shadow map: depth from the Key ---------------------------------------------

float4 k_spass : register(c17); // vertex: x one texel

struct DO {
    float4 pos : POSITION;
    float  d   : TEXCOORD0;
};

DO vs_depth(VI i)
{
    DO o;
    float3 w = warp(mul(float4(i.pos, 1.0), W).xyz);
    float4 q = float4(w, 1.0);
    float3 s = float3(dot(q, smat[0]), dot(q, smat[1]), dot(q, smat[2]));
    o.pos = float4(s.x * 2.0 - 1.0 - k_spass.x, 1.0 - s.y * 2.0 + k_spass.x, s.z, 1.0);
    o.d = s.z;
    return o;
}

float4 ps_depth(DO i) : COLOR
{
    return float4(i.d, 0.0, 0.0, 1.0);
}

// ---- sprites: SOD sprite nodes (running lights) and billboard effects ----------------
// One quad per sprite, its corners already where they go (world), the colour per vertex.

struct SI {
    float3 pos : POSITION;
    float4 col : COLOR0;
    float2 uv  : TEXCOORD0;
};

struct SO {
    float4 pos : POSITION;
    float4 col : COLOR0;
    float2 uv  : TEXCOORD0;
};

SO vs_sprite(SI i)
{
    SO o;
    o.pos = mul(float4(i.pos, 1.0), VP);
    o.col = i.col;
    o.uv = i.uv;
    return o;
}

float4 k_sprite : register(c0); // x: colour scale (HDR); y: 1 to cut out at alpha 0.5

float4 ps_sprite(SO i) : COLOR
{
    float4 t = tex2D(tex, i.uv);
    clip(k_sprite.y > 0.5 ? t.a - 0.5 : 1.0);
    return float4(t.rgb * i.col.rgb * k_sprite.x, t.a * i.col.a);
}
