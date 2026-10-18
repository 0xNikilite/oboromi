#include "compiler/glsl_types.h"
#include "compiler/nir/nir.h"
#include "compiler/nir/nir_builder.h"
#include "compiler/spirv/nir_spirv.h"
#include "nouveau/compiler/nak.h"
#include "nouveau/headers/nv_device_info.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static void die(const char *message)
{
   fprintf(stderr, "%s\n", message);
   exit(1);
}

struct binding {
   char kind;
   uint32_t offset;
};

struct bindings {
   struct binding binding[32];
};

static uint32_t align_u32(uint32_t value, uint32_t alignment)
{
   return (value + alignment - 1) & ~(alignment - 1);
}

static void parse_bindings(char *text, struct bindings *bindings)
{
   uint32_t offset = 0;
   for (char *item = strtok(text, ","); item; item = strtok(NULL, ",")) {
      unsigned binding = strtoul(item + 1, NULL, 10);
      if (binding >= 32 || !strchr("usba", item[0]))
         die("invalid BINDINGS");
      bindings->binding[binding].kind = item[0];
      if (item[0] == 's')
         bindings->binding[binding].offset = offset, offset += 4;
   }
   offset = align_u32(offset, 8);
   for (unsigned i = 0; i < 32; i++)
      if (bindings->binding[i].kind == 'a')
         bindings->binding[i].offset = offset, offset += 16;
   offset = align_u32(offset, 8);
   for (unsigned i = 0; i < 32; i++)
      if (bindings->binding[i].kind == 'u')
         bindings->binding[i].offset = offset;
   offset = align_u32(offset + 8, 16);
   for (unsigned i = 0; i < 32; i++)
      if (bindings->binding[i].kind == 'b')
         bindings->binding[i].offset = offset, offset += 16;
}

static nir_def *load_binding(nir_builder *b, const struct binding *binding,
                             unsigned components)
{
   return nir_ldc_nv(b, components, 32, nir_imm_int(b, 0x1d),
                     nir_imm_int(b, binding->offset),
                     .align_mul = components * 4, .align_offset = 0);
}

static unsigned deref_binding(nir_deref_instr *deref)
{
   while (deref->deref_type != nir_deref_type_var)
      deref = nir_deref_instr_parent(deref);
   if (deref->var->data.descriptor_set != 0)
      die("only descriptor set 0 is supported");
   return deref->var->data.binding;
}

static bool lower_descriptor(nir_builder *b, nir_instr *instr, void *data)
{
   const struct bindings *bindings = data;
   if (instr->type == nir_instr_type_tex) {
      nir_tex_instr *tex = nir_instr_as_tex(instr);
      nir_deref_instr *texture =
         nir_steal_tex_deref(tex, nir_tex_src_texture_deref);
      nir_deref_instr *sampler =
         nir_steal_tex_deref(tex, nir_tex_src_sampler_deref);
      if (!texture)
         return false;
      unsigned index = deref_binding(texture);
      if (sampler && deref_binding(sampler) != index)
         die("separate texture and sampler bindings are unsupported");
      if (bindings->binding[index].kind != 's')
         die("texture binding missing from BINDINGS");
      b->cursor = nir_before_instr(instr);
      nir_tex_instr_add_src(tex, nir_tex_src_texture_handle,
                            load_binding(b, &bindings->binding[index], 1));
      return true;
   }
   if (instr->type != nir_instr_type_intrinsic)
      return false;
   nir_intrinsic_instr *load = nir_instr_as_intrinsic(instr);
   if (load->intrinsic == nir_intrinsic_load_base_instance) {
      b->cursor = nir_before_instr(instr);
      nir_def_rewrite_uses(&load->def,
         nir_ldc_nv(b, 1, 32, nir_imm_int(b, 0), nir_imm_int(b, 4),
                    .align_mul = 4, .align_offset = 0));
      return true;
   }
   if (load->intrinsic != nir_intrinsic_load_vulkan_descriptor)
      return false;
   nir_intrinsic_instr *index = nir_src_as_intrinsic(load->src[0]);
   if (!index || index->intrinsic != nir_intrinsic_vulkan_resource_index ||
       nir_intrinsic_desc_set(index) != 0)
      die("unsupported descriptor index");
   const struct binding *binding =
      &bindings->binding[nir_intrinsic_binding(index)];
   b->cursor = nir_before_instr(instr);
   nir_def *desc;
   if (binding->kind == 'u') {
      nir_def *handle = load_binding(b, binding, 2);
      desc = nir_vec3(b, nir_channel(b, handle, 0),
                         nir_channel(b, handle, 1), nir_imm_int(b, 0));
   } else if (binding->kind == 'b') {
      desc = load_binding(b, binding, 4);
   } else if (binding->kind == 'a') {
      desc = load_binding(b, binding, load->def.num_components);
   } else {
      die("buffer binding missing from BINDINGS");
   }
   nir_def_rewrite_uses(&load->def, desc);
   return true;
}

