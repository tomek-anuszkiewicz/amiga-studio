; Test a bit in the immediate effective-address operand.
entry:
    rept {{instruction_count}}
    btst d3,#$23
    move.l d7,d0
    btst d3,#$23
    move.l d7,d0
    btst d3,#$23
    move.l d7,d0
    btst d3,#$23
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
