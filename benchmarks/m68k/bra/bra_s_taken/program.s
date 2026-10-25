; Four BRA.S *+4 targets, each followed by a MOVE helper.
entry:
    rept {{instruction_count}}
    bra.s *+4
    dc.w $4afc
    move.l d6,d7
    bra.s *+4
    dc.w $4afc
    move.l d6,d7
    bra.s *+4
    dc.w $4afc
    move.l d6,d7
    bra.s *+4
    dc.w $4afc
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
