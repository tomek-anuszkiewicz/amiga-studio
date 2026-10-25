; Four RTS targets, with MOVE instructions preparing each return frame.
entry:
    rept {{instruction_count}}
    move.l #*+8,-(sp)
    rts
    move.l #*+8,-(sp)
    rts
    move.l #*+8,-(sp)
    rts
    move.l #*+8,-(sp)
    rts
    endr
    dc.b "M68K_END_MARKER!"
