; Add a data register word to a memory effective address.
operand_data:
    dc.l $12345679
entry:
    rept {{instruction_count}}
    add.l d6,(a4)
    move.l d7,d0
    add.l d6,(a4)
    move.l d7,d0
    add.l d6,(a4)
    move.l d7,d0
    add.l d6,(a4)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
