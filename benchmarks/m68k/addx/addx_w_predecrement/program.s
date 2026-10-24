; Read or update successive words across the operand array.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    addx.w -(a3),-(a4)
    move.l d7,d0
    addx.w -(a3),-(a4)
    move.l d7,d0
    addx.w -(a3),-(a4)
    move.l d7,d0
    addx.w -(a3),-(a4)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
