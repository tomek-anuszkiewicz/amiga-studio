; Move w operands from address_indexed to address_indirect.
entry:
    rept {{instruction_count}}
    move.w 6(a1,d6.w),(a2)
    move.l d3,d4
    move.w 6(a1,d6.w),(a2)
    move.l d3,d4
    move.w 6(a1,d6.w),(a2)
    move.l d3,d4
    move.w 6(a1,d6.w),(a2)
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
