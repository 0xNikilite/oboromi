# Dungeon materials

The dungeon uses ambientCG's 2K JPG PBR texture sets, licensed under
[CC0 1.0](https://docs.ambientcg.com/license):

- [PavingStones142](https://ambientcg.com/view?id=PavingStones142): floor colour, OpenGL normal, roughness, displacement and ambient occlusion.
- [Rock035](https://ambientcg.com/view?id=Rock035): walls and cave formations, with the same maps.
- [Metal055C](https://ambientcg.com/view?id=Metal055C): rusty chains and railings, including metalness; this set has no AO map, so AO is neutral.

`scripts/dungeon_materials.py ZIP_DIRECTORY demos/assets/dungeon` packs the maps
into RGBA8 block-linear mip chains. The surface map stores AO, roughness,
metalness and height in RGBA order. Source archives are downloaded from
`https://ambientcg.com/get?file=ASSET_2K-JPG.zip`.
