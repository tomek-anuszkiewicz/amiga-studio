; Move b operands from immediate to address_indirect.
entry:
    rept {{instruction_count}}
    move.b #$5a,(a2)
    move.l d3,d4
    move.b #$5a,(a2)
    move.l d3,d4
    move.b #$5a,(a2)
    move.l d3,d4
    move.b #$5a,(a2)
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
