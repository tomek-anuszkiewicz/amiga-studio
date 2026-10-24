; Add a data register word to a memory effective address.
entry:
    rept {{instruction_count}}
    add.b d6,$1000.w
    move.l d7,d0
    add.b d6,$1000.w
    move.l d7,d0
    add.b d6,$1000.w
    move.l d7,d0
    add.b d6,$1000.w
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
