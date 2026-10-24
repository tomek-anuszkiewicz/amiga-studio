; Each destination word is read once, avoiding cumulative bit clearing.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    and.w d3,(a5)+
    move.l d7,d0
    and.w d3,(a5)+
    move.l d7,d0
    and.w d3,(a5)+
    move.l d7,d0
    and.w d3,(a5)+
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
