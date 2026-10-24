; Distinct longword streams, with SSP supplying the source.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    cmpm.l (a7)+,(a5)+
    move.l d1,d4
    cmpm.l (a7)+,(a5)+
    move.l d1,d4
    cmpm.l (a7)+,(a5)+
    move.l d1,d4
    cmpm.l (a7)+,(a5)+
    move.l d1,d4
    endr
    dc.b "M68K_END_MARKER!"
