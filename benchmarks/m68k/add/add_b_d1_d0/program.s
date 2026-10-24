; Add stable odd source words through three register pairs, repeating the first pair.
; Word-sized destinations preserve their upper halves.
entry:
    rept {{instruction_count}}
    add.b d0,d1
    move.l d6,d7
    add.b d2,d3
    move.l d6,d7
    add.b d4,d5
    move.l d6,d7
    add.b d0,d1
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
