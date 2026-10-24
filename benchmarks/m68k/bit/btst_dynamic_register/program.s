; Test three dynamic bit numbers and repeat the first register pair.
entry:
    rept {{instruction_count}}
    btst d0,d1
    move.l d6,d7
    btst d2,d3
    move.l d6,d7
    btst d4,d5
    move.l d6,d7
    btst d0,d1
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
