; Add four stable register words to the displacement memory destination.
    dc.w $1357
    dc.w $246B
    dc.w $369D
operand_data:
    dc.l $12345679
entry:
    rept {{instruction_count}}
    add.l d0,6(a5)
    move.l d5,d1
    add.l d1,6(a5)
    move.l d6,d2
    add.l d2,6(a5)
    move.l d7,d3
    add.l d3,6(a5)
    move.l d4,d0
    endr
    dc.b "M68K_END_MARKER!"
