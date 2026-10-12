/*
 * surfaces.h -- what Surfaces.asi gives Lighting.asi for a planet (surfaces/README.md,
 * "The contract with Lighting.asi"). Included by both; change it in both at once, and
 * bump the layout's size check if a field is added.
 *
 *   int a2_surface(void *dev8, const char *name, A2Surface *out);   (cdecl, exported)
 *
 * dev8: the game's IDirect3DDevice8. name: the planet class's groundTextureName (for a
 * ground hemisphere) or atmosphereTextureName (for the shell), as the ODF gives it.
 * Returns 1 and fills *out when the name has a recipe and its cubes are baked (the
 * first call for a recipe bakes it, inside a state block of its own, and may take a
 * few hundred milliseconds). Otherwise the stock texture is drawn: 0 when the name has
 * no recipe (a caller may remember that), -1 when it has one that cannot be drawn now
 * (Enable=0, no Direct3D 9 device, a failed bake). out->size must be sizeof(A2Surface).
 * The name may be any texture's: a model drawn through Lighting.asi's hull shaders
 * (the dilithium moons) is asked for by its texture's name.
 */
#ifndef A2_SURFACES_H
#define A2_SURFACES_H

typedef struct {
    int    size;          /* sizeof(A2Surface), set by the caller */
    void  *albedo;        /* IDirect3DCubeTexture9: rgb ground colour, a wet */
    void  *relief;        /* IDirect3DCubeTexture9: rgb normal - direction + 0.5, a glow */
    void  *cloud;         /* IDirect3DCubeTexture9: cloud density, or NULL for none */
    /* The draw's constants, planet.hlsl's c43..c46:
     *   k[0] rgb: the glow's colour; a: how much of it shows by day
     *   k[1] rgb: the clouds' colour; a: their opacity (0: no clouds)
     *   k[2] x, y: cos and sin of the clouds' drift about the pole; z: the strength of
     *        their shadow; w: how far it falls from them, towards the sun (radians)
     *   k[3] x: the relief's strength in the light (1: as baked); y: 1 with clouds;
     *        z: 1 when the albedo's alpha is a cut-out (Cut=), else it is wetness */
    float  k[4][4];
} A2Surface;

typedef int (__cdecl *A2SurfaceFn)(void *dev8, const char *name, A2Surface *out);

#endif
