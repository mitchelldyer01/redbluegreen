        .amdgcn_target "amdgcn-amd-amdhsa--gfx1151"
        // Bisect kernel: dump the registers vadd depends on.
        // kernarg as vadd. Stores to c (the 0x10 load) at index:
        // 0 s4, 1 s5, 2 s6, 3 s7, 4 s10 (n), 5 s2 (wg id),
        // 6 v0 (lane id), 7 the constant 42 as the done marker.
        .text
        .globl sdump
        .p2align 8
        .type sdump,@function
sdump:
        s_load_b128 s[4:7], s[0:1], 0x0
        s_load_b64 s[8:9], s[0:1], 0x10
        s_load_b32 s10, s[0:1], 0x18
        s_waitcnt lgkmcnt(0)
        v_mov_b32 v2, 0
        v_mov_b32 v1, s4
        global_store_b32 v2, v1, s[8:9]
        v_mov_b32 v1, s5
        global_store_b32 v2, v1, s[8:9] offset:4
        v_mov_b32 v1, s6
        global_store_b32 v2, v1, s[8:9] offset:8
        v_mov_b32 v1, s7
        global_store_b32 v2, v1, s[8:9] offset:12
        v_mov_b32 v1, s10
        global_store_b32 v2, v1, s[8:9] offset:16
        v_mov_b32 v1, s2
        global_store_b32 v2, v1, s[8:9] offset:20
        global_store_b32 v2, v0, s[8:9] offset:24
        v_mov_b32 v1, 42
        global_store_b32 v2, v1, s[8:9] offset:28
        s_waitcnt_vscnt null, 0x0
        s_endpgm
        .rodata
        .p2align 6
        .amdhsa_kernel sdump
          .amdhsa_user_sgpr_kernarg_segment_ptr 1
          .amdhsa_next_free_vgpr 3
          .amdhsa_next_free_sgpr 12
          .amdhsa_wavefront_size32 1
        .end_amdhsa_kernel
