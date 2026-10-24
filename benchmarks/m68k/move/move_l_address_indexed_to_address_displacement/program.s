; Move l operands from address_indexed to address_displacement.
entry:
    rept {{instruction_count}}
    move.l 6(a1,d6.w),6(a2)
    move.l d3,d4
    move.l 6(a1,d6.w),6(a2)
    move.l d3,d4
    move.l 6(a1,d6.w),6(a2)
    move.l d3,d4
    move.l 6(a1,d6.w),6(a2)
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
