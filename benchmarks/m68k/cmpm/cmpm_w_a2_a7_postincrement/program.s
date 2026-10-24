; Distinct word streams, with SSP supplying the destination.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    cmpm.w (a2)+,(a7)+
    move.l d5,d1
    cmpm.w (a2)+,(a7)+
    move.l d5,d1
    cmpm.w (a2)+,(a7)+
    move.l d5,d1
    cmpm.w (a2)+,(a7)+
    move.l d5,d1
    endr
    dc.b "M68K_END_MARKER!"
