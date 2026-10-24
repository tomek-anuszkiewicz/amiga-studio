; Load independent words through postincrement.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    movea.w (a3)+,a5
    move.l d7,d0
    movea.w (a3)+,a5
    move.l d7,d0
    movea.w (a3)+,a5
    move.l d7,d0
    movea.w (a3)+,a5
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
