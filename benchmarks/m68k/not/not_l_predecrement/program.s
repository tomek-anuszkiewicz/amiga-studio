; Measure NOT.L with the selected destination mode.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    not.l -(a3)
    move.l d7,d0
    not.l -(a3)
    move.l d7,d0
    not.l -(a3)
    move.l d7,d0
    not.l -(a3)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
