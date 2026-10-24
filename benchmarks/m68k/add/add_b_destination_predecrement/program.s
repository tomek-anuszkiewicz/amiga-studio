; Add four stable register words to separate predecrement memory streams.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    add.b d0,-(a0)
    move.l d5,d1
    add.b d1,-(a1)
    move.l d6,d2
    add.b d2,-(a2)
    move.l d7,d3
    add.b d3,-(a3)
    move.l d4,d0
    endr
    dc.b "M68K_END_MARKER!"
