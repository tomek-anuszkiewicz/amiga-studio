; The BCD word region holds separate source and destination byte arrays.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    sbcd -(a2),-(a4)
    move.l d7,d0
    sbcd -(a2),-(a4)
    move.l d7,d0
    sbcd -(a2),-(a4)
    move.l d7,d0
    sbcd -(a2),-(a4)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
