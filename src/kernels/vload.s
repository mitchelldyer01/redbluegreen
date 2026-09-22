        .amdgcn_target "amdgcn-amd-amdhsa--gfx1151"
        // Bisect kernel: one vector load. kernarg: p u64 @0.
        // *p = *p + 1. store42's shape plus a global_load.
        .text
        .globl vload
        .p2align 8
        .type vload,@function
vload:
        s_load_b64 s[2:3], s[0:1], 0x0
        s_waitcnt lgkmcnt(0)
        v_mov_b32 v0, 0
        global_load_b32 v1, v0, s[2:3]
        s_waitcnt vmcnt(0)
        v_add_nc_u32 v1, 1, v1
        global_store_b32 v0, v1, s[2:3]
        s_waitcnt_vscnt null, 0x0
        s_endpgm
        .rodata
        .p2align 6
        .amdhsa_kernel vload
          .amdhsa_user_sgpr_kernarg_segment_ptr 1
          .amdhsa_next_free_vgpr 2
          .amdhsa_next_free_sgpr 4
          .amdhsa_wavefront_size32 1
        .end_amdhsa_kernel
