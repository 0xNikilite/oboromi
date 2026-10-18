#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
for shader in dungeon.vert dungeon.frag dungeon_cones.vert dungeon_sprite.vert dungeon_sprite.frag dungeon_particles.comp dungeon_fog.comp dungeon_post.comp; do
    glslangValidator -V --target-env vulkan1.2 "demos/shaders/$shader" -o "demos/shaders/$shader.spv"
    spirv-val --target-env vulkan1.2 "demos/shaders/$shader.spv"
done
glslangValidator -V --target-env vulkan1.2 -DRAY_QUERY demos/shaders/dungeon.frag -o demos/shaders/dungeon_rt.frag.spv
for shader in dungeon_water; do
    glslangValidator -V --target-env vulkan1.2 "demos/shaders/$shader.comp" -o "demos/shaders/$shader.spv"
done
spirv-val --target-env vulkan1.2 demos/shaders/dungeon_rt.frag.spv
