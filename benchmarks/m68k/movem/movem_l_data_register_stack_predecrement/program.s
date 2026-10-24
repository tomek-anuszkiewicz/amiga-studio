; Transfer 1 l register(s) through stack_predecrement.
operand_data:
    dcb.w 200000,$1234
entry:
    rept {{instruction_count}}
    movem.l d2,-(a7)
    move.l d7,d0
    movem.l d2,-(a7)
    move.l d7,d0
    movem.l d2,-(a7)
    move.l d7,d0
    movem.l d2,-(a7)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
