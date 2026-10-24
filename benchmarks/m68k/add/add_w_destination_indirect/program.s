; Add a data register word to a memory effective address.
operand_data:
    dc.w $1234
entry:
    rept {{instruction_count}}
    add.w d6,(a4)
    move.l d7,d0
    add.w d6,(a4)
    move.l d7,d0
    add.w d6,(a4)
    move.l d7,d0
    add.w d6,(a4)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
