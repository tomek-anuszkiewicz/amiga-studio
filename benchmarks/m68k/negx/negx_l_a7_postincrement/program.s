; Measure NEGX.L with the selected destination mode.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    negx.l (a7)+
    move.l d7,d0
    negx.l (a7)+
    move.l d7,d0
    negx.l (a7)+
    move.l d7,d0
    negx.l (a7)+
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
