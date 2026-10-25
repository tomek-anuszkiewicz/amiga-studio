; User-mode STOP traps before entering the stopped state.
; This path needs no wake interrupt from the runner; the handler skips the
; four-byte STOP and returns to the next target.
privilege_handler:
    addq.l #4,2(a7)
    rte
entry:
    rept {{instruction_count}}
    stop #$2700
    stop #$2700
    stop #$2700
    stop #$2700
    endr
    dc.b "M68K_END_MARKER!"
