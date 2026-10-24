; Add a data register word to a memory effective address.
entry:
    rept {{instruction_count}}
    add.w d6,$1000.w
    move.l d7,d0
    add.w d6,$1000.w
    move.l d7,d0
    add.w d6,$1000.w
    move.l d7,d0
    add.w d6,$1000.w
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
