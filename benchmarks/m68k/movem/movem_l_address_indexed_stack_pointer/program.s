; Transfer 1 l register(s) through address_indexed.
entry:
    rept {{instruction_count}}
    movem.l 6(a3,d6.w),a7
    movea.l d7,a7
    movem.l 6(a3,d6.w),a7
    movea.l d7,a7
    movem.l 6(a3,d6.w),a7
    movea.l d7,a7
    movem.l 6(a3,d6.w),a7
    movea.l d7,a7
    endr
    dc.b "M68K_END_MARKER!"
