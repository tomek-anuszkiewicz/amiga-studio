; Move b operands from pc_indexed to address_indexed.
    dc.w $2803
entry:
    rept {{instruction_count}}
    move.b *-10(pc,d6.w),6(a2,d1.w)
    move.l d3,d4
    move.b *-10(pc,d6.w),6(a2,d1.w)
    move.l d3,d4
    move.b *-10(pc,d6.w),6(a2,d1.w)
    move.l d3,d4
    move.b *-10(pc,d6.w),6(a2,d1.w)
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
