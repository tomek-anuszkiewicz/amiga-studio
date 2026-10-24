; Measure EORI.W immediate operand with this destination mode.
operand_data:
    dc.w $3457
entry:
    rept {{instruction_count}}
    eori.w #$1357,(a3)
    move.l d7,d0
    eori.w #$1357,(a3)
    move.l d7,d0
    eori.w #$1357,(a3)
    move.l d7,d0
    eori.w #$1357,(a3)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
