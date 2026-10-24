; Reload register destinations from D7 so each logical target uses a fresh operand.
entry:
    rept {{instruction_count}}
    and.w #$1234,d0
    move.l d7,d0
    and.w #$1234,d0
    move.l d7,d0
    and.w #$1234,d0
    move.l d7,d0
    and.w #$1234,d0
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
