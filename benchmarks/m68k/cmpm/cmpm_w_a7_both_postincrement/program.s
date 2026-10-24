; Both word operands advance the same supervisor stack pointer.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    cmpm.w (a7)+,(a7)+
    move.l d4,d1
    cmpm.w (a7)+,(a7)+
    move.l d4,d1
    cmpm.w (a7)+,(a7)+
    move.l d4,d1
    cmpm.w (a7)+,(a7)+
    move.l d4,d1
    endr
    dc.b "M68K_END_MARKER!"
