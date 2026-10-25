; MOVE.L reloads the counter; MOVE.W sets Z for the next target.
; A missed branch executes the ILLEGAL sentinel.
entry:
    rept {{instruction_count}}
    dbeq d5,*+6
    dc.w $4AFC
    move.l d6,d5
    move.w #$0000,ccr
    dbeq d5,*+6
    dc.w $4AFC
    move.l d6,d5
    move.w #$0000,ccr
    dbeq d5,*+6
    dc.w $4AFC
    move.l d6,d5
    move.w #$0000,ccr
    dbeq d5,*+6
    dc.w $4AFC
    move.l d6,d5
    move.w #$0000,ccr
    endr
    dc.b "M68K_END_MARKER!"
