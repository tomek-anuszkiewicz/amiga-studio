; Add four stable register words to the absolute memory destination.
operand_data:
    dc.w $1234
entry:
    rept {{instruction_count}}
    add.b d0,$200000.l
    move.l d5,d1
    add.b d1,$200000.l
    move.l d6,d2
    add.b d2,$200000.l
    move.l d7,d3
    add.b d3,$200000.l
    move.l d4,d0
    endr
    dc.b "M68K_END_MARKER!"
