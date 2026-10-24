; Add stable odd source words through three register pairs, repeating the first pair.
; Word-sized destinations preserve their upper halves.
entry:
    rept {{instruction_count}}
    add.w d0,d1
    move.l d6,d7
    add.w d2,d3
    move.l d6,d7
    add.w d4,d5
    move.l d6,d7
    add.w d0,d1
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
