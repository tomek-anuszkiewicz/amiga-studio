; Use a dynamic bit number and a fixed destination.
operand_data:
    dc.w $1234
entry:
    rept {{instruction_count}}
    bchg d6,(a1)
    move.l d7,d0
    bchg d6,(a1)
    move.l d7,d0
    bchg d6,(a1)
    move.l d7,d0
    bchg d6,(a1)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
