; Move the selected source word to CCR in supervisor mode.
; An inert MOVE.L precedes each status or USP restoration.
source_data:
    dcb.w 100000,$5a3c
entry:
    rept {{instruction_count}}
    move.w -(a1),ccr
    move.l d0,d1
    move.w d7,sr
    move.w -(a1),ccr
    move.l d0,d1
    move.w d7,sr
    move.w -(a1),ccr
    move.l d0,d1
    move.w d7,sr
    move.w -(a1),ccr
    move.l d0,d1
    move.w d7,sr
    endr
    dc.b "M68K_END_MARKER!"
