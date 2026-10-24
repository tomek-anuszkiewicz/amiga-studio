; Use a dynamic bit number and a fixed destination.
operand_data:
    dc.w $1234
entry:
    rept {{instruction_count}}
    bset d0,(a5)
    move.l d7,d1
    bset d0,(a5)
    move.l d7,d1
    bset d0,(a5)
    move.l d7,d1
    bset d0,(a5)
    move.l d7,d1
    endr
    dc.b "M68K_END_MARKER!"
