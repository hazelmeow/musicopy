package zip.meows.musicopy

import kotlinx.datetime.Clock
import uniffi.musicopy.ClientModel
import uniffi.musicopy.CounterModel
import uniffi.musicopy.FileSizeModel
import uniffi.musicopy.IndexItemModel
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

fun mockClientModel(): ClientModel {
    val nodeId = mockNodeId()

    return ClientModel(
        name = "My Desktop",
        nodeId = mockNodeId(),
        connectedAt = now(),
        accepted = true,
        connectionType = "direct",
        latencyMs = 42u,
        index = listOf(
            mockIndexItemModel(nodeId = nodeId, root = "one", basePath = "/a"),
            mockIndexItemModel(nodeId = nodeId, root = "one", basePath = "/a"),
            mockIndexItemModel(nodeId = nodeId, root = "one", basePath = "/a"),
            mockIndexItemModel(nodeId = nodeId, root = "one", basePath = "/a/b"),
            mockIndexItemModel(nodeId = nodeId, root = "one", basePath = "/a/b"),
            mockIndexItemModel(nodeId = nodeId, root = "one", basePath = "/a/b"),
            mockIndexItemModel(nodeId = nodeId, root = "one", basePath = "/a/b/c"),
            mockIndexItemModel(nodeId = nodeId, root = "one", basePath = "/a/b/c"),
            mockIndexItemModel(nodeId = nodeId, root = "one", basePath = "/a/b/c"),
            mockIndexItemModel(nodeId = nodeId, root = "one", basePath = "/a/d"),
            mockIndexItemModel(nodeId = nodeId, root = "one", basePath = "/a/d"),
            mockIndexItemModel(nodeId = nodeId, root = "one", basePath = "/a/d"),
            mockIndexItemModel(nodeId = nodeId, root = "one", basePath = "/e"),
            mockIndexItemModel(nodeId = nodeId, root = "one", basePath = "/e"),
            mockIndexItemModel(nodeId = nodeId, root = "one", basePath = "/e"),
            mockIndexItemModel(nodeId = nodeId, root = "one", basePath = "/e"),

            mockIndexItemModel(nodeId = nodeId, root = "two", basePath = "/a/foo/bar/baz"),
            mockIndexItemModel(nodeId = nodeId, root = "two", basePath = "/a/foo/bar/baz"),
            mockIndexItemModel(nodeId = nodeId, root = "two", basePath = "/a/foo/bar/baz"),
            mockIndexItemModel(nodeId = nodeId, root = "two", basePath = "/a/foo/bar/baz/b"),
            mockIndexItemModel(nodeId = nodeId, root = "two", basePath = "/a/foo/bar/baz/b"),
            mockIndexItemModel(nodeId = nodeId, root = "two", basePath = "/a/foo/bar/baz/b"),
            mockIndexItemModel(nodeId = nodeId, root = "two", basePath = "/a/foo/bar/baz/b/c"),
            mockIndexItemModel(nodeId = nodeId, root = "two", basePath = "/a/foo/bar/baz/b/c"),
            mockIndexItemModel(nodeId = nodeId, root = "two", basePath = "/a/foo/bar/baz/b/c"),
            mockIndexItemModel(nodeId = nodeId, root = "two", basePath = "/a/foo/bar/baz/d"),
            mockIndexItemModel(nodeId = nodeId, root = "two", basePath = "/a/foo/bar/baz/d"),
            mockIndexItemModel(nodeId = nodeId, root = "two", basePath = "/a/foo/bar/baz/d"),
            mockIndexItemModel(nodeId = nodeId, root = "two", basePath = "/e/foo/bar/baz"),
            mockIndexItemModel(nodeId = nodeId, root = "two", basePath = "/e/foo/bar/baz"),
            mockIndexItemModel(nodeId = nodeId, root = "two", basePath = "/e/foo/bar/baz"),
            mockIndexItemModel(nodeId = nodeId, root = "two", basePath = "/e/foo/bar/baz"),

            mockIndexItemModel(nodeId = nodeId, root = "ex", basePath = "/gen1/art1/alb1"),
            mockIndexItemModel(nodeId = nodeId, root = "ex", basePath = "/gen1/art1/alb1"),
            mockIndexItemModel(nodeId = nodeId, root = "ex", basePath = "/gen1/art1/alb1"),
            mockIndexItemModel(nodeId = nodeId, root = "ex", basePath = "/gen1/art1/alb2"),
            mockIndexItemModel(nodeId = nodeId, root = "ex", basePath = "/gen1/art1/alb2"),
            mockIndexItemModel(nodeId = nodeId, root = "ex", basePath = "/gen1/art1/alb2"),
            mockIndexItemModel(nodeId = nodeId, root = "ex", basePath = "/gen1/art2"),
            mockIndexItemModel(nodeId = nodeId, root = "ex", basePath = "/gen1/art2"),
            mockIndexItemModel(nodeId = nodeId, root = "ex", basePath = "/gen1/art2"),
            mockIndexItemModel(nodeId = nodeId, root = "ex", basePath = "/gen1/art2/alb"),
            mockIndexItemModel(nodeId = nodeId, root = "ex", basePath = "/gen1/art2/alb"),
            mockIndexItemModel(nodeId = nodeId, root = "ex", basePath = "/gen1/art2/alb"),
            mockIndexItemModel(nodeId = nodeId, root = "ex", basePath = "/gen2/art3/alb1"),
            mockIndexItemModel(nodeId = nodeId, root = "ex", basePath = "/gen2/art3/alb1"),
            mockIndexItemModel(nodeId = nodeId, root = "ex", basePath = "/gen2/art3/alb1"),
            mockIndexItemModel(nodeId = nodeId, root = "ex", basePath = "/gen2/art3/alb2"),
            mockIndexItemModel(nodeId = nodeId, root = "ex", basePath = "/gen2/art3/alb2"),
            mockIndexItemModel(nodeId = nodeId, root = "ex", basePath = "/gen2/art3/alb2"),
            mockIndexItemModel(nodeId = nodeId, root = "ex", basePath = "/gen2/art4/alb1"),
            mockIndexItemModel(nodeId = nodeId, root = "ex", basePath = "/gen2/art4/alb1"),
            mockIndexItemModel(nodeId = nodeId, root = "ex", basePath = "/gen2/art4/alb1"),
            mockIndexItemModel(nodeId = nodeId, root = "ex", basePath = "/gen2/art4/alb2"),
            mockIndexItemModel(nodeId = nodeId, root = "ex", basePath = "/gen2/art4/alb2"),
            mockIndexItemModel(nodeId = nodeId, root = "ex", basePath = "/gen2/art4/alb2"),
        ),
        transferJobs = buildList {
            repeat(100) {
                add(mockTransferJobModel(progress = mockTransferJobProgressModelRequested()))
                add(mockTransferJobModel(progress = mockTransferJobProgressModelTranscoding()))
                add(mockTransferJobModel(progress = mockTransferJobProgressModelReady()))
                add(mockTransferJobModel(progress = mockTransferJobProgressModelInProgress()))
                add(mockTransferJobModel(progress = mockTransferJobProgressModelFinished()))
                add(mockTransferJobModel(progress = mockTransferJobProgressModelFailed()))
            }
        }
    )
}

