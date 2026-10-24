; Move b operands from absolute_short to address_displacement.
entry:
    rept {{instruction_count}}
    move.b $1000.w,6(a2)
    move.l d3,d4
    move.b $1000.w,6(a2)
    move.l d3,d4
    move.b $1000.w,6(a2)
    move.l d3,d4
    move.b $1000.w,6(a2)
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
