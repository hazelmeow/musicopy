package app.musicopy

expect class DirectoryPicker {
    constructor(platformContext: PlatformActivityContext)

    suspend fun pickDownloadDirectory()
}
