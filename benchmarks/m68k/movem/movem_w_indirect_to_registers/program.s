; Load a small register mask from a fixed memory address.
operand_data:
    dc.w $1235,$5A7B
entry:
    rept {{instruction_count}}
    movem.w (a3),d2/d5
    move.l d7,d0
    movem.w (a3),d2/d5
    move.l d7,d0
    movem.w (a3),d2/d5
    move.l d7,d0
    movem.w (a3),d2/d5
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
