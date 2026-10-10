; ==============================================================================
; Bubble Sort (Motorola 68000)
; ==============================================================================
; Copies an unsorted 8-element 16-bit word array to working RAM at $002000,
; then sorts it in-place using bubble sort.
;
; Target Load Address: $001000
; Execution Mode:      Pure CPU (no Amiga OS or custom chip DMA required)
;
; Initial Data:
;   $0042, $0010, $0099, $0003, $0077, $0025, $0001, $0050
;
; Expected Sorted Output at $002000:
;   $0001, $0003, $0010, $0025, $0042, $0050, $0077, $0099
;
; Register Usage:
;   A0 - Current array element pointer (walks through $002000)
;   A1 - Destination copy pointer during initial data setup
;   D0 - array[i] / copy loop counter
;   D1 - array[i+1] adjacent comparison element
;   D6 - Inner loop pass counter (number of comparisons in current pass)
;   D7 - Outer loop pass counter (runs N-2 = 6 down to 0)
;
; Debugger Tips:
;   - Set a breakpoint at $001028 (swap block) to break only when elements swap.
;   - Open Memory Hex Viewer at $002000 to observe in-place sorting in real time.
;   - Use Temporal History (Alt+Left) to scrub backward and inspect previous passes.
; ==============================================================================

            ORG     $001000

start:
            ; --- Step 1: Copy sample data to working RAM at $002000 ---
            LEA     sample_data(PC), A0 ; A0 = source data in ROM/code section
            LEA     $002000, A1         ; A1 = working RAM destination
            MOVEQ   #7, D0              ; 8 words to copy (counter 7 down to 0)

copy_loop:
            MOVE.W  (A0)+, (A1)+        ; Copy word and post-increment both pointers
            DBRA    D0, copy_loop

            ; --- Step 2: Bubble sort in-place at $002000 ---
            MOVE.W  #6, D7              ; Outer loop: N-2 (6 down to 0 for 8 elements)

outer_loop:
            LEA     $002000, A0         ; Reset array pointer to start of table
            MOVE.W  D7, D6              ; Inner loop comparisons = outer index

inner_loop:
            MOVE.W  (A0), D0            ; D0 = array[i]
            MOVE.W  2(A0), D1           ; D1 = array[i+1] (indexed offset +2)
            CMP.W   D1, D0              ; Compare D0 - D1 (array[i] - array[i+1])
            BLS.S   no_swap             ; If array[i] <= array[i+1], already in order

            ; Swap elements in-place
            MOVE.W  D1, (A0)            ; Store smaller element at array[i]
            MOVE.W  D0, 2(A0)           ; Store larger element at array[i+1]

no_swap:
            ADDQ.L  #2, A0              ; Advance to next word pair
            DBRA    D6, inner_loop      ; Decrement inner counter and loop

            DBRA    D7, outer_loop      ; Decrement outer pass counter and loop

halt:
            BRA.S   halt                ; Finished! Spin in idle loop ($60FE)

            ; --- Data Section ---
sample_data:
            DC.W    $0042, $0010, $0099, $0003, $0077, $0025, $0001, $0050
