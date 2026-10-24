; Both longword operands use SSP and advance the same stack pointer.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    cmpm.l (a7)+,(a7)+
    move.l d4,d0
    cmpm.l (a7)+,(a7)+
    move.l d4,d0
    cmpm.l (a7)+,(a7)+
    move.l d4,d0
    cmpm.l (a7)+,(a7)+
    move.l d4,d0
    endr
    dc.b "M68K_END_MARKER!"
