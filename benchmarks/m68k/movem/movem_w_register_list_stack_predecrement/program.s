; Transfer 3 w register(s) through stack_predecrement.
operand_data:
    dcb.w 300000,$1234
entry:
    rept {{instruction_count}}
    movem.w d2/d5/a5,-(a7)
    move.l d7,d0
    movem.w d2/d5/a5,-(a7)
    move.l d7,d0
    movem.w d2/d5/a5,-(a7)
    move.l d7,d0
    movem.w d2/d5/a5,-(a7)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
