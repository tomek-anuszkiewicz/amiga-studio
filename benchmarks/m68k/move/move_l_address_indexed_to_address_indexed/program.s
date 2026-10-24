; Move l operands from address_indexed to address_indexed.
entry:
    rept {{instruction_count}}
    move.l 6(a1,d6.w),6(a2,d1.w)
    move.l d3,d4
    move.l 6(a1,d6.w),6(a2,d1.w)
    move.l d3,d4
    move.l 6(a1,d6.w),6(a2,d1.w)
    move.l d3,d4
    move.l 6(a1,d6.w),6(a2,d1.w)
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
