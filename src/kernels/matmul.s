.amdgcn_target "amdgcn-amd-amdhsa--gfx1151"
        // matmul: C = A x B, one work-item per element of C.
        // col = workgroup_id_x * 256 + the lane, row =
        // workgroup_id_y. kernarg: a u64 @0, b u64 @8, c u64
        // @16, m u32 @24, n u32 @28, k u32 @32.
        .text
        .globl matmul
        .p2align 8
        .type matmul,@function
matmul:
        s_load_b128 s[4:7], s[0:1], 0x0
        s_load_b64 s[8:9], s[0:1], 0x10
        s_load_b32 s10, s[0:1], 0x1c
        s_load_b32 s11, s[0:1], 0x20
        v_lshl_add_u32 v0, s2, 8, v0
        s_waitcnt lgkmcnt(0)
        v_cmp_gt_u32 vcc_lo, s10, v0
        s_and_saveexec_b32 s12, vcc_lo
        s_cbranch_execz done
        v_mov_b32 v1, s3
        v_lshlrev_b32 v2, 2, v0
        v_mul_lo_u32 v3, v1, s11
        v_lshlrev_b32 v3, 2, v3
        v_mov_b32 v4, 0
        v_mov_b32 v6, 0
loop:
        v_lshlrev_b32 v5, 2, v6
        v_add_u32 v5, v5, v3
        v_mul_lo_u32 v7, v6, s10
        v_lshlrev_b32 v7, 2, v7
        v_add_u32 v7, v7, v2
        global_load_b32 v8, v5, s[4:5]
        global_load_b32 v9, v7, s[6:7]
        s_waitcnt vmcnt(0)
        v_mul_f32 v8, v8, v9
        v_add_f32 v4, v4, v8
        v_add_u32 v6, v6, 1
        v_cmp_lt_u32 vcc_lo, v6, s11
        s_cbranch_vccnz loop
        v_mul_lo_u32 v7, v1, s10
        v_lshlrev_b32 v7, 2, v7
        v_add_u32 v7, v7, v2
        global_store_b32 v7, v4, s[8:9]
        s_waitcnt_vscnt null, 0x0
done:
        s_endpgm
        .rodata
        .p2align 6
        .amdhsa_kernel matmul
          .amdhsa_user_sgpr_kernarg_segment_ptr 1
          .amdhsa_system_sgpr_workgroup_id_y 1
          .amdhsa_next_free_vgpr 18
          .amdhsa_next_free_sgpr 13
          .amdhsa_wavefront_size32 1
        .end_amdhsa_kernel
