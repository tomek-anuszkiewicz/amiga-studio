; Distinct longword streams, with SSP supplying the destination.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    cmpm.l (a3)+,(a7)+
    move.l d4,d2
    cmpm.l (a3)+,(a7)+
    move.l d4,d2
    cmpm.l (a3)+,(a7)+
    move.l d4,d2
    cmpm.l (a3)+,(a7)+
    move.l d4,d2
    endr
    dc.b "M68K_END_MARKER!"
