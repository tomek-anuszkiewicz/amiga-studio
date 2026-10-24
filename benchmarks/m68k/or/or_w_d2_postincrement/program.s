; Each destination word is read once, avoiding cumulative bit setting.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    or.w d2,(a4)+
    move.l d7,d0
    or.w d2,(a4)+
    move.l d7,d0
    or.w d2,(a4)+
    move.l d7,d0
    or.w d2,(a4)+
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
