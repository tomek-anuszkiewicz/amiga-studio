; Measure ANDI.L immediate operand with this destination mode.
entry:
    rept {{instruction_count}}
    andi.l #$13572469,$1000.w
    move.l d7,d0
    andi.l #$13572469,$1000.w
    move.l d7,d0
    andi.l #$13572469,$1000.w
    move.l d7,d0
    andi.l #$13572469,$1000.w
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
