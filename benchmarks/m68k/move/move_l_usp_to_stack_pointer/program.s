; Restore the destination after each privileged USP transfer.
; An inert MOVE.L precedes each status or USP restoration.
entry:
    rept {{instruction_count}}
    move.l usp,a7
    move.l d0,d1
    movea.l d7,a7
    move.l usp,a7
    move.l d0,d1
    movea.l d7,a7
    move.l usp,a7
    move.l d0,d1
    movea.l d7,a7
    move.l usp,a7
    move.l d0,d1
    movea.l d7,a7
    endr
    dc.b "M68K_END_MARKER!"
