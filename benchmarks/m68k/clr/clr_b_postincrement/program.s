; Measure CLR.B with the selected destination mode.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    clr.b (a3)+
    move.l d7,d0
    clr.b (a3)+
    move.l d7,d0
    clr.b (a3)+
    move.l d7,d0
    clr.b (a3)+
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
