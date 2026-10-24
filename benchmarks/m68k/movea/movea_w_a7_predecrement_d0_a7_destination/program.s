; Load A7 from the selected source EA, then restore it.
; An inert MOVE.L precedes each address-register restoration.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    movea.w -(a7),a7
    move.l d7,d0
    movea.l d7,a7
    movea.w -(a7),a7
    move.l d7,d0
    movea.l d7,a7
    movea.w -(a7),a7
    move.l d7,d0
    movea.l d7,a7
    movea.w -(a7),a7
    move.l d7,d0
    movea.l d7,a7
    endr
    dc.b "M68K_END_MARKER!"
