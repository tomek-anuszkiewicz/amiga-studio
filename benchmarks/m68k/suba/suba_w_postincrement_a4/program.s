; Read independent words while updating the address destination.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    suba.w (a3)+,a4
    move.l d7,d0
    suba.w (a3)+,a4
    move.l d7,d0
    suba.w (a3)+,a4
    move.l d7,d0
    suba.w (a3)+,a4
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
