; MOVE.W #$0000,CCR sets the intended flags. DBVS condition is false.
; The counter is reloaded before every target; a missed branch
; executes the ILLEGAL sentinel.
entry:
    rept {{instruction_count}}
    dbvs d5,*+6
    dc.w $4AFC
    move.l d6,d5
    move.w #$0000,ccr
    dbvs d5,*+6
    dc.w $4AFC
    move.l d6,d5
    move.w #$0000,ccr
    dbvs d5,*+6
    dc.w $4AFC
    move.l d6,d5
    move.w #$0000,ccr
    dbvs d5,*+6
    dc.w $4AFC
    move.l d6,d5
    move.w #$0000,ccr
    endr
    dc.b "M68K_END_MARKER!"
