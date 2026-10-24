; Move l operands from stack_postincrement to absolute_short.
source_data:
    dcb.w 200000,$5a3c
entry:
    rept {{instruction_count}}
    move.l (a7)+,$2000.w
    move.l d3,d4
    move.l (a7)+,$2000.w
    move.l d3,d4
    move.l (a7)+,$2000.w
    move.l d3,d4
    move.l (a7)+,$2000.w
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
