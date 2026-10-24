; Compare independent memory words with an address register.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    cmpa.w (a3)+,a4
    move.l d7,d0
    cmpa.w (a3)+,a4
    move.l d7,d0
    cmpa.w (a3)+,a4
    move.l d7,d0
    cmpa.w (a3)+,a4
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
