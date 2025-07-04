package zip.meows.musicopy

import kotlinx.datetime.Clock
import uniffi.musicopy.ServerModel

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
        connectedAt = Clock.System.now().epochSeconds.toULong(),
        connectionType = "direct",
        latencyMs = 42u
    )
}
