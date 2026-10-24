; Measure NBCD memory addressing with packed decimal operands.
operand_data:
    dc.w $2525
entry:
    rept {{instruction_count}}
    nbcd (a3)
    move.l d7,d0
    nbcd (a3)
    move.l d7,d0
    nbcd (a3)
    move.l d7,d0
    nbcd (a3)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