var nextMockIndexItemCount: Int = 1

fun mockIndexItemModel(
    nodeId: String = mockNodeId(),
    root: String = "library",
    basePath: String = "/a/b/c",
): IndexItemModel {
    val itemCount = nextMockIndexItemCount++

    return IndexItemModel(
        nodeId = nodeId,
        root = root,
        path = "${basePath}/file${itemCount}.flac",

        hashKind = "test",
        hash = byteArrayOf(12, 34),

        fileSize = when (itemCount % 10) {
            0 -> FileSizeModel.Unknown
            else -> when (itemCount % 2) {
                0 -> FileSizeModel.Actual(12345678u)
                else -> FileSizeModel.Estimated(10000000u)
            }
        },
    )
}

var nextMockJobId: ULong = 0u

fun mockTransferJobModel(
    progress: TransferJobProgressModel = mockTransferJobProgressModelInProgress(),
): TransferJobModel {
    return TransferJobModel(
        jobId = nextMockJobId++,
        fileRoot = "root",
        filePath = "a/b/c.mp3",
        fileSize = 12345678u,
        progress = progress
    )
}

fun mockTransferJobProgressModelRequested() = TransferJobProgressModel.Requested

fun mockTransferJobProgressModelTranscoding() = TransferJobProgressModel.Transcoding

fun mockTransferJobProgressModelReady() = TransferJobProgressModel.Ready

fun mockTransferJobProgressModelInProgress() = TransferJobProgressModel.InProgress(
    startedAt = now() - 5u,
    bytes = CounterModel(2345678u)
)

fun mockTransferJobProgressModelFinished() = TransferJobProgressModel.Finished(
    finishedAt = now() - 1u
)

fun mockTransferJobProgressModelFailed() = TransferJobProgressModel.Failed(
    error = "something went wrong"
)


internal fun now(): ULong {
    return Clock.System.now().epochSeconds.toULong()
}
