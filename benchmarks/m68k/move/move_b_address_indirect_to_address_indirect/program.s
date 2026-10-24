; Move b operands from address_indirect to address_indirect.
entry:
    rept {{instruction_count}}
    move.b (a1),(a2)
    move.l d3,d4
    move.b (a1),(a2)
    move.l d3,d4
    move.b (a1),(a2)
    move.l d3,d4
    move.b (a1),(a2)
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
