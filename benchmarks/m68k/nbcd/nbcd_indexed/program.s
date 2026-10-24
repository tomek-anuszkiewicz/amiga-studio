; Measure NBCD memory addressing with packed decimal operands.
    dc.w $1325
    dc.w $2436
    dc.w $3547
    dc.w $4658
    dc.w $5769
    dc.w $6871
operand_data:
    dc.w $2525
entry:
    rept {{instruction_count}}
    nbcd 6(a3,d5.w)
    move.l d7,d0
    nbcd 6(a3,d5.w)
    move.l d7,d0
    nbcd 6(a3,d5.w)
    move.l d7,d0
    nbcd 6(a3,d5.w)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
