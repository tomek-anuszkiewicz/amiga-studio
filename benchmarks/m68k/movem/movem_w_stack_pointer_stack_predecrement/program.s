; Transfer 1 w register(s) through stack_predecrement.
operand_data:
    dcb.w 100000,$1234
entry:
    rept {{instruction_count}}
    movem.w a7,-(a7)
    move.l d7,d0
    movem.w a7,-(a7)
    move.l d7,d0
    movem.w a7,-(a7)
    move.l d7,d0
    movem.w a7,-(a7)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
