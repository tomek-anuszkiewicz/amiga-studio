; Measure SUBQ.W #5 with this destination mode.
operand_data:
    dc.w $3457
entry:
    rept {{instruction_count}}
    subq.w #5,$200000.l
    move.l d7,d0
    subq.w #5,$200000.l
    move.l d7,d0
    subq.w #5,$200000.l
    move.l d7,d0
    subq.w #5,$200000.l
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
