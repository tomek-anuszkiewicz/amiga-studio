; Compare two disjoint arrays through CMPM's only addressing form.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    cmpm.w (a2)+,(a4)+
    move.l d7,d0
    cmpm.w (a2)+,(a4)+
    move.l d7,d0
    cmpm.w (a2)+,(a4)+
    move.l d7,d0
    cmpm.w (a2)+,(a4)+
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
