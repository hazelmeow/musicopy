package zip.meows.musicopy

import kotlinx.datetime.Clock
import uniffi.musicopy.CounterModel
import uniffi.musicopy.ServerModel
import uniffi.musicopy.TransferJobModel
import uniffi.musicopy.TransferJobProgressModel

fun shortenNodeId(nodeId: String): String {
    return "${nodeId.slice(0..<6)}...${nodeId.slice((nodeId.length - 6)..<(nodeId.length))}"
}

fun mockNodeId(): String {
    val allowedChars = ('a'..'f') + ('0'..'9')
    return (1..64)
        .map { allowedChars.random() }
        .joinToString("")
}

fun mockServerModel(): ServerModel {
    return ServerModel(
        name = "My Phone",
        nodeId = mockNodeId(),
        connectedAt = now(),
        accepted = true,
        connectionType = "direct",
        latencyMs = 42u,
        transferJobs = emptyList()
    )
}

fun mockTransferJobModel(): TransferJobModel {
    return TransferJobModel(
        jobId = 1u,
        fileRoot = "root",
        filePath = "a/b/c.mp3",
        fileSize = 12345678u,
        progress = TransferJobProgressModel.InProgress(
            startedAt = now() - 5u,
            bytes = CounterModel(2345678u)
        )
    )
}

internal fun now(): ULong {
    return Clock.System.now().epochSeconds.toULong()
}
