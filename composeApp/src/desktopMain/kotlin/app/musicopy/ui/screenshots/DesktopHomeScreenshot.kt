package app.musicopy.ui.screenshots

import androidx.compose.runtime.Composable
import app.musicopy.mockLibraryModel
import app.musicopy.mockNodeModel
import app.musicopy.mockServerModel
import app.musicopy.mockTransferJobModel
import app.musicopy.mockTransferJobProgressModelFinished
import app.musicopy.mockTransferJobProgressModelInProgress
import app.musicopy.mockTransferJobProgressModelReady
import app.musicopy.mockTransferJobProgressModelTranscoding
import app.musicopy.ui.DesktopHome
import uniffi.musicopy.LibraryRootModel
import uniffi.musicopy.Model

@Composable
fun DesktopHomeScreenshot() {
    val model = Model(
        node = mockNodeModel(
            nodeId = "ec3d55519d7486a99d326774e2831335a75ce2810156cddc279311ef670e0e21",
            servers = listOf(
                mockServerModel(
                    transferJobs = buildList {
                        repeat(12) {
                            add(mockTransferJobModel(progress = mockTransferJobProgressModelTranscoding()))
                            add(mockTransferJobModel(progress = mockTransferJobProgressModelReady()))
                            add(mockTransferJobModel(progress = mockTransferJobProgressModelInProgress()))
                            add(mockTransferJobModel(progress = mockTransferJobProgressModelFinished()))
                            add(mockTransferJobModel(progress = mockTransferJobProgressModelFinished()))
                            add(mockTransferJobModel(progress = mockTransferJobProgressModelFinished()))
                            add(mockTransferJobModel(progress = mockTransferJobProgressModelFinished()))
                            add(mockTransferJobModel(progress = mockTransferJobProgressModelFinished()))
                        }
                    }
                )
            )
        ),
        library = mockLibraryModel(
            localRoots = listOf(
                LibraryRootModel(
                    name = "Favorites",
                    path = "~/music/fav2025",
                    numFiles = 83u
                ),
                LibraryRootModel(
                    name = "Backlog",
                    path = "~/music/backlog",
                    numFiles = 427u
                ),
            ),
            transcoding = true,
        )
    )

    DesktopHome(
        model = model,
        showHints = false,
        onAcceptAndTrust = {},
        onAcceptOnce = {},
        onDeny = {},
        onAddLibraryRoot = { _: String, _: String -> },
        onRemoveLibraryRoot = {},
        onRescanLibrary = {},
    )
}