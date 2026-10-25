; The target writes a byte through address register indirect.
operand_data:
    dc.w $3457
entry:
    rept {{instruction_count}}
    st (a1)
    move.l d6,d7
    st (a1)
    move.l d6,d7
    st (a1)
    move.l d6,d7
    st (a1)
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
