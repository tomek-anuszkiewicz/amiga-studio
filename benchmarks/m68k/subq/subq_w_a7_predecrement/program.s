; Measure SUBQ.W #5 with this destination mode.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    subq.w #5,-(a7)
    move.l d7,d0
    subq.w #5,-(a7)
    move.l d7,d0
    subq.w #5,-(a7)
    move.l d7,d0
    subq.w #5,-(a7)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
