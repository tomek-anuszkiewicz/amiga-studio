; Four BSR.S targets jump over sentinels to MOVE.L; return addresses stay on the stack.
entry:
    rept {{instruction_count}}
    bsr.s *+4
    dc.w $4afc
    move.l d6,d7
    bsr.s *+4
    dc.w $4afc
    move.l d6,d7
    bsr.s *+4
    dc.w $4afc
    move.l d6,d7
    bsr.s *+4
    dc.w $4afc
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
