package app.musicopy

import uniffi.musicopy.CoreException
import uniffi.musicopy.pickFolder

actual class DirectoryPicker {
    actual constructor(platformContext: PlatformActivityContext)

    actual suspend fun pickDownloadDirectory() {
        try {
            val pickedPath = pickFolder()
            AppSettings.downloadDirectory = pickedPath
        } catch (e: CoreException) {
            // TODO: toast?
            println("Error: ${e}")
        }
    }
}
