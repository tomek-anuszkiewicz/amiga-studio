; Move l operands from absolute_short to data_register.
entry:
    rept {{instruction_count}}
    move.l $1000.w,d5
    move.l d3,d4
    move.l $1000.w,d5
    move.l d3,d4
    move.l $1000.w,d5
    move.l d3,d4
    move.l $1000.w,d5
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
