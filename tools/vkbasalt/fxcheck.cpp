// fxcheck -- compile a ReShade FX effect exactly the way vkBasalt does, offline.
//
//   fxcheck <effect.fx> <include-dir> [WIDTH HEIGHT] [out.spv]
//
// A shader vkBasalt cannot compile does not fail the game: vkBasalt logs the error and
// the effect is simply absent, which in game reads as "the setting did nothing" -- the
// same silent failure that cost several rounds on the DXVK side.  This runs the same
// preprocessor macros, the same parser and the same SPIR-V codegen flags as
// ReshadeEffect::createReshadeModule() (src/effect_reshade.cpp), linked against the
// very libreshade.a the layer was built from, so a pass here is a pass there.
//
// It prints the techniques, the textures the effect loads from disk (each must exist
// under reshadeTexturePath), and the spec constants -- the uniforms vkBasalt.conf can
// set BY NAME, and so the only ones worth putting in a config.
#include <climits>
#include <cstdio>
#include <fstream>
#include <memory>
#include <string>

#include "effect_codegen.hpp"
#include "effect_parser.hpp"
#include "effect_preprocessor.hpp"

static std::string annotation(const std::vector<reshadefx::annotation>& as, const char* name)
{
    for (const auto& a : as)
        if (a.name == name)
            return a.value.string_data;
    return "";
}

int main(int argc, char** argv)
{
    if (argc < 3)
    {
        std::fprintf(stderr, "usage: fxcheck <effect.fx> <include-dir> [WIDTH HEIGHT] [out.spv]\n");
        return 2;
    }
    const std::string w = argc > 4 ? argv[3] : "3440";
    const std::string h = argc > 4 ? argv[4] : "1440";

    reshadefx::preprocessor pp;
    pp.add_macro_definition("__RESHADE__", std::to_string(INT_MAX));
    pp.add_macro_definition("__RESHADE_PERFORMANCE_MODE__", "1");
    pp.add_macro_definition("__RENDERER__", "0x20000");
    pp.add_macro_definition("BUFFER_WIDTH", w);
    pp.add_macro_definition("BUFFER_HEIGHT", h);
    pp.add_macro_definition("BUFFER_RCP_WIDTH", "(1.0 / BUFFER_WIDTH)");
    pp.add_macro_definition("BUFFER_RCP_HEIGHT", "(1.0 / BUFFER_HEIGHT)");
    pp.add_macro_definition("BUFFER_COLOR_DEPTH", "8");
    pp.add_include_path(argv[2]);

    if (!pp.append_file(argv[1]))
    {
        std::printf("FAIL  cannot load %s\n%s", argv[1], pp.errors().c_str());
        return 1;
    }
    if (!pp.errors().empty())
        std::printf("preprocessor:\n%s", pp.errors().c_str());

    // Same four flags as vkBasalt: vulkan semantics, debug info, uniforms to spec
    // constants, flip vertex shader.
    std::unique_ptr<reshadefx::codegen> cg(reshadefx::create_codegen_spirv(true, true, true, true));
    reshadefx::parser parser;
    const bool ok = parser.parse(pp.output(), cg.get());
    if (!parser.errors().empty())
        std::printf("parser:\n%s", parser.errors().c_str());
    if (!ok)
    {
        std::printf("FAIL  %s does not compile\n", argv[1]);
        return 1;
    }

    reshadefx::module m;
    cg->write_result(m);

    for (const auto& t : m.techniques)
        std::printf("technique  %s (%zu passes)\n", t.name.c_str(), t.passes.size());
    for (const auto& t : m.textures)
    {
        const std::string src = annotation(t.annotations, "source");
        if (!src.empty())
            std::printf("loads      %s\n", src.c_str());
    }
    for (const auto& s : m.spec_constants)
        std::printf("settable   %s\n", s.name.c_str());

    if (argc > 5 || argc == 4)
    {
        const char* out = argc > 5 ? argv[5] : argv[3];
        std::ofstream f(out, std::ios::binary);
        f.write(reinterpret_cast<const char*>(m.spirv.data()), m.spirv.size() * sizeof(uint32_t));
    }
    std::printf("OK    %s -> %zu words of SPIR-V\n", argv[1], m.spirv.size());
    return 0;
}
