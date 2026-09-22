        .amdgcn_target "amdgcn-amd-amdhsa--gfx1151"
        // Bisect kernel: vadd's exec-mask sequence with no loads.
        // kernarg as vadd. Lanes with gid < n store 42 to c[gid].
        .text
        .globl vbranch
        .p2align 8
        .type vbranch,@function
vbranch:
        s_load_b128 s[4:7], s[0:1], 0x0
        s_load_b64 s[8:9], s[0:1], 0x10
        s_load_b32 s10, s[0:1], 0x18
        v_lshl_add_u32 v1, s2, 8, v0
        s_waitcnt lgkmcnt(0)
        v_cmp_gt_u32 vcc_lo, s10, v1
        s_and_saveexec_b32 s11, vcc_lo
        s_cbranch_execz done
        v_lshlrev_b32 v2, 2, v1
        v_mov_b32 v3, 42
        global_store_b32 v2, v3, s[8:9]
        s_waitcnt_vscnt null, 0x0
done:
        s_endpgm
        .rodata
        .p2align 6
        .amdhsa_kernel vbranch
          .amdhsa_user_sgpr_kernarg_segment_ptr 1
          .amdhsa_next_free_vgpr 4
          .amdhsa_next_free_sgpr 12
          .amdhsa_wavefront_size32 1
        .end_amdhsa_kernel
