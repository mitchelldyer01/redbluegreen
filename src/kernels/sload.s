        .amdgcn_target "amdgcn-amd-amdhsa--gfx1151"
        // Bisect kernel: vadd's three scalar loads, no vector
        // load. kernarg: a u64 @0, b u64 @8, c u64 @16, n u32 @24.
        // Stores 42 to c[0] through s[8:9] (the 0x10 load) and
        // uses s10 (the 0x18 load) as an operand.
        .text
        .globl sload
        .p2align 8
        .type sload,@function
sload:
        s_load_b128 s[4:7], s[0:1], 0x0
        s_load_b64 s[8:9], s[0:1], 0x10
        s_load_b32 s10, s[0:1], 0x18
        s_waitcnt lgkmcnt(0)
        v_mov_b32 v0, 0
        v_mov_b32 v1, s10
        v_add_nc_u32 v1, 41, v1
        global_store_b32 v0, v1, s[8:9]
        s_waitcnt_vscnt null, 0x0
        s_endpgm
        .rodata
        .p2align 6
        .amdhsa_kernel sload
          .amdhsa_user_sgpr_kernarg_segment_ptr 1
          .amdhsa_next_free_vgpr 2
          .amdhsa_next_free_sgpr 12
          .amdhsa_wavefront_size32 1
        .end_amdhsa_kernel
