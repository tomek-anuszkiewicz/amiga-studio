; Move b operands from immediate to address_indexed.
entry:
    rept {{instruction_count}}
    move.b #$5a,6(a2,d1.w)
    move.l d3,d4
    move.b #$5a,6(a2,d1.w)
    move.l d3,d4
    move.b #$5a,6(a2,d1.w)
    move.l d3,d4
    move.b #$5a,6(a2,d1.w)
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
