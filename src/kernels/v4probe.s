        .amdgcn_target "amdgcn-amd-amdhsa--gfx1151"
        // Bisect kernel: store42 but the value travels through
        // v4, and a load lands in v4 first. kernarg: p u64 @0.
        .text
        .globl v4probe
        .p2align 8
        .type v4probe,@function
v4probe:
        s_load_b64 s[2:3], s[0:1], 0x0
        s_waitcnt lgkmcnt(0)
        v_mov_b32 v0, 0
        global_load_b32 v4, v0, s[2:3]
        s_waitcnt vmcnt(0)
        v_add_nc_u32 v4, 1, v4
        global_store_b32 v0, v4, s[2:3]
        s_waitcnt_vscnt null, 0x0
        s_endpgm
        .rodata
        .p2align 6
        .amdhsa_kernel v4probe
          .amdhsa_user_sgpr_kernarg_segment_ptr 1
          .amdhsa_next_free_vgpr 5
          .amdhsa_next_free_sgpr 4
          .amdhsa_wavefront_size32 1
        .end_amdhsa_kernel
