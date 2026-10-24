; Walk initialized word data backward with A7 byte predecrement.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    eor.b d4,-(a7)
    move.l d7,d0
    eor.b d4,-(a7)
    move.l d7,d0
    eor.b d4,-(a7)
    move.l d7,d0
    eor.b d4,-(a7)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
