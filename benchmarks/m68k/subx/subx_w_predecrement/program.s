; Walk two disjoint word arrays backward through predecrement.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    subx.w -(a2),-(a4)
    move.l d7,d0
    subx.w -(a2),-(a4)
    move.l d7,d0
    subx.w -(a2),-(a4)
    move.l d7,d0
    subx.w -(a2),-(a4)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
