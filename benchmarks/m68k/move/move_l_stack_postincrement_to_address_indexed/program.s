; Move l operands from stack_postincrement to address_indexed.
source_data:
    dcb.w 200000,$5a3c
entry:
    rept {{instruction_count}}
    move.l (a7)+,6(a2,d1.w)
    move.l d3,d4
    move.l (a7)+,6(a2,d1.w)
    move.l d3,d4
    move.l (a7)+,6(a2,d1.w)
    move.l d3,d4
    move.l (a7)+,6(a2,d1.w)
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