static bool lower_load(nir_builder *b, nir_intrinsic_instr *load,
                       void *data)
{
   if (load->intrinsic != nir_intrinsic_load_ubo)
      return false;
   b->cursor = nir_before_instr(&load->instr);
   nir_def *handle = nir_pack_64_2x32(b, load->src[0].ssa);
   nir_def *value = nir_ldcx_nv(b, load->num_components, load->def.bit_size,
                                handle, load->src[1].ssa,
                                .access = nir_intrinsic_access(load),
                                .align_mul = nir_intrinsic_align_mul(load),
                                .align_offset = nir_intrinsic_align_offset(load));
   nir_def_rewrite_uses(&load->def, value);
   return true;
}

int main(int argc, char **argv)
{
   if (argc != 5)
      die("usage: nak-offline <vert|frag> <input.spv> <output.bin> <bindings>");

   gl_shader_stage stage;
   if (!strcmp(argv[1], "vert"))
      stage = MESA_SHADER_VERTEX;
   else if (!strcmp(argv[1], "frag"))
      stage = MESA_SHADER_FRAGMENT;
   else
      die("stage must be vert or frag");

   FILE *input = fopen(argv[2], "rb");
   if (!input || fseek(input, 0, SEEK_END))
      die("failed to open input");
   const long size = ftell(input);
   if (size <= 0 || size % 4 || fseek(input, 0, SEEK_SET))
      die("invalid SPIR-V size");
   uint32_t *words = malloc(size);
   if (!words || fread(words, size, 1, input) != 1)
      die("failed to read input");
   fclose(input);

   const struct nv_device_info dev = {
      .sm = 86,
      .max_warps_per_mp = 48,
   };
   struct nak_compiler *nak = nak_compiler_create(&dev);
   const struct spirv_to_nir_options spirv_options = {
      .environment = NIR_SPIRV_VULKAN,
      .ssbo_addr_format = nir_address_format_64bit_bounded_global,
      .ubo_addr_format = nir_address_format_vec2_index_32bit_offset,
   };
   glsl_type_singleton_init_or_ref();
   nir_shader *nir = spirv_to_nir(words, size / 4, NULL, 0, stage, "main",
                                  &spirv_options, nak_nir_options(nak));
   free(words);
   if (!nir)
      die("SPIR-V to NIR failed");
   nir_lower_variable_initializers(nir, nir_var_function_temp);
   nir_lower_returns(nir);
   nir_inline_functions(nir);
   nir_copy_prop(nir);
   nir_opt_deref(nir);
   nir_remove_non_entrypoints(nir);
   nir_lower_variable_initializers(nir, ~0);
   nak_preprocess_nir(nir, nak);
   struct bindings bindings = {0};
   parse_bindings(argv[4], &bindings);
   nir_shader_instructions_pass(nir, lower_descriptor,
                                nir_metadata_control_flow, &bindings);
   nir_opt_dce(nir);
   nir_lower_explicit_io(nir, nir_var_mem_ssbo,
                         nir_address_format_64bit_bounded_global);
   nir_lower_explicit_io(nir, nir_var_mem_ubo,
                         nir_address_format_vec2_index_32bit_offset);
   nir_shader_intrinsics_pass(nir, lower_load, nir_metadata_none, NULL);
   nir_opt_dce(nir);
   struct nak_shader_bin *bin = nak_compile_shader(nir, true, nak, 0, NULL);
   if (!bin)
      die("NAK compilation failed");

   FILE *output = fopen(argv[3], "wb");
   if (!output || fwrite(bin->info.hdr, sizeof(bin->info.hdr), 1, output) != 1 ||
       fwrite(bin->code, bin->code_size, 1, output) != 1 || fclose(output))
      die("failed to write output");
   fprintf(stderr, "SM%u: %u instructions, %u GPRs, %u code bytes\n",
           bin->info.sm, bin->info.num_instrs, bin->info.num_gprs,
           bin->code_size);
   if (bin->asm_str)
      fputs(bin->asm_str, stderr);

   nak_shader_bin_destroy(bin);
   ralloc_free(nir);
   nak_compiler_destroy(nak);
   glsl_type_singleton_decref();
   return 0;
}
