        .amdgcn_target "amdgcn-amd-amdhsa--gfx1151"
        // Probe: vadd unchanged, but the descriptor asks for 16
        // VGPRs (field 1) instead of 8 (field 0). If this passes
        // where vadd hangs, field 0 gives fewer than 5 registers.
        // c[i] = a[i] + b[i] for i < n. One work-item per element.
        // Workgroup size is fixed at 256 (the packet must say so).
        // kernarg: a u64 @0, b u64 @8, c u64 @16, n u32 @24.
        // s[0:1] = kernarg ptr, s2 = workgroup id x, v0 = local id.
        .text
        .globl vadd16
        .p2align 8
        .type vadd16,@function
vadd16:
        s_load_b128 s[4:7], s[0:1], 0x0
        s_load_b64 s[8:9], s[0:1], 0x10
        s_load_b32 s10, s[0:1], 0x18
        v_lshl_add_u32 v1, s2, 8, v0
        s_waitcnt lgkmcnt(0)
        v_cmp_gt_u32 vcc_lo, s10, v1
        s_and_saveexec_b32 s11, vcc_lo
        s_cbranch_execz done
        v_lshlrev_b32 v2, 2, v1
        global_load_b32 v3, v2, s[4:5]
        global_load_b32 v4, v2, s[6:7]
        s_waitcnt vmcnt(0)
        v_add_f32 v3, v3, v4
        global_store_b32 v2, v3, s[8:9]
        s_waitcnt_vscnt null, 0x0
done:
        s_endpgm
        .rodata
        .p2align 6
        .amdhsa_kernel vadd16
          .amdhsa_user_sgpr_kernarg_segment_ptr 1
          .amdhsa_next_free_vgpr 16
          .amdhsa_next_free_sgpr 12
          .amdhsa_wavefront_size32 1
        .end_amdhsa_kernel
