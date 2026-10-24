; Distinct word streams, with SSP supplying the source.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    cmpm.w (a7)+,(a4)+
    move.l d2,d5
    cmpm.w (a7)+,(a4)+
    move.l d2,d5
    cmpm.w (a7)+,(a4)+
    move.l d2,d5
    cmpm.w (a7)+,(a4)+
    move.l d2,d5
    endr
    dc.b "M68K_END_MARKER!"
