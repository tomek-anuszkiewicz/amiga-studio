; Transfer 1 w register(s) through address_indexed.
entry:
    rept {{instruction_count}}
    movem.w 6(a3,d6.w),a7
    movea.l d7,a7
    movem.w 6(a3,d6.w),a7
    movea.l d7,a7
    movem.w 6(a3,d6.w),a7
    movea.l d7,a7
    movem.w 6(a3,d6.w),a7
    movea.l d7,a7
    endr
    dc.b "M68K_END_MARKER!"
