; Move l operands from absolute_short to address_indexed.
entry:
    rept {{instruction_count}}
    move.l $1000.w,6(a2,d1.w)
    move.l d3,d4
    move.l $1000.w,6(a2,d1.w)
    move.l d3,d4
    move.l $1000.w,6(a2,d1.w)
    move.l d3,d4
    move.l $1000.w,6(a2,d1.w)
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
