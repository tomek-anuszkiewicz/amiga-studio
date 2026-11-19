function Start-SharedRagService {
    <#
    .SYNOPSIS
        Start the RAG service through its owning Compose project's CLI.

    .PARAMETER RagCommand
        Path to the rag_qdrant launcher resolved by the bootstrap script.
    #>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)]
        [string]$RagCommand
    )

    & $RagCommand service start
}
