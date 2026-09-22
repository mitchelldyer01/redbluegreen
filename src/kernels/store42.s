        .amdgcn_target "amdgcn-amd-amdhsa--gfx1151"
        .text
        .globl store42
        .p2align 8
        .type store42,@function
store42:
        s_load_b64 s[2:3], s[0:1], 0x0
        s_waitcnt lgkmcnt(0)
        v_mov_b32 v0, 0
        v_mov_b32 v1, 0x2a
        global_store_b32 v0, v1, s[2:3]
        s_waitcnt_vscnt null, 0x0
        s_endpgm
        .rodata
        .p2align 6
        .amdhsa_kernel store42
          .amdhsa_user_sgpr_kernarg_segment_ptr 1
          .amdhsa_next_free_vgpr 2
          .amdhsa_next_free_sgpr 4
          .amdhsa_wavefront_size32 1
        .end_amdhsa_kernel
