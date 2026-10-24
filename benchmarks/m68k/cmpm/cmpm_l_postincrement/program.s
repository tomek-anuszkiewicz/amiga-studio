; Compare independent longword streams through both postincrement operands.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    cmpm.l (a3)+,(a5)+
    move.l d6,d1
    cmpm.l (a3)+,(a5)+
    move.l d6,d1
    cmpm.l (a3)+,(a5)+
    move.l d6,d1
    cmpm.l (a3)+,(a5)+
    move.l d6,d1
    endr
    dc.b "M68K_END_MARKER!"
